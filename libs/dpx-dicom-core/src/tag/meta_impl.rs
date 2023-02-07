use super::*;
use crate::{utils::unescape::unescape_with_validator, Cow, Vr};
use std::fmt::Display;

// cSpell:ignore strtok тест

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

/// Internal to dicom dictionary parser error reporting structure.
///
/// It contains a byte offset of the character causing the problem
/// in a line and an problem description.
#[cfg_attr(test, derive(Debug))]
struct MetaParseErr(usize, String);

// ---------------------------------------------------------------------------
// Meta struct implementation
// ---------------------------------------------------------------------------
impl Meta {
    /// Returns a string representation of `tag` member combined with `mask`.
    ///
    /// The "masked" half-bytes becomes 'x' symbol. Example
    /// - Tag: 0x12345678, Mask: 0xFFFFFFFF will be formatted as "(1234,5678)"
    /// - Tag: 0x12345678, Mask: 0xFF00FFFF - "(12xx,5678)"
    /// - Tag: 0x12345678, Mask: 0xFFFFFFFF, creator: "test" - "(1234,5678,\"creator\")"
    pub fn tag_string(&self) -> String {
        let mut ret_val = String::with_capacity(match &self.tag.creator {
            Some(x) => 14 + x.len(), // 14 = '('  gggg  ','  eeee ',' '"' '"' ')'
            None => 11,              // 11 = '('  gggg  ','  eeee ')'
        });
        ret_val.push('(');
        const HEX: [char; 16] = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
        ];

        for index in 0u32..8 {
            let bit_offset = 28 - 4 * index;
            let mask_part = (self.mask >> bit_offset) & 0xF;
            let key_part = (self.tag.key.as_u32() >> bit_offset) & 0xF;
            if mask_part == 0 {
                ret_val.push('X');
            } else if mask_part == 1 && key_part == 1 {
                ret_val.push('N');
            } else if mask_part == 1 && key_part == 0 {
                ret_val.push('O');
            } else {
                ret_val.push(HEX[key_part as usize])
            }
            if index == 3 {
                ret_val.push(',');
            }
        }
        match &self.tag.creator {
            Some(x) => {
                ret_val.push_str(",\"");
                ret_val.push_str(x);
                ret_val.push_str("\")");
            }
            None => ret_val.push(')'),
        }
        ret_val
    }

    /// Returns a string representation of `vr` tuple member.
    ///
    /// Returns `--`, `VR`, `VR or VR`, `VR or VR or VR` depending on tuple content.
    pub fn vr_string(&self) -> String {
        match self.vr {
            (v1, Vr::Undefined, Vr::Undefined) => String::from(v1.keyword()),
            (v1, v2, Vr::Undefined) => format!("{v1} or {v2}"),
            (v1, v2, v3) => format!("{v1} or {v2} or {v3}"),
        }
    }

    /// Returns a string representation of 'vm' tuple member.
    /// - If vm.2 > 1, then `{vm.2}-{vm.2}n`
    /// - If vm.1 == 0, then `{vm.0}-n`
    /// - If vm.0 < vm.1, then `{vm.0}-{vm.1}`
    /// - else `{vm.0}`
    pub fn vm_string(&self) -> String {
        if self.vm.2 > 1 {
            format!("{0}-{0}n", self.vm.2)
        } else if self.vm.1 == 0 {
            format!("{}-n", self.vm.0)
        } else if self.vm.0 < self.vm.1 {
            format!("{}-{}", self.vm.0, self.vm.1)
        } else {
            format!("{}", self.vm.0)
        }
    }

    /// Formats this Meta as a `tsv` (Tab Separated Values) line
    pub fn to_tsv_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            self.tag_string(),
            self.vr_string(),
            self.name.escape_default(),
            self.keyword.escape_default(),
            self.vm_string(),
            self.source
        )
    }

    /// Parses a line from `tsv` (Tab Separated Values) file.
    ///
    /// See [Dictionary::add_from_file](crate::tag::Dictionary::add_from_file) for details about expected line format.
    ///
    /// Return:
    /// - `Err(MetaParseFailed)` if parsing failed.
    /// - `Ok(None)` if line is empty or contains only a comment
    /// - `Ok(Some(Meta))` if line successfully parsed
    pub fn from_tsv_line(line: &str) -> Result<Option<Meta>> {
        Self::parse_tsv_line(line).map_err(|e| Error::MetaParseFailed {
            char_pos: Self::byte_offset_to_char_offset(line, e.0),
            msg: e.1,
        })
    }
}

