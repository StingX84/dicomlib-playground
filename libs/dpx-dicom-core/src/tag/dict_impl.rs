use super::*;
use crate::{utils::unescape::unescape_with_validator, Cow, Vr};
use std::{cmp::Ordering, fmt::Debug, io::{self, BufRead}, ptr::NonNull};

#[cfg(test)]
mod tests;

// cSpell:ignore strtok aabb

/// Structure managing a collection of attributes and their metadata.
///
/// This structure is rarely accessed by the application. All the functions
/// provided by this struct are mirrored in higher-level abstractions.
///
/// The structure itself contains a vector of statically allocated tag
/// description ([`Meta`]) lists and dynamically registered individual tag
/// descriptions.
///
/// Search in the directory is carried out by calling methods
/// [search_by_tag](Self::search_by_tag) or
/// [search_by_key](Self::search_by_key). To speed up the search process,
/// Dictionary supports a "cache". It is invalidated every time content mutated
/// and may be rebuild with [rebuild_cache](Self::rebuild_cache).
///
/// Dictionary may be extended with static lists using
/// [add_static_list](Self::add_static_list) or with dynamic lists with
/// [add_dynamic_list](Self::add_dynamic_list),
/// [add_from_memory](Self::add_from_memory) or
/// [add_from_file](Self::add_from_file).
///
/// Dictionary lookup by higher-level abstractions forwarded through
/// [`State`](crate::State)
///
/// This class supports "automatic" registration of static descriptions list
/// using crate [`inventory`]. Built-in standard DICOM attributes are already
/// registered automatically.
///
/// Example of application custom dictionary loaded automatically:
/// ```
/// // Declare your attributes and register them in the Dictionary
/// mod app {
///     use dpx_dicom_core::declare_tags;
///     use inventory::submit;
///     declare_tags!{
///         pub const ALL_APP_TAGS = [
///             PatientSpacecraftLicense: { (0x4321, 0x1000, "CoolApp Group1"), LO, 1-n, "Patient's Spacecraft License", Vendored(X) },
///             IssuerOfPatientSpacecraftLicense: { (0x4321, 0x1001, "CoolApp Group1"), LO, 1-n, "Issuer of Patient's Spacecraft License", Vendored(None) },
///             DoctorCryingReason: { (0x4323, 0x10BB, "CoolApp Group2"), UT, 1, "Doctor Crying Reason", Vendored(None) },
///         ];
///     }
///     submit!(ALL_APP_TAGS);
/// }
///
/// // Use your attributes anywhere in the application
/// use dpx_dicom_core::tag::Dictionary;
/// # #[cfg(not(miri))]
/// # fn main() {
/// let dict = Dictionary::new();
/// assert_eq!(
///     dict.search_by_tag(&app::DoctorCryingReason).unwrap().name,
///     "Doctor Crying Reason"
/// );
/// # }
/// # #[cfg(miri)]
/// # fn main() {}
/// ```
///
/// Note: automatic registration is currently unsupported under
/// [Miri](https://github.com/rust-lang/miri).
#[derive(Default)]
pub struct Dictionary {
    /// Vector of statically defined attribute lists added with
    /// [add_static_list](Self::add_static_list) or gathered in [new](Self::new)
    /// with [`inventory`]
    statics: Vec<&'static StaticMetaList>,

    /// Vector of dynamically added attributes with
    /// [add_dynamic_list](Self::add_dynamic_list),
    /// [add_from_memory](Self::add_from_memory) or
    /// [add_from_file](Self::add_from_file)
    dynamic: Vec<Meta>,

    /// Contains TagKey (element 0), private creator (element 1) and TagInfo
    /// (element 2) "flattened" from static and dynamic dictionaries sorted by
    /// Tag.
    ///
    /// Private Attributes Tag's are "flattened" in following forms:
    /// 1. original form as given
    /// 2. normalized form IF private creator is `Some`
    cache: Option<DictCache>,
}

// SAFETY:
// The reason this struct is not `Send` nor `Sync` by auto traits
// is presence of "raw" pointers. Struct guarantees, that
// no external shared state involved in those pointers.
unsafe impl Send for Dictionary {}
unsafe impl Sync for Dictionary {}

