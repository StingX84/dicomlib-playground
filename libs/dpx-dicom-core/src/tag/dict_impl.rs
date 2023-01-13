use super::*;
use crate::{utils::unescape::unescape_with_validator, Cow, Vr};

use std::io::BufRead;

// cSpell:ignore Deidentification Nonidentifying тест

#[cfg(test)]
mod tests;

/// This structure contains information about a specific DICOM [`Tag`]
///
/// See [`dpx_dicom_core::tag::Dictionary`]
#[derive(Debug, Clone)]
pub struct Meta<'a> {
    /// Tag key and it's private creator
    pub tag: Tag<'a>,
    /// TagKey mask.
    ///
    /// This number represents an AND mask applied to the attribute tag key
    /// when searching in a [`Dictionary`].
    ///
    /// The value of 0xFFFFFFFFu32 means attribute is searched exactly as-is.
    ///
    /// Mask may contain only one block of zero bits!
    pub mask: u32,
    /// Attribute Value Representation
    pub vr: Vr,
    /// Alternative Value Representation
    ///
    /// Most of attributes has a single VR and this member will
    /// be set to [`Vr::Undefined`]
    ///
    /// Note: This value deliberately not an [`Option`] for the performance considerations.
    pub alt_vr: Vr,
    /// Value Multiplicity constraint
    ///
    /// The first value is the minimum multiplicity, the second value is the maximum multiplicity.
    /// If the maximum multiplicity is open-ended, 0 is used. The third value, if present, is the "stride", i.e.,
    /// the increment between valid multiplicity values. A stride is used when values are added in sets, such as
    /// an x/y/z set of coordinate values that is recorded in triplets. The stride is not permitted to be 0.
    ///
    /// Examples:
    /// - VM of 1-3 is expressed as (1,3,1) meaning the multiplicity is permitted to be 1, 2 or 3
    /// - VM of 1-n is expressed as (1,0,1)
    /// - VM of 0-n is expressed as (0,0,1)
    /// - VM of 3-3n is expressed as (3,0,3)
    pub vm: (u8, u8, u8),
    /// Short display name of the attribute Tag
    ///
    /// For example: "Patient's Name"
    pub name: Cow<'a, str>,
    /// Alphanumeric keyword of this attribute
    ///
    /// For example: "Patient​Name"
    pub keyword: Cow<'a, str>,
    /// Section of the standard or a vendor name
    pub source: Source,
}

impl<'a> PartialEq for Meta<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.tag.eq(&other.tag)
    }
}

impl<'a, 'b> PartialEq<Tag<'b>> for Meta<'a> {
    fn eq(&self, other: &Tag<'b>) -> bool {
        self.tag.eq(other)
    }
}

impl<'a> PartialEq<TagKey> for Meta<'a> {
    fn eq(&self, other: &TagKey) -> bool {
        self.tag.key.eq(other)
    }
}

impl<'a> Eq for Meta<'a> {}

impl<'a> PartialOrd for Meta<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.tag.partial_cmp(&other.tag)
    }
}

impl<'a, 'b> PartialOrd<Tag<'b>> for Meta<'a> {
    fn partial_cmp(&self, other: &Tag<'b>) -> Option<std::cmp::Ordering> {
        self.tag.partial_cmp(other)
    }
}

impl<'a> PartialOrd<TagKey> for Meta<'a> {
    fn partial_cmp(&self, other: &TagKey) -> Option<std::cmp::Ordering> {
        self.tag.key.partial_cmp(other)
    }
}

impl<'a> Ord for Meta<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tag.cmp(&other.tag)
    }
}

/// Section of the Standard, that declares this attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Invalid attribute
    Invalid,
    /// Standard DICOM attribute
    Dicom,
    /// Digital Imaging and Communication for Security
    Dicos,
    /// Digital Imaging and Communication in Nondestructive Evaluation
    Diconde,
    /// Retired standard DICOM attribute
    Retired,
    /// Custom attribute from a vendor
    Vendored(PrivateIdentificationAction),
}

