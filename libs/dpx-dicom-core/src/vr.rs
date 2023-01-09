#![allow(non_upper_case_globals)] // Seize bitflags warning

use snafu::{ensure, Snafu};

use core::fmt;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("VR code should be exactly 2 bytes long, {} given", len))]
    InvalidVrLength { len: usize },

    #[snafu(display("unknown VR name \"{}\"", name))]
    UnknownVr { name: String },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;


// cSpell:ignore ZZXX hhmmss

/// A enumeration of known Value Representation codes
///
/// The Value Representation of a Data Element describes the data type and format
/// of that Data Element's Value(s).
///
/// A detailed description could be found in the [DICOM PS 3.5 "6.2 Value Representation (VR)"](https://dicom.nema.org/medical/dicom/current/output/html/part05.html#sect_6.2) standard.
///
/// Note, that some attributes may have a different length or format requirements depending on the context.
/// Some Query/Retrieve matching techniques requires special characters (`*`,`?`,`-`,`=`,`\`, and `"` (QUOTATION MARK) ),
/// which need not be part of the character repertoire for the VR of the Key Attributes. See [DICOM PS 3.4 "C.2.2.2 Attribute Matching"](https://dicom.nema.org/medical/dicom/current/output/html/part04.html#sect_C.2.2.2)
/// for more information.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Vr {
    /// Application Entity
    ///
    /// A string of characters that identifies an Application Entity
    /// - Format: Text, ASCII-only
    ///   - Length: variable, 16 bytes max.
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Allowed bytes: `\x20` to `x7E` except `\x5C` BACKSLASH.
    ///   - Leading and trailing spaces are not significant. Should not consist of only spaces!
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    ///   - Note: `"` character is allowed here, so "Empty Value Matching" against attribute of `""` will fail.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Limit to 16 bytes if string is too long
    ///   - Unsupported characters replaced by `_`
    AE,

    /// Age String
    ///
    /// A string of characters specifying a patient age
    /// - Format: Text, ASCII-only
    ///   - Length: fixed, 4 bytes
    ///   - VM: any, delimited with `\`
    ///   - Padding char: N/A
    ///   - Format regexp: `[0-9]{3}(D|W|M|Y)`
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Trim any white-spaces from beginning and ending of the string
    ///   - Allow any amount of digits that fits into 64-bit integer, then limit it's value to range 0..=999
    ///   - Ignore spaces after digits and before D/W/M/Y suffix.
    ///   - Replace invalid or missing D/W/M/Y suffix with 'Y'
    AS,

    /// Attribute Tag
    ///
    /// A pair of `(group, element)` integers identifying a Data Element Tag
    /// - Format: Binary
    ///   - Length: fixed, 4 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Multiple Value Matching"
    AT,

    /// Code String
    ///
    /// A string of characters identifying a controlled concept.
    /// - Format: Text, ASCII-only
    ///   - Length: variable, 16 bytes max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Allowed bytes: `[A-Z0-9 _]`.
    ///   - Leading and trailing spaces are not significant.
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Limit to 16 bytes if string is too long
    ///   - Permit all symbols in the range \x20 - \x7F. All other symbols are replaced with '_'
    CS,

    /// Date
    ///
    /// A string of characters describing a day of year.
    /// - Format: Text, ASCII-only
    ///   - Length: fixed, 8 bytes
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space) for Q/R. N/A in other cases
    ///   - Format regexp: `[0-9]{8}`
    ///   - Format mnemonic: YYYYMMDD, where YYYY - year, MM - month, DD - day of month
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Range Matching", "Empty Value Matching"
    ///   - Note: In context of "Range Matching", length is 18 bytes fixed (space padded) and `-` character allowed.
    ///   - Note: In context of "Empty Value Matching", length is 2 bytes fixed and `"` character is allowed.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Trim any white-spaces from beginning and ending of the string
    ///   - Allow additional formats in all contexts: YYYY, YYYYMM, YYYY MM DD, DD MM YYYY, DD.MM.YYYY, MM/DD/YYYY
    ///   - Allow additional formats in non C-FIND contexts: YYYY-MM-DD, DD-MM-YYYY
    ///   - Allow timezone offset `&ZZXX`.
    DA,

    /// Decimal String
    ///
    /// A string of characters representing either a fixed point number or a floating point number.
    /// - Format: Text, ASCII-only
    ///   - Length: variable, 16 bytes max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Allowed bytes: `[0-9Ee.+-]`
    ///   - Leading and trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Ignore possible decimal-group separator symbols `_`, `'`, `` ` `` and any white space.
    ///   - Interpret `,` symbol as decimal-group separator IF `.` decimal point found.
    ///   - Interpret `,` as a decimal point IF `.` was not found
    DS,

    /// Date Time
    ///
    /// A string of characters describing some point in time.
    /// - Format: Text, ASCII-only
    ///   - Length: variable, 26 bytes max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Format mnemonic: `YYYY[MM[DD[hh[mm[ss[.f{1,6}]]]]]][&ZZXX]` where `&` is `+` or `-`, `f` - fraction of a second with 1 to 6 digits precision., `ZZXX` - timezone offset(`ZZ` - hours, `XX` - minutes).
    ///   - Trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Range Matching", "Empty Value Matching"
    ///   - Note: In context of "Range Matching", length is 54 bytes max and `-` character allowed.
    ///   - Note: In context of "Empty Value Matching", length is 2 bytes fixed and `"` character is allowed.
    ///   - Note: Negative timezone offsets are forbidden in C-FIND, because `-` is qualified as "Range Matching"
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes and white-spaces from the end of the string
    ///   - Trim leading white-spaces
    ///   - Allow [RFC 3339] (subset of [ISO 8601]) date-time representation such as `1996-12-19T16:39:57-08:00`
    ///
    /// [RFC 3339]: https://www.rfc-editor.org/rfc/rfc3339
    /// [ISO 8601]: https://en.wikipedia.org/wiki/ISO_8601
    DT,

    /// Floating Point Single
    ///
    /// Single precision binary floating point number represented in IEEE 754:1985 32-bit Floating Point Number Format.
    /// - Format: Binary
    ///   - Length: fixed, 4 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    FL,

    /// Floating Point Double
    ///
    /// Double precision binary floating point number represented in IEEE 754:1985 64-bit Floating Point Number Format.
    /// - Format: Binary
    ///   - Length: fixed, 8 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    FD,

    /// Integer String
    ///
    /// A string of characters representing an Integer in base-10 (decimal)
    /// - Format: Text, ASCII-only
    ///   - Length: variable, 12 bytes max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Allowed bytes: `[0-9+-]`
    ///   - Leading and trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Allow "0x", "0X", "0b" and "0B" prefixes for "HEX" and "BINARY" forms
    ///   - Ignore possible decimal-group separator symbols `_`, `'`, `` ` `` and any white space.
    ///   - Interpret `,` symbol as decimal-group separator IF `.` decimal point found.
    ///   - Interpret `,` as a decimal point IF `.` was not found
    ///   - Ignore any text after decimal point, `E` or `e` (treat float as integer)
    ///   - Saturate to 32-bit min/max integer if overflow
    IS,

    /// Long String
    ///
    /// A character string
    /// - Format: Text, subject to `Specific Character Set (0008,0005)`
    ///   - Length: variable, 64 chars max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Allowed chars: any valid unicode greater or equal to `\x20` except `\x5C` BACKSLASH
    ///   - Leading and trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    ///   - Note: `"` character is allowed here, so "Empty Value Matching" against attribute of `""` will fail.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Limit to 64 bytes if string is too long
    ///   - Disallowed characters replaced by `?`
    LO,

    /// Long Text
    ///
    /// A character string that may contain one or more paragraphs
    /// - Format: Text, subject to `Specific Character Set (0008,0005)`
    ///   - Length: variable, 10240 chars max
    ///   - VM: 0 or 1
    ///   - Padding char: `\x20` (space)
    ///   - Allowed chars: `\x09` (TAB), '\x0A' (LF), `\x0C` (FF), `\x0D` (CR) and any valid unicode greater or equal to `\x20`
    ///   - Trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching",
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Limit to 10240 bytes if string is too long
    ///   - Disallowed characters replaced by `?`
    LT,

    /// Other Byte
    ///
    /// An octet-stream where the encoding of the contents is specified by the negotiated Transfer Syntax.
    /// - Format: Special RAW-only binary
    ///   - Length: variable
    ///   - VM: 1 (even if empty binary)
    /// - Q/R C-FIND not supported
    OB,

    /// Other Double
    ///
    /// A stream of 64-bit IEEE 754:1985 floating point words.
    /// - Format: Special RAW-only binary
    ///   - Length: variable, 2^32-8 bytes max
    ///   - VM: 1 (even if empty binary)
    /// - Q/R C-FIND not supported
    OD,

    /// Other Float
    ///
    /// A stream of 32-bit IEEE 754:1985 floating point words.
    /// - Format: Special RAW-only binary
    ///   - Length: variable, 2^32-8 bytes max
    ///   - VM: 1 (even if empty binary)
    /// - Q/R C-FIND not supported
    OF,

    /// Other Long
    ///
    /// A stream of 32-bit words where the encoding of the contents is specified by the negotiated Transfer Syntax.
    /// - Format: Special RAW-only binary
    ///   - Length: variable
    ///   - VM: 1 (even if empty binary)
    /// - Q/R C-FIND not supported
    OL,

    /// Other 64-bit Very Long
    ///
    /// A stream of 64-bit words where the encoding of the contents is specified by the negotiated Transfer Syntax
    /// - Format: Special RAW-only binary
    ///   - Length: variable
    ///   - VM: 1 (even if empty binary)
    /// - Q/R C-FIND not supported
    OV,

    /// Other Word
    ///
    /// A stream of 16-bit words where the encoding of the contents is specified by the negotiated Transfer Syntax
    /// - Format: Special RAW-only binary
    ///   - Length: variable
    ///   - VM: 1 (even if empty binary)
    /// - Q/R C-FIND not supported
    OW,

    /// Person Name.
    ///
    /// A character string encoded using a 5 component convention. The five components in their order of occurrence are:
    /// family name complex, given name complex, middle name, name prefix, name suffix. Components separated by `\x5E`
    /// CARET "^" character. This group of five components is referred to as a Person Name component group. For the
    /// purpose of writing names in ideographic characters and in phonetic characters, up to 3 groups of components
    /// (see [PS3.5 Annex H], [PS3.5 Annex I] and [PS3.5 Annex J]) may be used. The delimiter for component groups is
    /// character `\x3D` EQUAL "=".
    ///
    /// Precise semantics are defined for each component group. See [PS3.5 Section 6.2.1.2].\
    /// For examples and notes, see [PS3.5 Section 6.2.1.1].
    ///
    /// - Format: Text, subject to `Specific Character Set (0008,0005)`
    ///   - Length: variable, 64 chars max per group, 194 max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Allowed chars: any valid unicode greater or equal to `\x20` except `\x5C` BACKSLASH
    ///   - Leading and trailing spaces are not significant
    ///   - Spaces between group of components and between components are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    ///   - Note: `"` character is allowed here, so "Empty Value Matching" against attribute of `""` will fail.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Every group is limited to 64 chars and whole string is Limited to 194 bytes if it is too long
    ///   - Disallowed characters replaced by `?`
    ///
    /// [PS3.5 Annex H]: https://dicom.nema.org/medical/dicom/current/output/html/part05.html#chapter_H
    /// [PS3.5 Annex I]: https://dicom.nema.org/medical/dicom/current/output/html/part05.html#chapter_I
    /// [PS3.5 Annex J]: https://dicom.nema.org/medical/dicom/current/output/html/part05.html#chapter_J
    /// [PS3.5 Section 6.2.1.2]: https://dicom.nema.org/medical/dicom/current/output/html/part05.html#sect_6.2.1.2
    /// [PS3.5 Section 6.2.1.1]: https://dicom.nema.org/medical/dicom/current/output/html/part05.html#sect_6.2.1.1
    PN,

    /// Short String.
    ///
    /// A character string
    /// - Format: Text, subject to `Specific Character Set (0008,0005)`
    ///   - Length: variable, 16 chars max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Allowed chars: any valid unicode greater or equal to `\x20` except `\x5C` BACKSLASH
    ///   - Leading and trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    ///   - Note: `"` character is allowed here, so "Empty Value Matching" against attribute of `""` will fail.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Limit to 16 bytes if string is too long
    ///   - Disallowed characters replaced by `?`
    SH,

    /// Signed Long.
    ///
    /// Signed binary integer 32 bits long
    /// - Format: Binary
    ///   - Length: fixed, 4 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    SL,

    /// Sequence.
    ///
    /// Value is a Sequence of zero or more Items
    /// - Format: Special list
    ///   - Length: variable, unlimited
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    SQ,

    /// Signed Short.
    ///
    /// Signed binary integer 16 bits long
    /// - Format: Binary
    ///   - Length: fixed, 2 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    SS,

    /// Short Text.
    ///
    /// A character string that may contain one or more paragraphs
    /// - Format: Text, subject to `Specific Character Set (0008,0005)`
    ///   - Length: variable, 1024 chars max
    ///   - VM: 0 or 1
    ///   - Padding char: `\x20` (space)
    ///   - Allowed chars: `\x09` (TAB), '\x0A' (LF), `\x0C` (FF), `\x0D` (CR) and any valid unicode greater or equal to `\x20`
    ///   - Trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching",
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Limit to 1024 bytes if string is too long
    ///   - Disallowed characters replaced by `?`
    ST,

    /// Signed 64-bit Very Long
    ///
    /// Signed binary integer 64 bits long
    /// - Format: Binary
    ///   - Length: fixed, 8 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    SV,

    /// Time.
    ///
    /// A string of characters describing a time of day.
    /// - Format: Text, ASCII-only
    ///   - Length: variable, 14 bytes max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` (space)
    ///   - Format mnemonic: `hhmmss.f`, where `hh` - hour in range 00-23, `mm` - minutes in range 00-59, `ss` - seconds in range 00-60, `f` - fraction of second 1 to 6 digits
    ///   - Trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Range Matching", "Empty Value Matching"
    ///   - Note: In context of "Range Matching", length is 28 bytes max and `-` character allowed.
    ///   - Note: In context of "Empty Value Matching", length is 2 bytes fixed and `"` character is allowed.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Trim any white-spaces from beginning of the string
    ///   - Allow additional formats: hh:mm:ss.f, hh:mm:ss, hh:mm
    ///   - Allow timezone offset `&ZZXX`.
    TM,

    /// Unlimited Characters.
    ///
    /// A character string that may be of unlimited length.
    /// - Format: Text, subject to `Specific Character Set (0008,0005)`
    ///   - Length: variable, 2^32-2 chars max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x20` SPACE
    ///   - Allowed chars: any valid unicode greater or equal to `\x20` except `\x5C` BACKSLASH
    ///   - Leading and trailing spaces are not significant
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    ///   - Note: `"` character is allowed here, so "Empty Value Matching" against attribute of `""` will fail.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Disallowed characters replaced by `?`
    UC,

    /// Unique Identifier.
    ///
    /// A character string containing a UID that is used to uniquely identify a wide variety of items.
    /// The UID is a series of numeric components separated by the period `.` character.
    /// - Format: Text, ASCII-only
    ///   - Length: variable, 64 bytes max
    ///   - VM: any, delimited with `\`
    ///   - Padding char: `\x00` NUL
    ///   - Allowed bytes: `[0-9.]`.
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching", "Multiple Value Matching"
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Limit to 16 bytes if string is too long
    ///   - Permit all symbols in the range \x20 - \x7F. All other symbols are replaced with '_'
    UI,

    /// Unsigned Long. (4 bytes fixed). Binary quint32. Endian depended..
    UL,

    /// Unknown element,
    UN,

    /// URI/URL. (2^32-2 bytes max). The subset of the Default Character Repertoire IETF RFC3986 Section 2, plus the space character permitted only as trailing padding. Leading spaces are not allowed, trailing should be ignored.
    UR,

    /// Unsigned short. (2 bytes fixed). Binary.
    US,

    /// Unlimited text. (no limit, 2^32-2 max). Same as LT.
    UT,

    /// Unsigned 64-bit Very Long
    UV,
}