/// This structure contains information about a specific DICOM [`Tag`]
///
/// See [`Dictionary`]
#[derive(Debug, Clone)]
pub struct Meta {
    /// Tag key and it's private creator
    pub tag: Tag<'static>,
    /// TagKey mask.
    ///
    /// This number represents an AND mask applied to the attribute tag key
    /// when searching in a [`Dictionary`].
    ///
    /// The value of `0xFFFFFFFF` means attribute is searched exactly as-is.
    ///
    /// Mask may contain only one block of zero bits!
    pub mask: u32,
    /// Attribute Value Representations for this tag
    ///
    /// This tuple holds up to three alternative value representations
    /// for the Tag. Most of attributes has single VR and rest are set
    /// to [Vr::Undefined].
    ///
    /// Note: This value deliberately not an [`Option`] for the performance considerations.
    pub vr: (Vr, Vr, Vr),
    /// Value Multiplicity constraint
    ///
    /// The first value is the minimum multiplicity, the second value is the
    /// maximum multiplicity. If the maximum multiplicity is open-ended, 0 is
    /// used. The third value, if present, is the "stride", i.e., the increment
    /// between valid multiplicity values. A stride is used when values are
    /// added in sets, such as an x/y/z set of coordinate values that is
    /// recorded in triplets. The stride is not permitted to be 0.
    ///
    /// This definition exactly follows definition of the standard attribute
    /// "Private Data Element Value Multiplicity (0008,0309)". See [PS3.3
    /// Section C.12.1.1.7.1]
    ///
    /// Examples:
    /// - VM of 1-3 is expressed as (1,3,1) meaning the multiplicity is
    ///   permitted to be 1, 2 or 3
    /// - VM of 1-n is expressed as (1,0,1)
    /// - VM of 0-n is expressed as (0,0,1)
    /// - VM of 3-3n is expressed as (3,0,3)
    ///
    /// [PS3.3 Section C.12.1.1.7.1]:
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.12.html#sect_C.12.1.1.7.1
    ///     "C.12.1.1.7.1 Private Data Element Value Multiplicity"
    pub vm: (u8, u8, u8),
    /// Short display name of the attribute Tag
    ///
    /// For example: "Patient's Name"
    pub name: Cow<'static, str>,
    /// Alphanumeric keyword of this attribute
    ///
    /// For example: "Patient​Name"
    pub keyword: Cow<'static, str>,
    /// Section of the standard or a vendor name
    pub source: Source,
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
/// This affects the action library takes on the attribute when de-identifying
/// the dataset. Also, this actions may be conveyed in the dataset, so other
/// dicom application know what to do with private attributes when it decides to
/// de-identify the dataset.
///
/// This library may automatically construct and/or update attribute "Private
/// Data Element Characteristics Sequence (0008,0300)" and this code affects
/// attribute "Block Identifying Information Status (0008,0303)",
/// "Nonidentifying Private Elements (0008,0304)" and "Deidentification Action
/// Sequence (0008,0305)":
///
/// If all of attributes in the single private group has
/// [`PrivateIdentificationAction::None`] type, then "Block Identifying
/// Information Status (0008,0303)" will be set to "SAFE" and no other
/// de-identifying related attributes are written.
///
/// If some of attributes in the single private group has
/// [`PrivateIdentificationAction::None`] type, then "Block Identifying
/// Information Status (0008,0303)" will be set to "MIXED", then "Nonidentifying
/// Private Elements (0008,0304)" and "Deidentification Action Sequence
/// (0008,0305)" attributes are written to reflect attribute de-identifying
/// actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateIdentificationAction {
    /// Attribute does not contain identifying information
    None,
    /// Attribute contains identifying information and recommended action for
    /// the de-identifying entity:\
    /// replace with a non-zero length value that may be a dummy value and
    /// consistent with the VR
    D,
    /// Attribute contains identifying information and recommended action for
    /// the de-identifying entity:\
    /// replace with a zero length value, or a non-zero length value that may be
    /// a dummy value and consistent with the VR
    Z,
    /// Attribute contains identifying information and recommended action for
    /// the de-identifying entity:\
    /// remove
    X,
    /// Attribute contains identifying information and recommended action for
    /// the de-identifying entity:\
    /// replace with a non-zero length UID that is internally consistent within
    /// a set of Instance
    U,
}

