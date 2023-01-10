use core::fmt;
use snafu::{ensure, Snafu};

/// Possible error of text to `Vr` conversion operations
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("VR code should be exactly 2 bytes long, {} given", len))]
    InvalidVrLength { len: usize },

    #[snafu(display("unknown VR name \"{}\"", name))]
    UnknownVr { name: String },
}

/// Result of text to `Vr` conversion operations
pub type Result<T, E = Error> = std::result::Result<T, E>;

// cSpell:ignore ZZXX hhmmss

/// A enumeration of known Value Representation codes
///
/// The Value Representation of a Data Element describes the data type and format
/// of that Data Element's Value(s).
///
/// A detailed description could be found in the [DICOM PS 3.5 "6.2 Value Representation (VR)"] standard.
///
/// Note, that some attributes may have a different length or format requirements depending on the context.
/// Some Query/Retrieve matching techniques requires special characters (`*`,`?`,`-`,`=`,`\`, and `"` (QUOTATION MARK) ),
/// which need not be part of the character repertoire for the VR of the Key Attributes. See [DICOM PS 3.4 "C.2.2.2 Attribute Matching"]
/// for more information.
///
/// [DICOM PS 3.5 "6.2 Value Representation (VR)"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.html
/// [DICOM PS 3.4 "C.2.2.2 Attribute Matching"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part04/sect_C.2.2.2.html
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "&str"),
    serde(into = "String")
)]
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

    /// Floating Point Double
    ///
    /// Double precision binary floating point number represented in IEEE 754:1985 64-bit Floating Point Number Format.
    /// - Format: Binary
    ///   - Length: fixed, 8 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    FD,

    /// Floating Point Single
    ///
    /// Single precision binary floating point number represented in IEEE 754:1985 32-bit Floating Point Number Format.
    /// - Format: Binary
    ///   - Length: fixed, 4 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    FL,

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
    /// [PS3.5 Annex H]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_H.html
    /// [PS3.5 Annex I]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_I.html
    /// [PS3.5 Annex J]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_J.html
    /// [PS3.5 Section 6.2.1.2]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.html#sect_6.2.1.2
    /// [PS3.5 Section 6.2.1.1]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.html#sect_6.2.1.1
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

    /// Unsigned Long
    ///
    /// Unsigned binary integer 32 bits long.
    /// - Format: Binary
    ///   - Length: fixed, 4 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    UL,

    /// Unknown
    ///
    /// An octet-stream where the encoding of the contents is unknown (see [PS3.5 "6.2.2 Unknown (UN) Value Representation"])
    ///
    /// - Format: Binary
    ///   - Length: variable
    ///   - VM: any
    /// - Q/R C-FIND: Not supported
    ///
    /// [PS3.5 "6.2.2 Unknown (UN) Value Representation"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.2.html
    UN,

    /// Universal Resource Identifier or Universal Resource Locator (URI/URL)
    ///
    /// A string of characters that identifies a URI or a URL as defined in [RFC3986].
    /// See description in [PS3.5 "6.2.3 URI/URL (UR) Value Representation"]
    /// - Format: Text, subject to `Specific Character Set (0008,0005)`
    ///   - Length: variable, 2^32-2 chars max
    ///   - VM: 0, 1
    ///   - Padding char: `\x20` SPACE
    ///   - Allowed chars: `[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=-]`
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching", "Wild Card Matching", "Empty Value Matching"
    ///   - Note: `*` and `?` characters are allowed here, so "Single Value Matching" against attribute containing these symbol would yield an unexpected results.
    /// - Fixes when `nonConformingTags` == `fix`:
    ///   - Trim any zero bytes from the end of the string
    ///   - Trim any white-spaces from beginning and ending of the string
    ///   - Invalid UTF-8 characters replaced by `?`
    ///   - URL parsed according to the [URL Standard] (this can convert domain into [Punycode], Percent-encode other parts of the URL)
    ///
    /// [RFC3986]: http://tools.ietf.org/html/rfc3986
    /// [PS3.5 "6.2.3 URI/URL (UR) Value Representation"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_6.2.3.html
    /// [Punycode]: https://en.wikipedia.org/wiki/Punycode
    /// [URL Standard]: https://url.spec.whatwg.org/
    UR,

    /// Unsigned short.
    ///
    /// Unsigned binary integer 16 bits long
    /// - Format: Binary
    ///   - Length: fixed, 2 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    US,

    /// Unlimited Text.
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
    UT,

    /// Unsigned 64-bit Very Long
    ///
    /// Unsigned binary integer 64 bits long
    /// - Format: Binary
    ///   - Length: fixed, 8 bytes
    ///   - VM: any
    /// - Q/R C-FIND:
    ///   - Supports: "Single Value Matching", "Universal Matching"
    UV,
}