/// Private attribute de-identification action
///
/// This affects the action library takes on the attribute
/// when de-identifying the dataset. Also, this actions
/// may be conveyed in the dataset, so other dicom application
/// know what to do with private attributes when it decides to
/// de-identify the dataset.
///
/// This library may automatically construct and/or update
/// attribute "Private Data Element Characteristics Sequence (0008,0300)"
/// and this code affects attribute "Block Identifying Information Status (0008,0303)",
/// "Nonidentifying Private Elements (0008,0304)" and "Deidentification Action Sequence (0008,0305)":
///
/// If all of attributes in the single private group has [`PrivateIdentificationAction::None`]
/// type, then "Block Identifying Information Status (0008,0303)" will be set to "SAFE" and
/// no other de-identifying related attributes are written.
///
/// If some of attributes in the single private group has [`PrivateIdentificationAction::None`]
/// type, then "Block Identifying Information Status (0008,0303)" will be set to "MIXED",
/// then "Nonidentifying Private Elements (0008,0304)" and "Deidentification Action Sequence (0008,0305)"
/// attributes are written to reflect attribute de-identifying actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateIdentificationAction {
    /// Attribute does not contain identifying information
    None,
    /// Attribute contains identifying information and recommended action for the de-identifying entity:\
    /// replace with a non-zero length value that may be a dummy value and consistent with the VR
    D,
    /// Attribute contains identifying information and recommended action for the de-identifying entity:\
    /// replace with a zero length value, or a non-zero length value that may be a dummy value and consistent with the VR
    Z,
    /// Attribute contains identifying information and recommended action for the de-identifying entity:\
    /// remove
    X,
    /// Attribute contains identifying information and recommended action for the de-identifying entity:\
    /// replace with a non-zero length UID that is internally consistent within a set of Instance
    U,
}

#[derive(Clone)]
pub struct StaticMetaList(&'static [Meta<'static>]);
inventory::collect!(StaticMetaList);

// cSpell:ignore aabb
struct DictCache<'a> {
    // Contains Tag (element 0) and TagInfo (element 1) "flattened" from static and
    // dynamic dictionaries sorted by Tag.
    // Private Attributes Tag's are "flattened" in 4 different forms:
    // 1. original form as given
    // 2. zeroed `XX` in TagKey (gggg,eeXX)
    // 3. original TagKey and "zeroed" `creator` field in Tag
    // 4. 2 and 3 combined
    sorted: Vec<(Tag<'a>, &'a Meta<'a>)>,

    // Contains TagKey bitwise and'ed with mask (element 0), mask from TagInfo (element 1)
    // and TagInfo "flattened" from static and dynamic dictionaries.
    // This list contains only elements with a mask other than 0xFFFFFFFF
    masked: Vec<(u32, u32, &'a Meta<'a>)>,
}

#[derive(Default)]
pub struct Dictionary<'a> {
    statics: Vec<&'static StaticMetaList>,
    dynamic: Vec<Meta<'a>>,
    cache: Option<DictCache<'a>>,
}

impl<'a> Dictionary<'a> {
    // This function requires unstable, because of function "is_sorted_by"
    // Function is enabled only in "debug" builds. Other function variant
    // for "Release" builds is no-op.
    #[cfg(all(feature = "unstable", debug_assertions))]
    fn verify_sorted(dict: &'static StaticMetaList) {
        assert!(
            dict.0.is_sorted_by_key(|i| &i.tag),
            "array in dpx_dicom_core::tag::StaticDictionary should be sorted by TagKey!"
        );
    }

    #[cfg(not(all(feature = "unstable", debug_assertions)))]
    const fn verify_sorted(_: &'static StaticMetaList) {}

    pub fn new() -> Self {
        let statics: Vec<&'static StaticMetaList> =
            inventory::iter::<StaticMetaList>.into_iter().collect();
        for dict in statics.iter() {
            Self::verify_sorted(dict);
        }

        Self {
            statics,
            dynamic: Vec::new(),
            cache: None,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            statics: Vec::new(),
            dynamic: Vec::new(),
            cache: None,
        }
    }

    pub fn add_static_list(&mut self, dict: &'static StaticMetaList) {
        if !self
            .statics
            .iter()
            .any(|e| ::core::ptr::eq((*e) as *const _, dict as *const _))
        {
            Self::verify_sorted(dict);
            self.statics.push(dict);
            self.cache = None;
        }
    }

    pub fn add_dynamic_list<'b: 'a, T: Iterator<Item = Meta<'b>>>(&mut self, iter: T) {
        self.dynamic.reserve(iter.size_hint().1.unwrap_or(0));
        for v in iter {
            self.dynamic.push(v);
        }
        self.cache = None;
    }