/// Structure holding a reference to a static list of tag descriptions (list of [`Meta`]'s)
#[derive(Clone, Copy)]
pub struct StaticMetaList(pub(crate) &'static [Meta]);
inventory::collect!(StaticMetaList);

/// Shorthand for private creator stored in [`Tag`]
type Creator<'a> = Option<Cow<'a, str>>;

/// Entry in the dictionary cache [DictCache]
type DictCacheEntry = (TagKey, NonNull<Creator<'static>>, NonNull<Meta>);

/// Shorthand for vector or "flattened" static and dynamic dictionaries
struct DictCache {
    vec: Vec<DictCacheEntry>,
}

/// metrics from [Dictionary::metrics()]
pub struct DictMetrics {
    pub static_lists: usize,
    pub static_tags: usize,
    pub dynamic_tags: usize,
    pub cached_tags: Option<usize>,
}

/// Internal to dicom dictionary parser error reporting structure.
///
/// It contains a byte offset of the character causing the problem
/// in a line and an problem description.
#[cfg_attr(test, derive(Debug))]
struct DictParseErr(usize, String);

// ---------------------------------------------------------------------------
// Dictionary public interface methods implementation
// ---------------------------------------------------------------------------
impl Dictionary {
    /// Constructs the class gathering all the [`StaticMetaList`] objects
    /// registered with [`inventory::submit!`].
    ///
    /// Note: See struct-level [documentation](Self) for examples.
    ///
    /// Note: The created dictionary has no "cache". To speed up searches,
    /// you should call [rebuild_cache](Self::rebuild_cache) after any
    /// mutating functions.
    pub fn new() -> Self {
        Self {
            statics: inventory::iter::<StaticMetaList>.into_iter().collect(),
            ..Default::default()
        }
    }

    /// Constructs the empty class without any statically registered lists.
    pub fn new_empty() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Adds custom constant list of ['Meta'] objects.
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    pub fn add_static_list(&mut self, dict: &'static StaticMetaList) {
        // Note: We do not check equality of dictionary content, only check if
        // they are at the same memory address.
        if !self
            .statics
            .iter()
            .any(|e| ::core::ptr::eq((*e) as *const _, dict as *const _))
        {
            // Invalidate cache early to satisfy SAFETY invariants including
            // safety on "panic" unwind
            self.cache = None;
            self.statics.push(dict);
        }
    }

    /// Adds custom constant list of ['Meta'] objects.
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    pub fn add_dynamic_list<T: Iterator<Item = Meta>>(&mut self, iter: T) {
        // Invalidate cache early to satisfy SAFETY invariants including safety
        // on "panic" unwind
        self.cache = None;
        self.dynamic.reserve(iter.size_hint().1.unwrap_or(0));
        for v in iter {
            self.dynamic.push(v);
        }
    }

    /// Parses a dictionary from a memory and adds it's content to the
    /// dictionary
    ///
    /// See expected format in [add_from_file](Self::add_from_file) method
    /// documentation.
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    pub fn add_from_memory(&mut self, buf: impl io::Read) -> Result<()> {
        let mut reader = io::BufReader::new(buf);
        let mut dict = Vec::<Meta>::new();

        let mut line_number = 1;
        let mut line = String::new();
        line.reserve(1024);
        while let Ok(size) = reader.read_line(&mut line) {
            if size == 0 {
                break;
            }
            if let Some(tag_info) = Self::dict_parse_line(line_number, line.trim())? {
                dict.push(tag_info);
            }
            line.clear();
            line_number += 1;
        }
        // Invalidate cache early to satisfy SAFETY invariants including safety on "panic" unwind
        self.cache = None;
        self.add_dynamic_list(dict.into_iter());

        Ok(())
    }

    /// Reads a dictionary file and adds it content to the dictionary
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    ///
    /// File format documentation:\
    /// Each line represents an entry in the data dictionary. Each line has 6
    /// fields `Tag`, `Name`, `Keyword`, `VR`, `VM` and `Version`.
    ///
    /// Entries need not be in ascending tag order. Entries may override
    /// existing entries. Each field must be separated by a single tab.
    ///
    /// `Tag` field must in form `(gggg,eeee[,"creator"])` where `gggg`, `eeee`
    /// must be in hexadecimal form with exception of `X` character, which
    /// denotes "any digit". `creator` string is optional and specifies Private
    /// Attribute creator. If present, it must be enclosed in double quotes and
    /// separated by comma from an adjacent element number.
    ///
    /// `Name` field should contain only graphical ASCII characters and white
    /// space ```[\x20-\x7E]```. Maximum length is 128 bytes.
    ///
    /// `Keyword` field should contain only a subset of ASCII characters
    /// ```[A-Za-z0-9_]``` preferably in CamelCase. Keyword should start with a
    /// letter. Maximum length is 64 bytes.
    ///
    /// `VR` field can contain up to three Value Representation names separated
    /// with " or " Undefined VR should be written as "--".
    ///
    /// `VM` field should contain one of the forms: `B`, `B-E`, `B-n`, `B-Bn`,
    /// where `B` - minimum number of repetitions 0 to 255, `E` - maximum number
    /// of repetitions 1 to 255, `n` - literal "n" symbol, which denotes
    /// "unbounded". Special form `B-Bn` means "arbitrary number multiple of B".
    ///
    /// `Version` field should contain one of the following terms (case
    /// insensitive):
    /// - `DICOM` - standard DICOM attribute
    /// - `DICONDE` - standard DICONDE attribute
    /// - `DICOS` - standard DICOS attribute
    /// - `Ret` - retired attribute from an unspecified source.
    /// - `Priv` - This is a private attribute known not to contain any patient
    ///   identifying information.
    /// - `Priv(X)` - This is a private attribute that contains patient
    ///   identifying information. 'X' specifies a method of "de-identification"
    ///   for this attribute and should be one of the following:
    /// - `D` - replace with a non-zero length value that may be a dummy value
    ///   and consistent with the VR
    /// - `Z` - replace with a zero length value, or a non-zero length value
    ///   that may be a dummy value and consistent with the VR
    /// - `X` - remove
    /// - `U` - replace with a non-zero length UID that is internally consistent
    ///   within a set of Instance
    ///
    /// Comments have a '#' at the beginning of the line. The file should be
    /// encoded as UTF-8 without BOM.
    ///
    /// Example line(tabs were replaced by spaces for documentation):
    /// ```text
    /// (0010,0020) Patient ID  PatientID   LO  1   dicom
    /// ```
    pub fn add_from_file(&mut self, file_name: impl AsRef<Path>) -> Result<()> {
        use std::fs::File;
        let file = File::open(file_name.as_ref()).context(DictFileOpenFailedSnafu {
            file_name: file_name.as_ref().to_path_buf(),
        })?;
        self.add_from_memory(file)
    }

    /// Rebuilds a cache
    pub fn rebuild_cache(&mut self) {
        let mut cache = DictCache { vec: Vec::new() };

        let guessed_total_count =
            self.statics.iter().fold(0, |acc, dict| acc + dict.0.len()) + self.dynamic.len();
        cache.vec.reserve(guessed_total_count);

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

        cache.vec.sort_by(Self::cmp_cache);
        cache
            .vec
            .dedup_by(|l, r| Self::cmp_cache(l, r) == Ordering::Equal);

        self.cache = Some(cache);
    }

    /// Searches a dictionary for the given [TagKey]
    ///
    /// This method does not honor Private Creators so it's usage
    /// should be carefully judged. It searches for the first
    /// [Meta] entry which `Meta.tag.key` matches the searched [TagKey]
    /// combined with a [Meta::mask]
    ///
    /// If cache is invalidated, linear search is performed against dynamic
    /// and all the static lists. If cache is available, binary search is
    /// performed.
    pub fn search_by_key(&self, key: TagKey) -> Option<&Meta> {
        // Search in the cache if it is available
        if let Some(cache) = &self.cache {
            if cache.vec.is_empty() {
                return None;
            }
            return Self::search_in_cache_ignore_creator(cache, key);
        }
        // Search the hard-way.
        let tag = Tag::new(key, None);

        let mut best_match = match Self::search_in_ary(self.dynamic.iter(), &tag) {
            None => None,
            Some((true, meta)) => return Some(meta),
            Some((false, meta)) => Some(meta),
        };

        for ary in self.statics.iter().rev() {
            match Self::search_in_ary(ary.0.iter(), &tag) {
                Some((true, meta)) => {
                    return Some(meta);
                }
                Some((false, meta)) => {
                    best_match.get_or_insert(meta);
                }
                None => (),
            }
        }

        best_match
    }

    /// Searches a dictionary for the given [Tag]
    ///
    /// It searches for the first [Meta] entry which `Meta.tag.key` matches the
    /// searched [Tag] combined with a [Meta::mask] exactly.
    ///
    /// In case of private attributes with [Some] creator, method also tried to
    /// find [canonical](TagKey::to_canonical_if_private) representation of the
    /// private attribute.
    ///
    /// If cache is invalidated, linear search is performed against dynamic and
    /// all the static lists. If cache is available, binary search is performed.
    pub fn search_by_tag<'a>(&self, tag: &'a Tag<'a>) -> Option<&Meta> {
        if !tag.key.is_private_attribute() {
            // This search will be slightly faster, because no "creator" comparisons involved
            return self.search_by_key(tag.key);
        }

        // Search in the cache if it is available
        if let Some(cache) = &self.cache {
            if cache.vec.is_empty() {
                return None;
            }
            // Step 1: exact original
            if let Some(meta) = Self::search_in_cache_exact(cache, tag.key, &tag.creator) {
                return Some(meta);
            }
            if tag.creator.is_some() {
                // Step 2 a: if creator: exact canonical
                if let Some(canonical_key) = tag.key.to_canonical_if_private() {
                    if let Some(meta) =
                        Self::search_in_cache_exact(cache, canonical_key, &tag.creator)
                    {
                        return Some(meta);
                    }
                }
                // Step 3: find an exact attribute with None creator
                if let Some(meta) = Self::search_in_cache_exact(cache, tag.key, &None) {
                    return Some(meta);
                }
            } else if let Some(meta) = Self::search_in_cache_ignore_creator(cache, tag.key) {
                // Step 2 b: if no creator: exact ignoring creator
                return Some(meta);
            }
            return None;
        }

        // Search the hard-way.
        let mut best_match = match Self::search_in_ary(self.dynamic.iter(), tag) {
            None => None,
            Some((true, meta)) => return Some(meta),
            Some((false, meta)) => Some(meta),
        };

        for ary in self.statics.iter().rev() {
            match Self::search_in_ary(ary.0.iter(), tag) {
                Some((true, meta)) => {
                    return Some(meta);
                }
                Some((false, meta)) => {
                    best_match.get_or_insert(meta);
                }
                None => (),
            }
        }

        best_match
    }

    /// Returns some simple metrics
    pub fn metrics(&self) -> DictMetrics {
        DictMetrics {
            static_lists: self.statics.len(),
            static_tags: self.statics.iter().fold(0, |v, c| v + c.0.len()),
            dynamic_tags: self.dynamic.len(),
            cached_tags: self.cache.as_ref().map(|c| c.vec.len()),
        }
    }
}

impl Clone for Dictionary {
    fn clone(&self) -> Self {
        Self {
            statics: self.statics.clone(),
            dynamic: self.dynamic.clone(),
            cache: None,
        }
    }
}

impl Debug for Dictionary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = self.metrics();
        write!(
            f,
            "tag::Dictionary(tags: {} lists, {} static, {} dynamic, {:?} cached)",
            m.static_lists, m.static_tags, m.dynamic_tags, m.cached_tags
        )
    }
}