/// Maximum value in `Vr` enum
pub const MAX_VR: Vr = Vr::UV;

/// Macro that makes `vr::Meta` for a given constants.
///
/// This doesn't allocate.
///
/// Note: Currently rust BUGGY traits implementation does not
/// allow const BitOr, so this macro also "wraps" this operator.
macro_rules! mk_meta {
    ($vr:expr, $code:expr, $description:expr, $kind:expr) => {
        $crate::vr::Meta {
            vr: $vr,
            code: $code,
            description: $description,
            kind: $kind,
        }
    };
}

impl Vr {
    /// Returns a list of `dpx_dicom_core::vr::Meta` structures describing all known VR constants.
    ///
    /// Example usage:
    /// ```
    /// println!("Supported VR's:");
    /// dpx_dicom_core::Vr::all().iter().for_each(|v|
    ///     println!("{} ({})", std::str::from_utf8(&v.code).unwrap(), v.description)
    /// );
    /// ```
    #[rustfmt::skip]
    pub const fn all() -> &'static [Meta] {
        use Vr::*;
        const LIST: [Meta; MAX_VR as usize + 1] = [
            mk_meta!(AE, [b'A', b'E'], "Application Entity",    Kind::Text { translatable: false, null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(AS, [b'A', b'S'], "Age String",            Kind::Text { translatable: false, null_padded: false, leading_spaces_important: true,  trailing_spaces_important: true}),
            mk_meta!(AT, [b'A', b'T'], "Attribute Tag",         Kind::U32),
            mk_meta!(CS, [b'C', b'S'], "Code String",           Kind::Text { translatable: false, null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(DA, [b'D', b'A'], "Date",                  Kind::Text { translatable: false, null_padded: false, leading_spaces_important: true,  trailing_spaces_important: true}),
            mk_meta!(DS, [b'D', b'S'], "Decimal String",        Kind::Text { translatable: false, null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(DT, [b'D', b'T'], "Date Time",             Kind::Text { translatable: false, null_padded: false, leading_spaces_important: true,  trailing_spaces_important: false}),
            mk_meta!(FD, [b'F', b'D'], "Floating Point Double", Kind::F64),
            mk_meta!(FL, [b'F', b'L'], "Floating Point Single", Kind::F32),
            mk_meta!(IS, [b'I', b'S'], "Integer String",        Kind::Text { translatable: false, null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(LO, [b'L', b'O'], "Long String",           Kind::Text { translatable: true,  null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(LT, [b'L', b'T'], "Long Text",             Kind::Text { translatable: true,  null_padded: false, leading_spaces_important: true,  trailing_spaces_important: false}),
            mk_meta!(OB, [b'O', b'B'], "Other Byte",            Kind::Bytes),
            mk_meta!(OD, [b'O', b'D'], "Other Double",          Kind::Bytes),
            mk_meta!(OF, [b'O', b'F'], "Other Float",           Kind::Bytes),
            mk_meta!(OL, [b'O', b'L'], "Other Long",            Kind::Bytes),
            mk_meta!(OV, [b'O', b'V'], "Other Very Long",       Kind::Bytes),
            mk_meta!(OW, [b'O', b'W'], "Other Word",            Kind::Bytes),
            mk_meta!(PN, [b'P', b'N'], "Person Name",           Kind::Text { translatable: true,  null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(SH, [b'S', b'H'], "Short String",          Kind::Text { translatable: true,  null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(SL, [b'S', b'L'], "Signed Long",           Kind::I32),
            mk_meta!(SQ, [b'S', b'Q'], "Sequence of Items",     Kind::Items),
            mk_meta!(SS, [b'S', b'S'], "Signed Short",          Kind::I16),
            mk_meta!(ST, [b'S', b'T'], "Short Text",            Kind::Text { translatable: true,  null_padded: false, leading_spaces_important: false, trailing_spaces_important: false}),
            mk_meta!(SV, [b'S', b'V'], "Signed Very Long",      Kind::I64),
            mk_meta!(TM, [b'T', b'M'], "Time",                  Kind::Text { translatable: false, null_padded: false, leading_spaces_important: true,  trailing_spaces_important: true}),
            mk_meta!(UC, [b'U', b'C'], "Unlimited Characters",  Kind::Text { translatable: true,  null_padded: false, leading_spaces_important: true,  trailing_spaces_important: false}),
            mk_meta!(UI, [b'U', b'I'], "Unique Identifier",     Kind::Text { translatable: false, null_padded: true,  leading_spaces_important: true,  trailing_spaces_important: true}),
            mk_meta!(UL, [b'U', b'L'], "Unsigned Long",         Kind::U32),
            mk_meta!(UN, [b'U', b'N'], "Unknown",               Kind::Bytes),
            mk_meta!(UR, [b'U', b'R'], "URI/URL",               Kind::Text { translatable: false, null_padded: false, leading_spaces_important: true,  trailing_spaces_important: false}),
            mk_meta!(US, [b'U', b'S'], "Unsigned Short",        Kind::U16),
            mk_meta!(UT, [b'U', b'T'], "Unlimited Text",        Kind::Text { translatable: true,  null_padded: false, leading_spaces_important: true,  trailing_spaces_important: false}),
            mk_meta!(UV, [b'U', b'V'], "Unsigned Very Long",    Kind::U64),
            ];
        &LIST
    }

    /// Returns a structure describing this VR
    ///
    /// Example:
    /// ```
    /// # use dpx_dicom_core::vr::*;
    /// assert!(matches!(Vr::SV.info().kind, Kind::I64));
    /// ```
    pub const fn info(&self) -> &'static Meta {
        &Self::all()[*self as usize]
    }

    /// Returns an array of two u8 elements with a code of this Vr
    ///
    /// Equivalent to
    /// ```
    /// # let my_vr = dpx_dicom_core::Vr::AE;
    /// my_vr.info().code;
    /// ```
    pub const fn code(&self) -> [u8; 2] {
        self.info().code
    }

    /// Returns a code of this Vr as a string slice
    ///
    /// Rough equivalent to
    /// ```
    /// # let my_vr = dpx_dicom_core::Vr::AE;
    /// std::str::from_utf8(&my_vr.info().code).unwrap();
    /// ```
    pub const fn name(&self) -> &'static str {
        // SAFETY: `code` is a static constant under our control.
        // It contains only ASCII characters, so it is a valid UTF-8 sequence.
        unsafe { ::core::str::from_utf8_unchecked(&self.info().code) }
    }

    /// Returns a short description of this Vr
    ///
    /// Equivalent to
    /// ```
    /// # let my_vr = dpx_dicom_core::Vr::AE;
    /// my_vr.info().description;
    /// ```
    pub const fn description(&self) -> &'static str {
        self.info().description
    }
}

impl fmt::Debug for Vr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VR({})", self.name())
    }
}

impl fmt::Display for Vr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl From<Vr> for String {
    fn from(value: Vr) -> Self {
        value.name().to_owned()
    }
}

impl TryFrom<&[u8]> for Vr {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        ensure!(value.len() == 2, InvalidVrLengthSnafu { len: value.len() });
        let all = Self::all();
        match all.binary_search_by(|v| v.code.as_ref().cmp(value)) {
            Ok(idx) => Ok(all[idx].vr),
            Err(_) => UnknownVrSnafu {
                name: value.escape_ascii().to_string(),
            }
            .fail(),
        }
    }
}

impl TryFrom<[u8; 2]> for Vr {
    type Error = Error;
    fn try_from(value: [u8; 2]) -> Result<Self, Self::Error> {
        let all = Self::all();
        match all.binary_search_by(|v| v.code.cmp(&value)) {
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
        match all.binary_search_by(|v| v.code.as_ref().cmp(value.as_bytes())) {
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

/// A enum representing a kind of this VR.
///
/// You rarely need to match against this enum unless
/// developing a custom file parser.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// Values of this element are stored as some byte array and not presentable directly.
    /// For example, pixel data
    Bytes,

    /// Values of this element represents a children datasets in a `SQ` (sequence)
    Items,

    /// Values of this element has a textual form
    Text {
        translatable: bool,
        null_padded: bool,
        leading_spaces_important: bool,
        trailing_spaces_important: bool,
    },

    // Values of this element has `i16` type
    I16,

    // Values of this element has `u16` type
    U16,

    // Values of this element has `i32` type
    I32,

    // Values of this element has `u32` type. This also applies to the `dpx_dicom_core::TagKey`.
    U32,

    // Values of this element has `i64` type
    I64,

    // Values of this element has `u32` type
    U64,

    // Values of this element has `f32` type
    F32,

    // Values of this element has `f64` type
    F64,
}

/// A structure describing generic properties of some Value Representation.
#[derive(Debug, Clone)]
pub struct Meta {
    /// Value Representation this meta structure describes.
    pub vr: Vr,
    /// DICOM term associated with this Value Representation
    ///
    /// This term uniquely identifies this VR in the DICOM file.
    /// Contains only ASCII letters and can be casted to `&str` unsafely.
    pub code: [u8; 2],
    /// The generic storage characteristic for values of this VR
    pub kind: Kind,
    /// Short textual description of this VR.
    pub description: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_retrieve_info() {
        assert_eq!(Vr::AE.name(), "AE");
        assert_eq!(Vr::AE.code(), [b'A', b'E']);
        assert_eq!(Vr::AE.description(), "Application Entity");
        assert_eq!(Vr::UV.name(), "UV");
        assert_eq!(Vr::UV.code(), [b'U', b'V']);
        assert_eq!(format!("{}", Vr::AE), "AE");
        assert_eq!(format!("{:?}", Vr::AE), "VR(AE)");
    }

    #[test]
    fn can_recognize_all_vrs() {
        // Try [u8;2]
        for vr in Vr::all().iter() {
            assert_eq!(Vr::try_from(vr.code).unwrap(), vr.vr)
        }
        // Try &[u8]
        for vr in Vr::all().iter() {
            assert_eq!(Vr::try_from(vr.code.as_slice()).unwrap(), vr.vr)
        }
        // Try &str
        for vr in Vr::all().iter() {
            assert_eq!(
                Vr::try_from(::core::str::from_utf8(&vr.code).unwrap()).unwrap(),
                vr.vr
            )
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn can_use_serde() {
        use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

        let vr = Vr::AE;

        assert_ser_tokens(&vr, &[Token::String("AE")]);
        assert_de_tokens(&vr, &[Token::BorrowedStr("AE")]);
    }
}

#[cfg(all(test, feature = "unstable"))]
mod benches {
    extern crate test;
    use super::*;
    use test::{black_box, Bencher};

    // Implementation of vr::Meta -like structure, but with "code" referenced in a constant memory.
    struct Meta2 {
        vr: Vr,
        code: &'static str,
        kind: Kind,
        description: &'static str,
    }

    // Returns a vector of `vr::Meta` for all the known VR's
    fn meta() -> Vec<Meta> {
        Vr::all().to_vec()
    }

    // Returns a vector of `vr::Meta2` for all the known VR's
    fn meta2() -> Vec<Meta2> {
        Vr::all()
            .iter()
            .map(|v| Meta2 {
                vr: v.vr,
                code: ::core::str::from_utf8(&v.code).unwrap(),
                kind: v.kind,
                description: v.description,
            })
            .collect()
    }

    // Returns a vector of all known VR codes
    fn all_vrs() -> Vec<[u8; 2]> {
        Vr::all().iter().map(|v| v.code).collect()
    }

    fn bench_meta_no_binary(needle: [u8; 2], haystack: &[Meta]) -> Result<&Meta, Error> {
        match haystack.iter().find(|v| v.code == needle) {
            Some(v) => Ok(v),
            None => UnknownVrSnafu {
                name: needle.escape_ascii().to_string(),
            }
            .fail(),
        }
    }

    fn bench_meta_binary(needle: [u8; 2], haystack: &[Meta]) -> Result<&Meta, Error> {
        match haystack.binary_search_by(|v| v.code.cmp(&needle)) {
            Ok(idx) => Ok(&haystack[idx]),
            Err(_) => UnknownVrSnafu {
                name: needle.escape_ascii().to_string(),
            }
            .fail(),
        }
    }

    fn bench_meta2_no_binary(needle: [u8; 2], haystack: &[Meta2]) -> Result<&Meta2, Error> {
        match haystack.iter().find(|v| v.code.as_bytes() == needle) {
            Some(v) => Ok(v),
            None => UnknownVrSnafu {
                name: needle.escape_ascii().to_string(),
            }
            .fail(),
        }
    }

    fn bench_meta2_binary(needle: [u8; 2], haystack: &[Meta2]) -> Result<&Meta2, Error> {
        match haystack.binary_search_by(|v| v.code.as_bytes().cmp(&needle)) {
            Ok(idx) => Ok(&haystack[idx]),
            Err(_) => UnknownVrSnafu {
                name: needle.escape_ascii().to_string(),
            }
            .fail(),
        }
    }

    // Search VR in a Vr::Meta vector using simple "loop"
    #[bench]
    fn vr_lookup_no_binary(b: &mut Bencher) {
        let haystack = meta();
        b.iter(|| {
            for needle in all_vrs() {
                black_box(bench_meta_no_binary(needle, &haystack).unwrap());
            }
        })
    }

    // Search VR in a Vr::Meta vector using binary search
    #[bench]
    fn vr_lookup_binary(b: &mut Bencher) {
        let haystack = meta();
        b.iter(|| {
            for needle in all_vrs() {
                black_box(bench_meta_binary(needle, &haystack).unwrap());
            }
        })
    }

    // Search VR in a Vr::Meta2 vector using simple "loop"
    #[bench]
    fn vr_lookup_in_ptr_no_binary(b: &mut Bencher) {
        let haystack = meta2();
        b.iter(|| {
            for needle in all_vrs() {
                black_box(bench_meta2_no_binary(needle, &haystack).unwrap());
            }
        })
    }

    // Search VR in a Vr::Meta2 vector using binary search
    #[allow(unused_must_use)]
    #[bench]
    fn vr_lookup_in_ptr_binary(b: &mut Bencher) {
        let haystack = meta2();
        b.iter(|| {
            for needle in all_vrs() {
                black_box(bench_meta2_binary(needle, &haystack).unwrap());
            }
        })
    }

    // Search VR with a current implementation. Should roughly equal to `vr_lookup_binary`
    #[allow(unused_must_use)]
    #[bench]
    fn vr_lookup_current(b: &mut Bencher) {
        b.iter(|| {
            for needle in all_vrs() {
                black_box(Vr::try_from(needle).unwrap());
            }
        })
    }
}