pub const MAX_VR: Vr = Vr::UV;

bitflags::bitflags! {
    pub struct Flags: u8 {
        const Translatable          = 1u8<<1;
        const NullPadded            = 1u8<<2;
        const KeepLeadingSpaces     = 1u8<<3;
        const KeepTrailingSpaces    = 1u8<<4;
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Kind {
    Bytes,
    Items,
    Text,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}

#[derive(Debug, Clone)]
pub struct Meta {
    pub vr: Vr,
    pub code: &'static str,
    pub kind: Kind,
    pub flags: Flags,
    pub name: &'static str,
}

/// Macro that makes `vr::Info` for a given constants.
///
/// This doesn't allocate.
///
/// Note: Currently rust BUGGY traits implementation does not
/// allow const BitOr, so this macro also "wraps" this operator.
macro_rules! mk_info {
    ($vr:expr, $name:expr, $kind:expr, $($($flags:path)|+)?) => {
        $crate::vr::Meta{vr: $vr, code: stringify!($vr), name: $name, kind: $kind, flags: $crate::vr::Flags::from_bits_truncate(0 $( | $($flags.bits())|+ )? )}
    };
}

impl Vr {
    #[rustfmt::skip]
    pub const fn all() -> &'static [Meta] {
        use Vr::*;
        const LIST: [Meta; MAX_VR as usize + 1] = [
            mk_info!(AE, "Application Entity",          Kind::Text, ),
            mk_info!(AS, "Age String",                  Kind::Text, Flags::KeepLeadingSpaces | Flags::KeepTrailingSpaces),
            mk_info!(AT, "Attribute Tag",               Kind::U32,  ),
            mk_info!(CS, "Code String",                 Kind::Text, ),
            mk_info!(DA, "Date",                        Kind::Text, Flags::KeepLeadingSpaces | Flags::KeepTrailingSpaces),
            mk_info!(DS, "Decimal String",              Kind::Text, ),
            mk_info!(DT, "Date Time",                   Kind::Text, Flags::KeepLeadingSpaces | Flags::KeepTrailingSpaces),
            mk_info!(FL, "Floating Point Single",       Kind::F32,  ),
            mk_info!(FD, "Floating Point Double",       Kind::F64,  ),
            mk_info!(IS, "Integer String",              Kind::Text, ),
            mk_info!(LO, "Long String",                 Kind::Text, Flags::Translatable),
            mk_info!(LT, "Long Text",                   Kind::Text, Flags::Translatable | Flags::KeepLeadingSpaces),
            mk_info!(OB, "Other Byte",                  Kind::Bytes,),
            mk_info!(OD, "Other Double",                Kind::Bytes,),
            mk_info!(OF, "Other Float",                 Kind::Bytes,),
            mk_info!(OL, "Other Long",                  Kind::Bytes,),
            mk_info!(OV, "Other 64-bit Very Long",      Kind::Bytes,),
            mk_info!(OW, "Other Word",                  Kind::Bytes,),
            mk_info!(PN, "Person Name",                 Kind::Text, Flags::Translatable),
            mk_info!(SH, "Short String",                Kind::Text, Flags::Translatable),
            mk_info!(SL, "Signed Long",                 Kind::I32,  ),
            mk_info!(SQ, "Sequence of Items",           Kind::Items,),
            mk_info!(SS, "Signed Short",                Kind::I16,  ),
            mk_info!(ST, "Short Text",                  Kind::Text, Flags::Translatable),
            mk_info!(SV, "Signed 64-bit Very Long",     Kind::I64,  ),
            mk_info!(TM, "Time",                        Kind::Text, Flags::KeepLeadingSpaces | Flags::KeepTrailingSpaces),
            mk_info!(UC, "Unlimited Characters",        Kind::Text, Flags::Translatable | Flags::KeepLeadingSpaces),
            mk_info!(UI, "Unique Identifier (UID)",     Kind::Text, Flags::NullPadded),
            mk_info!(UL, "Unsigned Long",               Kind::U32,  ),
            mk_info!(UN, "Unknown",                     Kind::Bytes,),
            mk_info!(UR, "URI/URL",                     Kind::Text, Flags::KeepLeadingSpaces),
            mk_info!(US, "Unsigned Short",              Kind::U16,  ),
            mk_info!(UT, "Unlimited Text",              Kind::U16,  Flags::Translatable | Flags::KeepLeadingSpaces),
            mk_info!(UV, "Unsigned 64-bit Very Long",   Kind::U64,  ),
            ];
        &LIST
    }

    pub const fn info(&self) -> &'static Meta {
        &Self::all()[*self as usize]
    }

    pub const fn code(&self) -> &'static str {
        &self.info().code
    }
}

impl fmt::Debug for Vr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VR({})", self.code())
    }
}

impl fmt::Display for Vr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl TryFrom<&[u8]> for Vr {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        ensure!(value.len() == 2, InvalidVrLengthSnafu { len: value.len() });
        let all = Self::all();
        match all.binary_search_by(|v| v.code.as_bytes().cmp(value)) {
            Ok(idx) => Ok(all[idx].vr),
            Err(_) => UnknownVrSnafu {
                name: value.escape_ascii().to_string(),
            }
            .fail(),
        }
    }
}

impl TryFrom<&str> for Vr {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        ensure!(value.len() == 2, InvalidVrLengthSnafu { len: value.len() });
        let all = Self::all();
        match all.binary_search_by(|v| v.code.cmp(value)) {
            Ok(idx) => Ok(all[idx].vr),
            Err(_) => UnknownVrSnafu {
                name: value.escape_default().to_string(),
            }
            .fail(),
        }
    }
}

impl std::str::FromStr for Vr {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}