// ---------------------------------------------------------------------------
// Dictionary private methods implementation
// ---------------------------------------------------------------------------

/// Shortcut to create [DictParseErr]
///
/// Expected parameters
/// - Reference to offending string (must be a slice of input line)
/// - Index of the offending symbol relative to provided slice
/// - String error description
macro_rules! mk_parse_err {
    ($str:expr, $index:expr, $msg:expr) => {
        DictParseErr($str.as_ptr() as usize + $index, ($msg).to_owned())
    };
}

/// Returns error result with [DictParseErr]
/// Same parameters as for [mk_parse_err]
macro_rules! parse_fail {
    ($str:expr, $index:expr, $msg:expr) => {{
        return Err(mk_parse_err!($str, $index, $msg));
    }};
}

/// Checks the condition and returns error result with [DictParseErr] if
/// condition failed.
///
/// Expected parameters:
/// - Condition expression yielding `true` or `false`
/// - Other parameters same as for [mk_parse_err]
macro_rules! parse_ensure {
    ($e:expr, $str:expr, $index:expr, $msg:expr) => {{
        if !($e) {
            parse_fail!($str, $index, $msg);
        }
    }};
}

// Private implementations
impl Dictionary {
    /// Wraps underlying method [dict_parse_line_int](Self::dict_parse_line_int)
    /// and transforms internal parser error to the module-level [`Error`]
    fn dict_parse_line(line_number: usize, line: &str) -> Result<Option<Meta>> {
        Self::dict_parse_line_int(line).map_err(|e| {
            let char_pos = Self::map_offset_to_char_pos(line, e.0);
            Error::DictParseFailed {
                line_number,
                char_pos,
                msg: e.1,
            }
        })
    }