    pub fn add_from_memory(&mut self, buf: &mut impl std::io::Read) -> Result<()> {
        let reader = std::io::BufReader::new(buf);
        let mut dict = Vec::<Meta<'static>>::new();

        for (line_number, line) in reader.buffer().lines().enumerate() {
            let line = line.context(DictFileReadFailedSnafu)?;
            if let Some(tag_info) = Self::dict_parse_line(line_number, line.trim())? {
                dict.push(tag_info);
            }
        }
        self.add_dynamic_list(dict.into_iter());
        Ok(())
    }

    pub fn add_from_file(&mut self, file_name: impl AsRef<Path>) -> Result<()> {
        use std::fs::File;
        let mut file = File::open(file_name.as_ref()).context(DictFileOpenFailedSnafu {
            file_name: file_name.as_ref().to_path_buf(),
        })?;
        self.add_from_memory(&mut file)
    }

    pub fn rebuild_cache(&'a mut self) {
        let mut cache = DictCache::<'a> {
            sorted: Vec::new(),
            masked: Vec::new(),
        };

        let guessed_total_count =
            self.statics.iter().fold(0, |acc, dict| acc + dict.0.len()) + self.dynamic.len();
        cache.sorted.reserve(guessed_total_count);

        let guessed_masked_count = self
            .statics
            .iter()
            .map(|dict| dict.0.iter().filter(|v| v.mask != 0xFFFFFFFFu32).count())
            .sum::<usize>()
            + self
                .dynamic
                .iter()
                .filter(|v| v.mask != 0xFFFFFFFFu32)
                .count();
        cache.masked.reserve(guessed_masked_count);

        // Reverse order, because after stable sort and dedup, we want
        // to prioritize dynamically added attributes over statically
        // added attributes. Each list of attributes also prioritizes
        // lastly added.
        for tag_info in self.dynamic.iter().rev() {
            Self::add_cached_tag(&mut cache, tag_info);
        }

        for dict in self.statics.iter().rev() {
            for tag_info in dict.0.iter() {
                Self::add_cached_tag(&mut cache, tag_info)
            }
        }

        cache.sorted.sort_by(|l, r| l.0.cmp(&r.0));
        cache.sorted.dedup_by(|l, r| l.0 == r.0);
        cache.masked.sort_by(|l, r| l.1.cmp(&r.1).reverse());
        cache.masked.dedup_by(|l, r| l.0 == r.0 && l.1 == r.1);

        self.cache = Some(cache);
    }

    pub fn get_by_tag_key(&self, key: TagKey) -> Option<&'a Meta> {
        // Search in the cache if it is available
        if let Some(c) = &self.cache {
            // first, search directly in a sorted "flattened" array
            if let Ok(index) = c.sorted.binary_search_by(|v| v.0.key.0.cmp(&key.0)) {
                // Safety: we've got this VALID index from the vector method and there is no way
                // to mutate vector content after the search.
                return Some(unsafe { c.sorted.get_unchecked(index).1 });
            }

            // Then, search in an UNSORTED array containing only tags with mask
            // This array expected to be small enough to do a O(N) search.
            return c.masked.iter().find(|v| v.0 == key.0 & v.1).map(|v| v.2);
        }

        // Search the hard-way.
        let tag = Tag::new(key, None);
        if let Some(v) = Self::search_in_ary(self.dynamic.iter(), &tag) {
            return Some(v);
        }
        for ary in self.statics.iter().rev() {
            if let Some(v) = Self::search_in_ary(ary.0.iter(), &tag) {
                return Some(v);
            }
        }

        None
    }

    pub fn get_by_tag(&self, tag: &Tag) -> Option<&'a Meta> {
        // Search in the cache if it is available
        if let Some(c) = &self.cache {
            // first, search directly in a sorted "flattened" array
            if let Ok(index) = c.sorted.binary_search_by(|v| v.0.cmp(tag)) {
                // Safety: we've got this VALID index from the vector method and there is no way
                // to mutate vector content after the search.
                return Some(unsafe { c.sorted.get_unchecked(index).1 });
            }

            // Then, search in an UNSORTED array containing only tags with mask
            // This array expected to be small enough to do a O(N) search.
            return c
                .masked
                .iter()
                .find(|v| v.0 == tag.key.0 & v.1)
                .map(|v| v.2);
        }

        // Search the hard-way.
        if let Some(v) = Self::search_in_ary(self.dynamic.iter(), tag) {
            return Some(v);
        }
        for ary in self.statics.iter().rev() {
            if let Some(v) = Self::search_in_ary(ary.0.iter(), tag) {
                return Some(v);
            }
        }

        None
    }
}