// ---------------------------------------------------------------------------
// Meta private methods implementation
// ---------------------------------------------------------------------------

/// Shortcut to create [DictParseErr]
///
/// Expected parameters
/// - Reference to offending string (must be a slice of input line)
/// - Index of the offending symbol relative to provided slice
/// - String error description
macro_rules! mk_parse_err {
    ($str:expr, $index:expr, $msg:expr) => {
        MetaParseErr($str.as_ptr() as usize + $index, ($msg).to_owned())
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

// Private implementation
impl Meta {
    fn byte_offset_to_char_offset(line: &str, address: usize) -> usize {
        let byte_offset = address - line.as_ptr() as usize;
        line.char_indices()
            .enumerate()
            .find_map(
                |(co, (i, _))| {
                    if i >= byte_offset {
                        Some(co)
                    } else {
                        None
                    }
                },
            )
            .unwrap_or(line.len())
    }

    /// Main function that parses a line of dictionary file returning processed [`Meta`]
    ///
    /// If line is empty or contains only a comment, than Ok(None) returned.
    fn parse_tsv_line(line: &str) -> Result<Option<Meta>, MetaParseErr> {
        let line = line.trim_start();
        if line.starts_with('#') || line.is_empty() {
            return Ok(None);
        }

        let take_first_field = |s| -> Result<(&str, &str), MetaParseErr> {
            match Self::parse_take_element(s, "\t") {
                (_, None) => parse_fail!(
                    s,
                    s.len().saturating_sub(1),
                    "unexpected end of line, expecting TAB character"
                ),
                (s1, Some(s2)) => Ok((s1, s2)),
            }
        };

        let (field_text, line_left) = take_first_field(line)?;
        let (tag, mask) = Self::parse_field_tag(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let vr = Self::parse_field_vr(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let name = Self::parse_field_name(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let keyword = Self::parse_field_keyword(field_text)?;

        let (field_text, line_left) = take_first_field(line_left)?;
        let vm = Self::parse_field_vm(field_text)?;

        let source = Self::parse_field_source(line_left)?;

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

    /// Splits a string with delimiter. Returns first and second halves.
    ///
    /// Somewhat resembles `strtok` from C world.
    fn parse_take_element<'b>(s: &'b str, sep: &'_ str) -> (&'b str, Option<&'b str>) {
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

    fn parse_tag_component(s: &str) -> Result<(u16, u16), MetaParseErr> {
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
            } else if c == 'o' || c == 'O' {
                number |= 0x1 << (4 * (3 - n));
                mask |= 0xE << (4 * (3 - n));
            } else if c == 'n' || c == 'N' {
                mask |= 0xE << (4 * (3 - n));
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
    /// #[rustfmt::skip]
    fn parse_field_tag(s: &str) -> Result<(Tag<'static>, u32), MetaParseErr> {
        let s = s.trim();
        parse_ensure!(
            !s.is_empty() && s.starts_with('(') && s.ends_with(')'),
            s,
            0,
            "expecting Tag definition in parentheses"
        );
        // Remove surrounding parentheses
        let s = &s[1..s.len() - 1];

        let (group_chars, line_left) = Self::parse_take_element(s, ",");
        let (group, group_mask) = Self::parse_tag_component(group_chars)?;

        parse_ensure!(
            line_left.is_some(),
            s,
            s.len().saturating_sub(1),
            "expecting comma after Tag group number"
        );
        // Panic safety: Before unwrapping, we've checked "line_left.is_some()"
        let (element_chars, creator_chars) = Self::parse_take_element(line_left.unwrap(), ",");
        let (element, element_mask) = Self::parse_tag_component(element_chars)?;

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
    fn parse_field_name(s: &'_ str) -> Result<Cow<'static, str>, MetaParseErr> {
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
    fn parse_field_keyword(s: &'_ str) -> Result<Cow<'static, str>, MetaParseErr> {
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
    fn parse_field_vr(s: &'_ str) -> Result<(Vr, Vr, Vr), MetaParseErr> {
        fn parse_vr(vr_text: &str) -> Result<Vr, MetaParseErr> {
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
        let (vr_text, line_left) = Self::parse_take_element(s, " or ");
        let vr_text = vr_text.trim();
        let vr1 = parse_vr(vr_text)?;

        let mut vr2 = Vr::Undefined;
        let mut vr3 = Vr::Undefined;
        if let Some(s) = line_left {
            let (vr_text, line_left) = Self::parse_take_element(s, " or ");
            let vr_text = vr_text.trim();
            vr2 = parse_vr(vr_text)?;
            if let Some(s) = line_left {
                let (vr_text, line_left) = Self::parse_take_element(s, " or ");
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
    fn parse_field_vm(s: &'_ str) -> Result<(u8, u8, u8), MetaParseErr> {
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
    fn parse_field_source(s: &'_ str) -> Result<Source, MetaParseErr> {
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
        } else if s.eq_ignore_ascii_case("invalid") {
            Ok(Source::Invalid)
        } else {
            Err(mk_parse_err!(
                s,
                0,
                "unrecognized Source field. expected Diconde, Dicos, Ret, Priv, Priv(d|z|x|u)"
            ))
        }
    }
}

impl Display for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {} {}",
            self.tag_string(),
            self.vr_string(),
            self.vm_string(),
            self.name.escape_default(),
            self.source,
        )
    }
}

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

impl Source {
    pub const fn as_str(&self) -> &'static str {
        use PrivateIdentificationAction as I;
        match self {
            Self::Invalid => "",
            Self::Dicom => "Dicom",
            Self::Diconde => "Diconde",
            Self::Dicos => "Dicos",
            Self::Retired => "Retired",
            Self::Vendored(I::None) => "Priv",
            Self::Vendored(I::D) => "Priv(D)",
            Self::Vendored(I::Z) => "Priv(Z)",
            Self::Vendored(I::X) => "Priv(X)",
            Self::Vendored(I::U) => "Priv(U)",
        }
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    macro_rules! assert_parser_err {
        ($fn:path, $text:expr, $expected_pos:expr) => {{
            let text: &str = $text;
            match $fn(text) {
                Err(MetaParseErr(pos, _)) => {
                    let pos = Meta::byte_offset_to_char_offset(text, pos);
                    assert!(pos == $expected_pos, "pos \"{}\" is not equal to expected \"{}\" in error from \"{}\"", pos, $expected_pos, stringify!($fn));
                }
                _ => assert!(false, "{} expected to fail", stringify!($fn)),
            }
        }};
        ($fn:path, $text:expr, $expected_pos:expr, $expected_msg:expr) => {{
            let text: &str = $text;
            match $fn(text) {
                Err(MetaParseErr(pos, msg)) => {
                    let pos = Meta::byte_offset_to_char_offset(text, pos);
                    assert!(pos == $expected_pos, "pos \"{}\" is not equal to expected \"{}\" in error from \"{}\"", pos, $expected_pos, stringify!($fn));
                    assert!(msg.contains($expected_msg), "message \"{}\" does not contains expected \"{}\" in error from \"{}\"", msg.escape_default(), $expected_msg.escape_default(), stringify!($fn));
                }
                _ => assert!(false, "{} expected to fail", stringify!($fn)),
            }
        }};
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_line_int() {
        assert!(Meta::parse_tsv_line(" # Comment").unwrap().is_none());
        assert!(Meta::parse_tsv_line(" ").unwrap().is_none());
        assert_eq!(Meta::parse_tsv_line("(0010,0020)\tLO\tPatient ID\tPatientID\t1\tdicom").unwrap().unwrap(),
            Meta{
                tag: Tag::standard(0x0010, 0x0020),
                mask: 0xFFFFFFFFu32,
                vr: (Vr::LO, Vr::Undefined, Vr::Undefined),
                vm: (1, 1, 1),
                name: Cow::Borrowed("Patient ID"),
                keyword: Cow::Borrowed("PatientID"),
                source: Source::Dicom
            });
        assert_eq!(Meta::parse_tsv_line("(xxxo,00xx)\tLO\tPrivate Reservation\tPrivateReservation\t1\tDicom").unwrap().unwrap(),
            Meta{
                tag: Tag::standard(0x0001, 0x0000),
                mask: 0x0001FF00u32,
                vr: (Vr::LO, Vr::Undefined, Vr::Undefined),
                vm: (1, 1, 1),
                name: Cow::Borrowed("Private	Reservation"),
                keyword: Cow::Borrowed("PrivateReservation"),
                source: Source::Dicom
            });
        assert_parser_err!(Meta::parse_tsv_line, "a", 0, "unexpected end of line");
        assert_parser_err!(Meta::parse_tsv_line, "(0010,0020)\tLO\tPatient ID\tPatientID\t1", 36, "unexpected end of line");
        assert_parser_err!(Meta::parse_tsv_line, "(0010,0020)\tLO\tPatient ID\tPatientID\t1\t", 38, "unrecognized Source");
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_take_element() {
        assert_eq!(Meta::parse_take_element("\t", "\t"), ("", Some("")));
        assert_eq!(Meta::parse_take_element("1\t", "\t"), ("1", Some("")));
        assert_eq!(Meta::parse_take_element("\t2", "\t"), ("", Some("2")));
        assert_eq!(Meta::parse_take_element("1\t2", "\t"), ("1", Some("2")));
        assert_eq!(Meta::parse_take_element("", "\t"), ("", None));
        assert_eq!(Meta::parse_take_element("Abc", "\t"), ("Abc", None));
        assert_eq!(Meta::parse_take_element("A", " or "), ("A", None));
        assert_eq!(Meta::parse_take_element("A or", " or "), ("A or", None));
        assert_eq!(Meta::parse_take_element("A or B", " or "), ("A", Some("B")));
        assert_eq!(Meta::parse_take_element("A or B or C", " or "), ("A", Some("B or C")));
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_tag_component() {
        assert_eq!(Meta::parse_tag_component("0123").unwrap(),
            (0x0123, 0xFFFF));
        assert_eq!(Meta::parse_tag_component("  0123  ").unwrap(),
            (0x0123, 0xFFFF));
        assert_eq!(Meta::parse_tag_component("cDeF").unwrap(),
            (0xcdef, 0xFFFF));
        assert_eq!(Meta::parse_tag_component("AxbC").unwrap(),
            (0xA0BC, 0xF0FF));
        assert_eq!(Meta::parse_tag_component("xXxX").unwrap(),
            (0x0000, 0x0000));
        assert_eq!(Meta::parse_tag_component("xXoO").unwrap(),
            (0x0011, 0x0011));
        assert_eq!(Meta::parse_tag_component("xXnN").unwrap(),
            (0x0000, 0x0011));
        assert_eq!(Meta::parse_tag_component("abcO").unwrap(),
            (0xabc1, 0xFFF1));
        assert_eq!(Meta::parse_tag_component("abcN").unwrap(),
            (0xabc0, 0xFFF1));

        assert_parser_err!(Meta::parse_tag_component, "", 0, "unexpected end ");
        assert_parser_err!(Meta::parse_tag_component, "T", 0, "invalid character \"T\"");
        assert_parser_err!(Meta::parse_tag_component, "012", 3, "unexpected end");
        assert_parser_err!(Meta::parse_tag_component, "01234", 4, "unexpected extra");
        assert_parser_err!(Meta::parse_tag_component, "012Z", 3, "invalid character \"Z\"");
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_field_tag() {
        assert_eq!(Meta::parse_field_tag("(4321,5678,\"creator\")").unwrap(),
            (Tag::private(0x4321, 0x5678, "creator"), 0xFFFFFFFFu32));
        assert_eq!(Meta::parse_field_tag("(4321,5678,\"тест\")").unwrap(),
            (Tag::private(0x4321, 0x5678, "тест"), 0xFFFFFFFFu32));
        assert_eq!(Meta::parse_field_tag("(cDeF,xXaB)").unwrap(),
            (Tag::standard(0xcdef, 0x00ab), 0xFFFF00FFu32));
        assert_eq!(Meta::parse_field_tag("(xxxx,xxxx)").unwrap(),
            (Tag::standard(0x0000, 0x0000), 0x00000000u32));
        assert_eq!(Meta::parse_field_tag(" ( 4321 , 5678 , \"creator\" ) ").unwrap(),
            (Tag::private(0x4321, 0x5678, "creator"), 0xFFFFFFFFu32));

        let max_creator = String::from_iter(['Ы'; 64]);
        let long_tag = format!("(4321,5678,\"{max_creator}\")");
        assert_eq!(Meta::parse_field_tag(long_tag.as_str()).unwrap(),
            (Tag::private(0x4321, 0x5678, max_creator.as_str()), 0xFFFFFFFFu32));

        assert_parser_err!(Meta::parse_field_tag, "", 0, "expecting Tag definition");
        assert_parser_err!(Meta::parse_field_tag, "A", 0, "expecting Tag definition");
        assert_parser_err!(Meta::parse_field_tag, "(A", 0, "expecting Tag definition");
        assert_parser_err!(Meta::parse_field_tag, "()", 1, "unexpected end of Tag");
        assert_parser_err!(Meta::parse_field_tag, "(123456)", 5, "unexpected extra");
        assert_parser_err!(Meta::parse_field_tag, "(123,)", 4, "unexpected end of Tag");
        assert_parser_err!(Meta::parse_field_tag, "(123456,)", 5, "unexpected extra");
        assert_parser_err!(Meta::parse_field_tag, "(1234)", 4, "expecting comma");
        assert_parser_err!(Meta::parse_field_tag, "(1234,)", 6, "unexpected end of Tag");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,)", 11, "expecting non-empty private");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\")", 11, "expecting non-empty private");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\"\")", 11, "expecting non-empty private");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\"\\\")", 12, "incomplete escape");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\"\\Z\")", 13, "invalid escape sequence character \"Z\"");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\"\\u012Z\")", 17, "unable to parse escape");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\"\r\")", 12, "invalid character");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\"\\r\")", 12, "invalid character");
        assert_parser_err!(Meta::parse_field_tag, "(1234,6789,\"A\\\\B\")", 13, "invalid character");
        //                       Char metrics for reference:  01234567890123456789012
        //                                                    0         1         2
        let overflow_tag = format!("(1234,6789,\"{max_creator}!\")");
        assert_parser_err!(Meta::parse_field_tag, overflow_tag.as_str(), 12, "creator is too long");

    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_field_name() {
        assert_eq!(Meta::parse_field_name("Patient's ID").unwrap(), "Patient's ID");
        assert_eq!(Meta::parse_field_name("1").unwrap(), "1");
        let max_name = String::from_iter(['A'; 128]);
        assert_eq!(Meta::parse_field_name(max_name.as_str()).unwrap(), max_name);

        assert_parser_err!(Meta::parse_field_name, "", 0, "unexpected empty");
        let overflow_name = String::from_iter(['A'; 129]);
        assert_parser_err!(Meta::parse_field_name, overflow_name.as_str(), 0, "Name field is too long");
        assert_parser_err!(Meta::parse_field_name, "\0", 0, "invalid character \"\\x00\"");
        assert_parser_err!(Meta::parse_field_name, "a\tb", 1, "invalid character \"\\t\"");
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_field_keyword() {
        assert_eq!(Meta::parse_field_keyword("PatientID").unwrap(), "PatientID");
        assert_eq!(Meta::parse_field_keyword("A").unwrap(), "A");
        let max_name = String::from_iter(['A'; 64]);
        assert_eq!(Meta::parse_field_keyword(max_name.as_str()).unwrap(), max_name);

        assert_parser_err!(Meta::parse_field_keyword, "", 0, "unexpected empty");
        let overflow_name = String::from_iter(['A'; 65]);
        assert_parser_err!(Meta::parse_field_keyword, overflow_name.as_str(), 0, "Keyword field is too long");
        assert_parser_err!(Meta::parse_field_keyword, "a!b", 1, "invalid character \"!\"");
        assert_parser_err!(Meta::parse_field_keyword, "0A", 0, "first character \"0\"");
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_field_vr() {
        assert_eq!(Meta::parse_field_vr("UT").unwrap(), (Vr::UT, Vr::Undefined, Vr::Undefined));
        assert_eq!(Meta::parse_field_vr("??").unwrap(), (Vr::Undefined, Vr::Undefined, Vr::Undefined));
        assert_eq!(Meta::parse_field_vr("OB or OW").unwrap(), (Vr::OB, Vr::OW, Vr::Undefined));
        assert_eq!(Meta::parse_field_vr("US or SS or OW").unwrap(), (Vr::US, Vr::SS, Vr::OW));
        assert_eq!(Meta::parse_field_vr("  OB  or  OW  ").unwrap(), (Vr::OB, Vr::OW, Vr::Undefined));

        assert_parser_err!(Meta::parse_field_vr, "", 0, "empty string");
        assert_parser_err!(Meta::parse_field_vr, "ZZ", 0, "unsupported VR \"ZZ\"");
        assert_parser_err!(Meta::parse_field_vr, "UTor ", 0, "unsupported VR \"UTor\"");
        assert_parser_err!(Meta::parse_field_vr, "UT or", 0, "unsupported VR \"UT or\"");
        assert_parser_err!(Meta::parse_field_vr, "UT or ", 6, "empty string");
        assert_parser_err!(Meta::parse_field_vr, "UT or ZZ", 6, "unsupported VR \"ZZ\"");
        assert_parser_err!(Meta::parse_field_vr, "UT or AE or ", 12, "empty string");
        assert_parser_err!(Meta::parse_field_vr, "UT or AE or CS or ", 18, "too many VR");
        //                      Char metrics for reference:  0123456789012345678
        //                                                   0         1
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_field_vm() {
        assert_eq!(Meta::parse_field_vm("0-1").unwrap(), (0, 1, 1));
        assert_eq!(Meta::parse_field_vm("1").unwrap(), (1, 1, 1));
        assert_eq!(Meta::parse_field_vm("255").unwrap(), (255, 255, 1));
        assert_eq!(Meta::parse_field_vm("10-n").unwrap(), (10, 0, 1));
        assert_eq!(Meta::parse_field_vm("10-255").unwrap(), (10, 255, 1));
        assert_eq!(Meta::parse_field_vm("8-8n").unwrap(), (8, 0, 8));

        assert_parser_err!(Meta::parse_field_vm, "", 0, "invalid VM number");
        assert_parser_err!(Meta::parse_field_vm, "-", 0, "invalid VM number");
        assert_parser_err!(Meta::parse_field_vm, "-2", 0, "invalid VM number");
        assert_parser_err!(Meta::parse_field_vm, "1-", 2, "unexpected end of VM");
        assert_parser_err!(Meta::parse_field_vm, "0-0", 2, "zero second");
        assert_parser_err!(Meta::parse_field_vm, "0", 0, "zero first");
        assert_parser_err!(Meta::parse_field_vm, "1n", 1, "unexpected \"n\"");
        assert_parser_err!(Meta::parse_field_vm, "2-1", 0, "second VM number");
        assert_parser_err!(Meta::parse_field_vm, "2-1n", 0, "unequal numbers");
    }

    #[test]
    #[rustfmt::skip]
    fn check_dict_parse_field_source() {
        assert_eq!(Meta::parse_field_source("DiCoM").unwrap(), Source::Dicom);
        assert_eq!(Meta::parse_field_source("DiCoS").unwrap(), Source::Dicos);
        assert_eq!(Meta::parse_field_source("DiCoNdE").unwrap(), Source::Diconde);
        assert_eq!(Meta::parse_field_source("ReT").unwrap(), Source::Retired);
        assert_eq!(Meta::parse_field_source("PrIv").unwrap(), Source::Vendored(PrivateIdentificationAction::None));
        assert_eq!(Meta::parse_field_source("PrIv(d)").unwrap(), Source::Vendored(PrivateIdentificationAction::D));
        assert_eq!(Meta::parse_field_source("PrIv(z)").unwrap(), Source::Vendored(PrivateIdentificationAction::Z));
        assert_eq!(Meta::parse_field_source("PrIv(x)").unwrap(), Source::Vendored(PrivateIdentificationAction::X));
        assert_eq!(Meta::parse_field_source("PrIv(u)").unwrap(), Source::Vendored(PrivateIdentificationAction::U));

        assert_parser_err!(Meta::parse_field_source, "", 0, "unrecognized Source");
        assert_parser_err!(Meta::parse_field_source, "priv(t)", 0, "unrecognized Source");
    }
}