    /// Main function that parses a line of dictionary file returning processed [`Meta`]
    ///
    /// If line is empty or contains only a comment, than Ok(None) returned.
    fn dict_parse_line_int(line: &str) -> Result<Option<Meta>, DictParseErr> {
        let line = line.trim_start();
        if line.starts_with('#') || line.is_empty() {
            return Ok(None);
        }

        let take_first_field = |s| -> Result<(&str, &str), DictParseErr> {
            match Self::dict_parse_take_element(s, "\t") {
                (_, None) => parse_fail!(
                    s,
                    s.len().saturating_sub(1),
                    "unexpected end of line, expecting TAB character"
                ),
                (s1, Some(s2)) => Ok((s1, s2)),
            }
        };

        let (field_text, line_left) = take_first_field(line)?;
        let (tag, mask) = Self::dict_parse_field_tag(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let vr = Self::dict_parse_field_vr(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let name = Self::dict_parse_field_name(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let keyword = Self::dict_parse_field_keyword(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let vm = Self::dict_parse_field_vm(field_text)?;

        let source = Self::dict_parse_field_source(line_left)?;

        Ok(Some(Meta {
            tag,
            mask,
            vr,
            vm,
            name,
            keyword,
            source,
        }))
    }

    /// Helper function, that translates byte-offset in the string to the char-offset.
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

    /// Splits a string with delimiter. Returns first and second halves.
    ///
    /// Somewhat resembles `strtok` from C world.
    fn dict_parse_take_element<'b>(s: &'b str, sep: &'_ str) -> (&'b str, Option<&'b str>) {
        match s.find(sep) {
            None => (s, None),
            Some(index) => (&s[0..index], Some(&s[index + sep.len()..])),
        }
    }

    /// Parses a tag component (group or element) from a string line.
    ///
    /// Expects exactly 4 hexadecimal or 'X' characters.
    ///
    /// Returns numeric component and it's mask.
    fn dict_parse_tag_component(s: &str) -> Result<(u16, u16), DictParseErr> {
        let s = s.trim();
        let mut mask = 0u16;
        let mut number = 0u16;
        let mut it = s.chars();
        let mut byte_offset = 0usize;
        for n in 0usize..4 {
            let c = it.next().ok_or_else(|| {
                mk_parse_err!(
                    s,
                    byte_offset,
                    "unexpected end of Tag component, expecting 4 hexadecimal or 'x' characters"
                )
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
            "unexpected extra character after Tag component."
        );
        Ok((number, !mask))
    }

    /// Parses [`Tag`] and it's mask from the input string slice.
    ///
    /// Expects format `(gggg,eeee[,"creator"])` where `gggg`, `eeee` - hexadecimal
    /// or 'X' characters.
    ///
    /// Returns a parsed `Tag` and it's mask (synthesized from 'X' characters).
    fn dict_parse_field_tag(s: &str) -> Result<(Tag<'static>, u32), DictParseErr> {
        let s = s.trim();
        parse_ensure!(
            !s.is_empty() && s.starts_with('(') && s.ends_with(')'),
            s,
            0,
            "expecting Tag definition in parentheses"
        );
        // Remove surrounding parentheses
        let s = &s[1..s.len() - 1];

        let (group_chars, line_left) = Self::dict_parse_take_element(s, ",");
        let (group, group_mask) = Self::dict_parse_tag_component(group_chars)?;

        parse_ensure!(
            line_left.is_some(),
            s,
            s.len().saturating_sub(1),
            "expecting comma after Tag group number"
        );
        // Panic safety: Before unwrapping, we've checked "line_left.is_some()"
        let (element_chars, creator_chars) = Self::dict_parse_take_element(line_left.unwrap(), ",");
        let (element, element_mask) = Self::dict_parse_tag_component(element_chars)?;

        let creator: Option<Cow<'static, str>> = match creator_chars {
            None => None,
            Some(creator) => {
                let creator = creator.trim();
                parse_ensure!(
                    creator.len() >= 3 && creator.starts_with('"') && creator.ends_with('"'),
                    creator,
                    0,
                    "expecting non-empty private creator string in double quotes"
                );
                let creator = &creator[1..creator.len() - 1];

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
                        "private creator is too long ({chars_count} chars), maximum 64 chars allowed"
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

    /// Parses name of element
    ///
    /// Expects a string of maximum 128 characters composed of only ASCII graphic and white
    /// space characters.
    ///
    /// Returns trimmed version of the source string
    fn dict_parse_field_name(s: &'_ str) -> Result<Cow<'static, str>, DictParseErr> {
        let s = s.trim();
        parse_ensure!(!s.is_empty(), s, 0, "unexpected empty Name field");
        parse_ensure!(
            s.len() <= 128,
            s,
            0,
            format!(
                "Name field is too long ({} bytes), maximum 128 chars allowed",
                s.len()
            )
        );
        if let Some(index) = s.bytes().position(|c| !c.is_ascii_graphic() && c != b' ') {
            parse_fail!(
                s,
                index,
                format!(
                    "invalid character \"{}\" in Name field. only space and ascii graphic chars allowed",
                    s.as_bytes()[index].to_owned().escape_ascii()
                )
            );
        }
        Ok(Cow::Owned(s.to_owned()))
    }

    /// Parses keyword(identifier) of element
    ///
    /// Expects a string of maximum 64 characters composed of only alphanumeric
    /// and '_' characters. String must start with an alphabetic character.
    ///
    /// Returns trimmed version of the source string.
    fn dict_parse_field_keyword(s: &'_ str) -> Result<Cow<'static, str>, DictParseErr> {
        let s = s.trim();
        parse_ensure!(!s.is_empty(), s, 0, "unexpected empty Keyword field");
        parse_ensure!(
            s.len() <= 64,
            s,
            0,
            format!(
                "Keyword field is too long ({} bytes), maximum 64 chars allowed",
                s.len()
            )
        );
        let c = s.as_bytes()[0];
        parse_ensure!(
            c.is_ascii_alphabetic(),
            s,
            0,
            format!(
                "unexpected non-alphabetic first character \"{}\"  in Keyword field",
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

    /// Parses value representation code
    ///
    /// Expects one to three [`Vr`] codes separated by " or ".
    ///
    /// Returns a tuple of all the Vr's in the input string. Missing entries are
    /// set to [Vr::Undefined]
    fn dict_parse_field_vr(s: &'_ str) -> Result<(Vr, Vr, Vr), DictParseErr> {
        fn parse_vr(vr_text: &str) -> Result<Vr, DictParseErr> {
            let vr_text = vr_text.trim();
            parse_ensure!(
                !vr_text.is_empty(),
                vr_text,
                0,
                "empty string found. expecting AE, AS, AT, etc"
            );
            let vr = Vr::try_from(vr_text).map_err(|_| {
                mk_parse_err!(
                    vr_text,
                    0,
                    format!(
                        "unsupported VR \"{}\". expecting AE, AS, AT, etc",
                        vr_text.escape_default()
                    )
                )
            })?;
            Ok(vr)
        }
        let (vr_text, line_left) = Self::dict_parse_take_element(s, " or ");
        let vr_text = vr_text.trim();
        let vr1 = parse_vr(vr_text)?;

        let mut vr2 = Vr::Undefined;
        let mut vr3 = Vr::Undefined;
        if let Some(s) = line_left {
            let (vr_text, line_left) = Self::dict_parse_take_element(s, " or ");
            let vr_text = vr_text.trim();
            vr2 = parse_vr(vr_text)?;
            if let Some(s) = line_left {
                let (vr_text, line_left) = Self::dict_parse_take_element(s, " or ");
                if let Some(s) = line_left {
                    parse_fail!(s, 0, "too many VR values, maximum 3 allowed");
                }
                let vr_text = vr_text.trim();
                vr3 = parse_vr(vr_text)?;
            }
        }
        Ok((vr1, vr2, vr3))
    }

    /// Parses a value representation expression
    ///
    /// Expects a string in form `A`, `B-C`, `B-n`, `A-An` where `A`, `B` and
    /// `C` - decimal numbers. `A` in range 1..=255, `B`, `C` in range 1..=255,
    /// `C` >= `B`.
    ///
    /// Returns a tuple of 3 numbers as described in [Meta::vm]
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

    /// Parses a tag source information.
    ///
    /// Expects one of predefined string (see function body)
    ///
    /// Returns `Source` corresponding to the input string.
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
            Err(mk_parse_err!(
                s,
                0,
                "unrecognized Source field. expected Diconde, Dicos, Ret, Priv, Priv(d|z|x|u)"
            ))
        }
    }

    /// Adds a specified tag to the end of provided vector
    fn add_cached_tag(cache: &mut DictCache, tag_info: &Meta) {
        let key = &tag_info.tag.key;

        // Add tag as-is
        cache.vec.push((
            TagKey(tag_info.tag.key.as_u32() & tag_info.mask),
            NonNull::from(&tag_info.tag.creator),
            NonNull::from(tag_info),
        ));

        // Note: if you did not provide 'private creator' for the private
        // attribute, then it will be matched "exactly". This is a
        // "compatibility" feature with some buggy software, that does not
        // provide any "private creator" for their attributes. If private
        // attribute in non-canonical form and has a creator specified, then it
        // will also lands in cache in canonical form.
        if tag_info.tag.creator.is_some() {
            // Add canonical form
            if let Some(normalized_key) = key.to_canonical_if_private() {
                cache.vec.push((
                    normalized_key,
                    NonNull::from(&tag_info.tag.creator),
                    NonNull::from(tag_info),
                ));
            }
        }
    }

    /// Performs a binary search of [Tag] in the list of [Meta]'s.
    ///
    /// Private creator is matched exactly as passed.
    ///
    /// Supports masked values by positioning at "lower_bound" of the searched text
    /// and rewinding back.
    fn search_in_cache_exact<'a, 'b>(
        c: &'a DictCache,
        key: TagKey,
        creator: &'b Option<Cow<'b, str>>,
    ) -> Option<&'a Meta> {
        match c
            .vec
            .binary_search_by(|v| Self::cmp_cache_key_creator(v, key, creator))
        {
            Ok(index) => {
                // Exact match found

                // SAFETY "get_unchecked": we've got this VALID index from the
                // vector method and there is no way to mutate vector content
                // after the search. SAFETY "deref *const": all pointers are
                // invalidated when data they point to mutates, so there is no
                // chance for pointer to dangle.
                return Some(unsafe { c.vec.get_unchecked(index).2.as_ref() });
            }
            Err(lower_bound) => {
                // Non exact match found. Index - lower bound
                for index in (0..lower_bound).rev() {
                    // SAFETY "get_unchecked": lower_bound is less or equal to
                    // vector.len(), so index in range to "0 .. lower_bound"
                    // will never got beyond array length. If array is empty,
                    // this range will not yield any indices. SAFETY "deref
                    // *const": all pointers are invalidated when data they
                    // point to mutates, so there is no chance for pointer to
                    // dangle.
                    let info = unsafe { c.vec.get_unchecked(index).2.as_ref() };
                    // We must account possible mask in the meta description.
                    let tag_key_masked = TagKey(key.as_u32() & info.mask);
                    // Early bail out if moved to another key
                    if info.tag.key != tag_key_masked {
                        break;
                    }
                    // Match private creator exactly
                    if info.tag.creator != *creator {
                        continue;
                    }
                    return Some(info);
                }
            }
        };
        None
    }

    /// Performs a binary search of [Tag] in the list of [Meta]'s.
    ///
    /// This method ignores private creator on initial binary search, but when
    /// positioned to "lower_bound" of a searched string peeks one element ahead
    /// and one element behind for the match.
    fn search_in_cache_ignore_creator(c: &DictCache, key: TagKey) -> Option<&Meta> {
        match c
            .vec
            .binary_search_by(|v| Self::cmp_cache_key_creator(v, key, &None))
        {
            Ok(index) => {
                // Exact match found

                // SAFETY "get_unchecked": we've got this VALID index from the
                // vector method and there is no way to mutate vector content
                // after the search. SAFETY "deref *const": all pointers are
                // invalidated when data they point to mutates, so there is no
                // chance for pointer to dangle.
                return Some(unsafe { c.vec.get_unchecked(index).2.as_ref() });
            }
            Err(lower_bound) => {
                // Non exact match found. Index - lower bound. Next entries MAY
                // contain same key, but with private creator set
                if lower_bound < c.vec.len() {
                    // SAFETY "get_unchecked": we've got this VALID index from
                    // the vector method and there is no way to mutate vector
                    // content after the search. SAFETY "deref *const": all
                    // pointers are invalidated when data they point to mutates,
                    // so there is no chance for pointer to dangle.
                    let info = unsafe { c.vec.get_unchecked(lower_bound).2.as_ref() };
                    // We must account possible mask in the meta description.
                    let tag_key_masked = TagKey(key.as_u32() & info.mask);
                    // Early bail out if moved to another key
                    if info.tag.key == tag_key_masked {
                        // Found same key ignoring private creator
                        return Some(info);
                    }
                }
                // Lower - ranked entry may contain our key if it is masked
                if lower_bound > 0 {
                    // SAFETY "get_unchecked": we've got this VALID index from
                    // the vector method and there is no way to mutate vector
                    // content after the search. SAFETY "deref *const": all
                    // pointers are invalidated when data they point to mutates,
                    // so there is no chance for pointer to dangle.
                    let info = unsafe { c.vec.get_unchecked(lower_bound - 1).2.as_ref() };
                    // We must account possible mask in the meta description.
                    let tag_key_masked = TagKey(key.as_u32() & info.mask);
                    // Early bail out if moved to another key
                    if info.tag.key == tag_key_masked {
                        // Found some key with mask ignoring creator
                        return Some(info);
                    }
                }
            }
        };
        None
    }

    /// Comparator function for sorting `DictCache::sorted` array.
    /// It compares elements 0 and 1 of a given tuples.
    fn cmp_cache(l: &DictCacheEntry, r: &DictCacheEntry) -> Ordering {
        match l.0.as_u32().cmp(&r.0.as_u32()) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            // SAFETY: Cache is always cleared when any data it points to
            // mutated, so there is no chance to get a dangling pointer.
            Ordering::Equal => unsafe { (l.1).as_ref().cmp(r.1.as_ref()) },
        }
    }

    /// Comparator function for searching `DictCache::sorted` array
    /// by [TagKey] ignoring any private creators
    fn cmp_cache_key(l: &DictCacheEntry, r: TagKey) -> Ordering {
        l.0.as_u32().cmp(&r.as_u32())
    }

    /// Comparator function for searching `DictCache::sorted` array by [TagKey]
    /// and private creator
    fn cmp_cache_key_creator<'a>(
        l: &DictCacheEntry,
        r_key: TagKey,
        r_creator: &'a Creator<'a>,
    ) -> Ordering {
        match l.0.as_u32().cmp(&r_key.as_u32()) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            // SAFETY: Cache is always cleared when any data it points to
            // mutated, so there is no chance to get a dangling pointer.
            Ordering::Equal => unsafe { l.1.as_ref().cmp(r_creator) },
        }
    }

    /// Performs a linear search of the [Tag] in a specified iterator.
    ///
    /// Returns Some if some match found or None if no matches found. Some
    /// contains a tuple of `bool` and matched `Meta`. `bool` indicates a
    /// confidentiality of a match. `true` - match exacted, `false` match with
    /// some "generalization".
    fn search_in_ary<'a, T: Iterator<Item = &'a Meta>>(
        iter: T,
        tag: &Tag,
    ) -> Option<(bool, &'a Meta)> {
        let mut matched = None;

        if !tag.key.is_private_attribute() {
            // Search for a regular tag
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    return Some((true, v));
                }
            }
        } else if tag.creator.is_none() {
            // Search for a private tag without known "creator".
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    return Some((v.tag.creator.is_none(), v));
                }
            }
        } else if let Some(canonical_key) =
            tag.key.to_canonical_if_private().filter(|v| *v != tag.key)
        {
            // Search for a private attribute with a known "creator"
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    if v.tag.creator.is_none() {
                        matched = Some((false, v)); // there may be a better alternatives
                    } else if v.tag.creator == tag.creator {
                        return Some((true, v));
                    }
                }

                if v.tag.key.as_u32() == canonical_key.as_u32() & v.mask
                    && v.tag.creator == tag.creator
                {
                    return Some((true, v));
                }
            }
        } else {
            // Search for a private attribute in canonical form with a known "creator"
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    if v.tag.creator.is_none() {
                        matched = Some((false, v)); // there may be a better alternative with a known creator
                    } else if v.tag.creator == tag.creator {
                        return Some((true, v));
                    }
                }
            }
        }

        matched
    }
}