#[derive(Debug)]
struct DictParseErr(usize, String);

macro_rules! mk_parse_err {
    ($str:expr, $index:expr, $msg:expr) => {
        DictParseErr($str.as_ptr() as usize + $index, ($msg).to_owned())
    };
}
macro_rules! parse_fail {
    ($str:expr, $index:expr, $msg:expr) => {{
        return Err(mk_parse_err!($str, $index, $msg));
    }};
}
macro_rules! parse_ensure {
    ($e:expr, $str:expr, $index:expr, $msg:expr) => {{
        if !($e) {
            parse_fail!($str, $index, $msg);
        }
    }};
}

// Private implementations
impl<'a> Dictionary<'a> {
    fn dict_parse_line(line_number: usize, line: &str) -> Result<Option<Meta<'static>>> {
        Self::dict_parse_line_int(line).map_err(|e| {
            let char_pos = Self::map_offset_to_char_pos(line, e.0);
            Error::DictParseFailed {
                line_number,
                char_pos,
                msg: e.1,
            }
        })
    }

    fn map_offset_to_char_pos(line: &str, offset: usize) -> usize {
        let byte_pos = offset.clamp(line.as_ptr() as usize, line.as_ptr() as usize + line.len())
            - line.as_ptr() as usize;
        let mut bytes_counted = 0usize;
        let mut char_pos = 0usize;
        for c in line.chars() {
            bytes_counted += c.len_utf8();
            if byte_pos < bytes_counted {
                break;
            }
            char_pos += 1;
        }

        char_pos
    }

    fn dict_parse_line_int(line: &str) -> Result<Option<Meta<'static>>, DictParseErr> {
        let line = line.trim_start();
        if line.starts_with('#') || line.is_empty() {
            return Ok(None);
        }

        let (field_text, line_left) = Self::dict_next_field(line)?;
        let (tag, mask) = Self::dict_parse_field_tag(field_text)?;

        let (field_text, line_left) = Self::dict_next_field(line_left)?;
        let name = Self::dict_parse_field_name(field_text)?;

        let (field_text, line_left) = Self::dict_next_field(line_left)?;
        let keyword = Self::dict_parse_field_keyword(field_text)?;

        let (field_text, line_left) = Self::dict_next_field(line_left)?;
        let (vr, alt_vr) = Self::dict_parse_field_vr(field_text)?;

        let (field_text, line_left) = Self::dict_next_field(line_left)?;
        let vm = Self::dict_parse_field_vm(field_text)?;

        let source = Self::dict_parse_field_source(line_left)?;

        Ok(Some(Meta::<'static> {
            tag,
            mask,
            vr,
            alt_vr,
            vm,
            name,
            keyword,
            source,
        }))
    }

    fn dict_next_field(s: &str) -> Result<(&str, &str), DictParseErr> {
        match s.find('\t') {
            None => parse_fail!(
                s,
                s.len().saturating_sub(1),
                "unexpected end of line, expecting TAB character"
            ),
            Some(index) => Ok((&s[0..index], &s[index + 1..])),
        }
    }

    fn dict_parse_tag_component(s: &str) -> Result<(u16, u16), DictParseErr> {
        let s = s.trim();
        let mut mask = 0u16;
        let mut number = 0u16;
        let mut it = s.chars();
        let mut byte_offset = 0usize;
        for n in 0usize..4 {
            let c = it.next().ok_or_else(|| {
                mk_parse_err!(s, byte_offset, "expecting hexadecimal number or \"x\"")
            })?;
            byte_offset += c.len_utf8();
            if let Some(num) = c.to_digit(16) {
                number |= (num as u16) << (4 * (3 - n));
            } else if c == 'x' || c == 'X' {
                mask |= 0xF << (4 * (3 - n));
            } else {
                parse_fail!(
                    s,
                    n,
                    format!(
                        "invalid character \"{}\" in Tag component, expected hexadecimal or 'X'",
                        c.escape_default()
                    )
                );
            }
        }
        parse_ensure!(
            it.next().is_none(),
            s,
            byte_offset,
            "extra characters after Tag element"
        );
        Ok((number, !mask))
    }

    fn dict_parse_field_tag(s: &str) -> Result<(Tag<'static>, u32), DictParseErr> {
        let s = s.trim();
        parse_ensure!(
            !s.is_empty() && s.starts_with('('),
            s,
            0,
            "expecting opening brace at Tag definition start"
        );
        parse_ensure!(
            s.ends_with(')'),
            s,
            s.len() - 1,
            "expecting closing brace at Tag definition end"
        );

        let mut components = s[1..s.len() - 1].splitn(3, ',');

        let group_chars = components
            .next()
            .expect("Bug: `split` returned zero elements");
        let element_chars = components.next().ok_or_else(|| {
            mk_parse_err!(
                group_chars,
                group_chars.len().saturating_sub(1),
                "expecting comma separated Tag group number"
            )
        })?;
        let (group, group_mask) = Self::dict_parse_tag_component(group_chars)?;
        let (element, element_mask) = Self::dict_parse_tag_component(element_chars)?;

        let creator: Option<Cow<'static, str>> = match components.next() {
            None => None,
            Some(creator) => {
                let creator = creator.trim();
                parse_ensure!(
                    creator.len() >= 2 && creator.starts_with('"') && creator.ends_with('"'),
                    creator,
                    0,
                    "no starting or ending double quote for private creator in Tag definition"
                );
                let creator = &creator[1..creator.len() - 1];
                parse_ensure!(
                    !creator.is_empty(),
                    creator,
                    0,
                    "empty private creator in Tag definition"
                );

                use crate::utils::unescape::Error::*;
                let unescaped = unescape_with_validator(creator, |c| !c.is_control() && c != '\\').map_err(|e| match e {
                    IncompleteStr { pos } => mk_parse_err!(
                        creator,
                        pos,
                        "incomplete escape sequence in private creator"
                    ),
                    InvalidChar { char, pos } => mk_parse_err!(
                        creator,
                        pos,
                        format!(
                            "invalid escape sequence character \"{}\" in private creator",
                            char.to_owned().escape_default()
                        )
                    ),
                    ParseInt { source, pos } => mk_parse_err!(
                        creator,
                        pos,
                        format!("unable to parse escape sequence({source}) in private creator")
                    ),
                    NotAllowedChar { char, pos } => mk_parse_err!(
                        creator,
                        pos,
                        format!("invalid character \"{char}\" in private creator. backslash and control characters are not allowed")
                    )
                })?;
                let chars_count = unescaped.chars().count();
                parse_ensure!(
                    chars_count <= 64,
                    creator,
                    0,
                    format!(
                        "private creator is too long ({chars_count} chars). maximum 64 allowed."
                    )
                );
                Some(Cow::Owned(unescaped))
            }
        };

        Ok((
            Tag::new(TagKey::new(group, element), creator),
            ((group_mask as u32) << 16) | element_mask as u32,
        ))
    }

    fn dict_parse_field_name(s: &'_ str) -> Result<Cow<'static, str>, DictParseErr> {
        let s = s.trim();
        parse_ensure!(!s.is_empty(), s, 0, "unexpected empty Name field");
        parse_ensure!(
            s.len() <= 128,
            s,
            0,
            format!(
                "Name field is too long ({} bytes) maximum allowed 128 bytes",
                s.len()
            )
        );
        if let Some(index) = s.bytes().position(|c| !c.is_ascii_graphic() && c != b' ') {
            parse_fail!(
                s,
                index,
                format!(
                    "invalid character \"{}\" in Name field. only space and ascii graphic allowed",
                    s.as_bytes()[index].to_owned().escape_ascii()
                )
            );
        }
        Ok(Cow::Owned(s.to_owned()))
    }

    fn dict_parse_field_keyword(s: &'_ str) -> Result<Cow<'static, str>, DictParseErr> {
        let s = s.trim();
        parse_ensure!(!s.is_empty(), s, 0, "unexpected empty Keyword field");
        parse_ensure!(
            s.len() <= 64,
            s,
            0,
            format!(
                "Keyword field is too long ({} bytes) maximum allowed 64 bytes",
                s.len()
            )
        );
        let c = s.as_bytes()[0];
        parse_ensure!(
            c.is_ascii_alphabetic(),
            s,
            0,
            format!(
                "first character \"{}\" is not alphabetic in Keyword field",
                c.to_owned().escape_ascii()
            )
        );
        if let Some(index) = s
            .bytes()
            .position(|c| !c.is_ascii_alphanumeric() && c != b'_')
        {
            parse_fail!(
                s,
                index,
                format!(
                    "invalid character \"{}\" in Name field. only underscore and alphabetic allowed",
                    s.as_bytes()[index].to_owned().escape_ascii()
                )
            );
        }
        Ok(Cow::Owned(s.to_owned()))
    }

    fn dict_parse_field_vr(s: &'_ str) -> Result<(Vr, Vr), DictParseErr> {
        if let Some(i) = s.find(" or ") {
            let vr_text = s[0..i].trim();
            //parse_ensure!(i + 4 < s.len(), s, 0, "unexpected empty second VR");
            let alt_vr_text = s[i + 4..].trim();
            let vr = Vr::try_from(vr_text).map_err(|_| {
                mk_parse_err!(
                    vr_text,
                    0,
                    format!("unsupported VR \"{}\"", vr_text.escape_default())
                )
            })?;
            let alt_vr = Vr::try_from(alt_vr_text).map_err(|_| {
                mk_parse_err!(
                    alt_vr_text,
                    0,
                    format!("unsupported VR \"{}\"", alt_vr_text.escape_default())
                )
            })?;
            Ok((vr, alt_vr))
        } else {
            let s = s.trim();
            let vr = Vr::try_from(s).map_err(|_| {
                mk_parse_err!(s, 0, format!("unsupported VR \"{}\"", s.escape_default()))
            })?;
            Ok((vr, Vr::Undefined))
        }
    }

    fn dict_parse_field_vm(s: &'_ str) -> Result<(u8, u8, u8), DictParseErr> {
        let mut s = s.trim();
        let vm_is_unbounded = s.ends_with(['n', 'N']);
        if vm_is_unbounded {
            s = &s[..s.len() - 1];
        }
        let mut vm1_text = s;
        let mut vm2_text = s;
        let (vm1, vm2) = if let Some(index) = s.find('-') {
            vm1_text = s[..index].trim();
            vm2_text = s[index + 1..].trim();
            let vm1 = vm1_text
                .parse::<u8>()
                .map_err(|e| mk_parse_err!(vm1_text, 0, format!("invalid VM number({e})")))?;
            parse_ensure!(
                vm_is_unbounded || !vm2_text.is_empty(),
                vm2_text,
                0,
                "unexpected end of VM. expecting number after \"-\""
            );
            let vm2 = if !vm2_text.is_empty() {
                let vm = vm2_text
                    .parse::<u8>()
                    .map_err(|e| mk_parse_err!(vm2_text, 0, format!("invalid VM number({e})")))?;
                Some(vm)
            } else {
                None
            };
            (vm1, vm2)
        } else {
            parse_ensure!(
                !vm_is_unbounded,
                s,
                s.len(),
                "unexpected \"n\". allowed \"-\" or end of VM"
            );
            let vm1 = vm1_text
                .parse::<u8>()
                .map_err(|e| mk_parse_err!(vm1_text, 0, format!("invalid VM number({e})")))?;
            (vm1, None)
        };

        if vm_is_unbounded {
            if let Some(vm2) = vm2 {
                // Form `A-Bn`
                parse_ensure!(
                    vm1 == vm2,
                    vm1_text,
                    0,
                    format!("unequal numbers ({vm1} and {vm2}) in unbound repetitive VM")
                );
                Ok((vm1, 0, vm1))
            } else {
                // Form `A-n`
                Ok((vm1, 0, 1))
            }
        } else if let Some(vm2) = vm2 {
            // Form `A-B`
            parse_ensure!(
                vm2 > 0,
                vm2_text,
                0,
                format!("zero second VM number. expected number in range 1-255")
            );
            parse_ensure!(
                vm1 <= vm2,
                vm1_text,
                0,
                format!("second VM number({vm2}) is less than first({vm1})")
            );
            Ok((vm1, vm2, 1))
        } else {
            // Form `A`
            parse_ensure!(
                vm1 > 0,
                vm1_text,
                0,
                format!("zero first VM number. expected number in range 1-255")
            );
            Ok((vm1, vm1, 1))
        }
    }

    fn dict_parse_field_source(s: &'_ str) -> Result<Source, DictParseErr> {
        if s.eq_ignore_ascii_case("dicom") {
            Ok(Source::Dicom)
        } else if s.eq_ignore_ascii_case("diconde") {
            Ok(Source::Diconde)
        } else if s.eq_ignore_ascii_case("dicos") {
            Ok(Source::Dicos)
        } else if s.eq_ignore_ascii_case("ret") {
            Ok(Source::Retired)
        } else if s.eq_ignore_ascii_case("priv") {
            Ok(Source::Vendored(PrivateIdentificationAction::None))
        } else if s.eq_ignore_ascii_case("priv(d)") {
            Ok(Source::Vendored(PrivateIdentificationAction::D))
        } else if s.eq_ignore_ascii_case("priv(z)") {
            Ok(Source::Vendored(PrivateIdentificationAction::Z))
        } else if s.eq_ignore_ascii_case("priv(x)") {
            Ok(Source::Vendored(PrivateIdentificationAction::X))
        } else if s.eq_ignore_ascii_case("priv(u)") {
            Ok(Source::Vendored(PrivateIdentificationAction::U))
        } else {
            Err(mk_parse_err!(s, 0, "unrecognized Source field"))
        }
    }

    fn add_cached_tag(cache: &'_ mut DictCache<'a>, tag_info: &'a Meta<'a>) {
        let key = &tag_info.tag.key;

        // Add tag as-is
        cache.sorted.push((tag_info.tag.clone(), tag_info));

        // Add "masked" tag in a special array
        if tag_info.mask != 0xFFFFFFFFu32 {
            debug_assert_eq!(key.as_u32() & tag_info.mask, key.as_u32(),
                "TagInfo for tag {} in dpx_dicom_core::tag::StaticDictionary must be pre-multiplied by it's mask {:08x}",
                tag_info.tag, tag_info.mask);
            cache.masked.push((key.as_u32(), tag_info.mask, tag_info));
        }

        // Note: if you did not provide 'private creator' for the private attribute,
        // then it will be matched "exactly". This is a "compatibility" feature with
        // some buggy software, that does not provide any "private creator" for their
        // attributes.
        if key.is_private() && tag_info.tag.creator.is_some() {
            // Add private attribute without "private creator"
            if key.is_private_attribute() {
                cache.sorted.push((
                    Tag {
                        creator: None,
                        ..tag_info.tag
                    },
                    tag_info,
                ));
            }

            // Add normalized attribute
            if let Some(normalized_key) = key.to_canonical_if_private() {
                cache.sorted.push((
                    Tag {
                        key: normalized_key,
                        ..tag_info.tag.clone()
                    },
                    tag_info,
                ));
            }
        }
    }

    fn search_in_ary<T: Iterator<Item = &'a Meta<'a>>>(
        iter: T,
        tag: &'_ Tag<'_>,
    ) -> Option<&'a Meta<'a>> {
        let mut matched = None;

        if !tag.key.is_private() || tag.creator.is_none() {
            // Search for a regular tag OR private without known "creator".
            // We can't coerce private attributes and reservations to it's canonical form,
            // because without known "creator" we will collide with someone other private attribute.
            for v in iter {
                // Condition 1: exact match
                if v.tag == *tag {
                    matched = Some(v);
                    break;
                }

                // Condition 2: is searched tag in the masked range
                if v.mask != 0xFFFFFFFFu32 && v.tag.key.0 & v.mask == tag.key.0 & v.mask {
                    matched = Some(v);
                    break;
                }
            }
        } else if let Some(canonical_key) =
            tag.key.to_canonical_if_private().filter(|v| *v != tag.key)
        {
            const WEIGHT_MATCH_KEY: u8 = 4;
            const WEIGHT_MATCH_MASK: u8 = 3;
            const WEIGHT_MATCH_CANONICAL_KEY: u8 = 2;
            const WEIGHT_MATCH_CANONICAL_MASK: u8 = 1;
            let mut matched_weight = 0u8;

            // This is a private reservation or attribute in non-canonical form with a known searched "creator".
            for v in iter {
                // Condition 1: searched Tag has exact match
                if v.tag == *tag {
                    matched = Some(v);
                    break;
                }

                // Mask is non zero and contains high 16-bits of the original mask IF
                // it contains non-zero bits in high 16-bits and has no masked out bits in lower 16-bits
                let mask = {
                    if v.mask != 0xFFFFFFFFu32 && v.mask & 0x0000FFFFu32 == 0x0000FFFFu32 {
                        v.mask & 0xFFFF0000u32
                    } else {
                        0u32
                    }
                };

                // Condition 2: "creator" matches and TagKey from searched Tag is in masked range
                if mask != 0u32
                    && v.tag.key.0 & mask == tag.key.0 & mask
                    && v.tag.creator == tag.creator
                {
                    matched = Some(v);
                    break;
                }

                // Condition 3: "creator" matches and canonical TagKey from searched Tag matches.
                if v.tag.key == canonical_key && v.tag.creator == tag.creator {
                    return Some(v);
                }

                // Condition 4: "creator" matches and canonical TagKey from searched Tag in masked range.
                if mask != 0u32
                    && v.tag.key.0 & mask == canonical_key.0 & mask
                    && v.tag.creator == tag.creator
                {
                    matched = Some(v);
                    break;
                }

                // Other matches are the "best guess" if our dictionary has no "creator"
                if matched_weight < WEIGHT_MATCH_KEY && v.tag.creator.is_none() {
                    // Condition 5: dict has no "creator", but TagKey exactly matched
                    if v.tag.key == tag.key {
                        matched_weight = WEIGHT_MATCH_KEY;
                        matched = Some(v);
                        continue;
                    }

                    if matched_weight < WEIGHT_MATCH_MASK {
                        // Condition 6: dict has no "creator", bur TagKey falls into the masked range
                        if mask != 0u32 && v.tag.key.0 & mask == tag.key.0 & mask {
                            matched_weight = WEIGHT_MATCH_MASK;
                            matched = Some(v);
                            continue;
                        }

                        if matched_weight < WEIGHT_MATCH_CANONICAL_KEY {
                            // Condition 7: dict has no "creator", but canonical TagKey matches
                            if v.tag.key == canonical_key {
                                matched_weight = WEIGHT_MATCH_CANONICAL_MASK;
                                matched = Some(v);
                                continue;
                            }

                            // Condition 8: dict has no "creator"
                            if matched_weight == 0
                                && mask != 0u32
                                && v.tag.key.0 & mask == canonical_key.0 & mask
                            {
                                matched_weight = WEIGHT_MATCH_CANONICAL_MASK;
                                matched = Some(v);
                                continue;
                            }
                        }
                    }
                }
            }
        } else {
            const WEIGHT_MATCH_KEY: u8 = 2;
            const WEIGHT_MATCH_MASK: u8 = 1;
            let mut matched_weight = 0u8;

            // This is a private reservation or attribute in non-canonical form with a known searched "creator".
            for v in iter {
                // Condition 1: searched Tag has exact match
                if v.tag == *tag {
                    matched = Some(v);
                    break;
                }

                // Mask is non zero and contains high 16-bits of the original mask IF
                // it contains non-zero bits in high 16-bits and has no masked out bits in lower 16-bits
                let mask = {
                    if v.mask != 0xFFFFFFFFu32 && v.mask & 0x0000FFFFu32 == 0x0000FFFFu32 {
                        v.mask & 0xFFFF0000u32
                    } else {
                        0u32
                    }
                };

                // Condition 2: "creator" matches and TagKey from searched Tag is in masked range
                if mask != 0u32
                    && v.tag.key.0 & mask == tag.key.0 & mask
                    && v.tag.creator == tag.creator
                {
                    matched = Some(v);
                    break;
                }

                // Other matches are the "best guess" if our dictionary has no "creator"
                if matched_weight < WEIGHT_MATCH_KEY && v.tag.creator.is_none() {
                    // Condition 5: dict has no "creator", but TagKey exactly matched
                    if v.tag.key == tag.key {
                        matched_weight = WEIGHT_MATCH_KEY;
                        matched = Some(v);
                        continue;
                    }

                    // Condition 6: dict has no "creator", bur TagKey falls into the masked range
                    if matched_weight < WEIGHT_MATCH_MASK
                        && mask != 0u32
                        && v.tag.key.0 & mask == tag.key.0 & mask
                    {
                        matched_weight = WEIGHT_MATCH_MASK;
                        matched = Some(v);
                        continue;
                    }
                }
            }
        }

        matched
    }
}