// ---------------------------------------------------------------------------
// Meta struct implementation
// ---------------------------------------------------------------------------
impl PartialEq for Meta {
    fn eq(&self, other: &Self) -> bool {
        self.tag.eq(&other.tag)
    }
}

impl<'a> PartialEq<Tag<'a>> for Meta {
    fn eq(&self, other: &Tag<'a>) -> bool {
        self.tag.eq(other)
    }
}

impl PartialEq<TagKey> for Meta {
    fn eq(&self, other: &TagKey) -> bool {
        self.tag.key.eq(other)
    }
}

impl Eq for Meta {}

impl PartialOrd for Meta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.tag.partial_cmp(&other.tag)
    }
}

impl<'a> PartialOrd<Tag<'a>> for Meta {
    fn partial_cmp(&self, other: &Tag<'a>) -> Option<std::cmp::Ordering> {
        self.tag.partial_cmp(other)
    }
}

impl PartialOrd<TagKey> for Meta {
    fn partial_cmp(&self, other: &TagKey) -> Option<std::cmp::Ordering> {
        self.tag.key.partial_cmp(other)
    }
}

impl Ord for Meta {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tag.cmp(&other.tag)
    }
}

// ---------------------------------------------------------------------------
// StaticMetaList struct implementation
// ---------------------------------------------------------------------------
impl StaticMetaList {
    /// Creates a new instance with a specified constant list
    pub const fn new(ml: &'static [Meta]) -> Self {
        Self(ml)
    }
    /// Returns a contained constant list of [`Meta`] objects
    pub const fn value(&self) -> &'static [Meta] {
        self.0
    }
}
