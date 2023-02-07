use crate::tables::constants::*;

/// Enumeration of character classes retrieved with [in_default_repertoire],
/// [in_extended_repertoire]
///
/// Detailed description of the character repertoires and character regions
/// could be found in [PS3.5 "6.1 Support of Character Repertoires"](
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_6.html#sect_6.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharClass {
    /// An invalid character (could not be used in DICOM)
    Invalid,
    // A control character (LF, CR, FF, TAB)
    Control,
    /// Delimiter (Backslash 0x5C)
    Delimiter,
    /// Valid character. 0x20 <= char < 0x7F or for non ISO_IR 6 0xA0 <= char <= 0x10FFFF excluding surrogate regions.
    Default,
}

/// Returns a character class for any character in default repertoire
///
/// A default character repertoire according to a standard is `ISO-IR 6`.
/// This character repertoire includes characters LF, CR, FF, TAB,
/// 0x20 to 0x7E.
///
/// Note: 0x7F is not allowed!
/// > The character DELETE (bit combination 07/15) shall not be used in DICOM character strings.
///
/// Example:
/// ```
/// use dpx_dicom_charset::char_class;
/// assert_eq!(char_class::in_default_repertoire(b'A', b'\\'),
///     char_class::CharClass::Default);
/// assert_eq!(char_class::in_default_repertoire(b'\\', b'\\'),
///     char_class::CharClass::Delimiter);
/// assert_eq!(char_class::in_default_repertoire(b'\r', b'\\'),
///     char_class::CharClass::Control);
/// // any value beyond GL (G0 graphics) character range is invalid
/// assert_eq!(char_class::in_default_repertoire('Ш', b'\\'),
///     char_class::CharClass::Invalid);
/// assert_eq!(char_class::in_default_repertoire(0x0010FFFFu32, b'\\'),
///     char_class::CharClass::Invalid);
/// // CL (G0 control) characters except LF, CR, FF, TAB and ESC are not valid
/// assert_eq!(char_class::in_default_repertoire(0x01u8, b'\\'),
///     char_class::CharClass::Invalid);
/// // 0x7F is not valid despite being in GR (G0 graphics)
/// assert_eq!(char_class::in_default_repertoire(0x88u8, b'\\'),
///     char_class::CharClass::Invalid);
/// ```
pub fn in_default_repertoire<T>(c: T, delimiter: u8) -> CharClass
where
    u32: From<T>,
{
    let c = u32::from(c);
    if delimiter != 0 && c == delimiter as u32 {
        CharClass::Delimiter
    } else if c >= GL_MIN as u32 && c < GL_MAX as u32 {
        // 'c < GL_MAX' "less than", because DICOM forbids usage of 0x7F character!
        CharClass::Default
    } else if c == CODE_LF as u32
        || c == CODE_CR as u32
        || c == CODE_FF as u32
        || c == CODE_TAB as u32
    {
        CharClass::Control
    } else {
        CharClass::Invalid
    }
}

/// Returns a character class for any character in non-default repertoire (i.e.,
/// when code extensions are allowed)
///
/// A non default character repertoire could be any character encoding
/// other than `ISO_IR 6` and `ISO 2022 IR 6` (single-valued) provided
/// in (0008,0005) Specific Character Set.
///
/// DICOM limits allowed characters in CL (0x00-0x1F) and CR (0x80-0x9F)
/// regions and
/// This character repertoire includes characters LF, CR, FF, TAB,
/// 0x20 to 0x7E.
///
/// Note: 0x7F is not allowed!
/// > The character DELETE (bit combination 07/15) shall not be used in DICOM character strings.
///
/// Example:
/// ```
/// use dpx_dicom_charset::char_class;
/// assert_eq!(char_class::in_extended_repertoire(b'A', b'\\'),
///     char_class::CharClass::Default);
/// assert_eq!(char_class::in_extended_repertoire(b'\\', b'\\'),
///     char_class::CharClass::Delimiter);
/// assert_eq!(char_class::in_extended_repertoire(b'\r', b'\\'),
///     char_class::CharClass::Control);
/// assert_eq!(char_class::in_extended_repertoire('Ш', b'\\'),
///     char_class::CharClass::Default);
/// assert_eq!(char_class::in_extended_repertoire(0x0010FFFFu32, b'\\'),
///     char_class::CharClass::Default);
/// // Values exceeding maximum unicode code point are not valid
/// assert_eq!(char_class::in_extended_repertoire(0x00110000u32, b'\\'),
///     char_class::CharClass::Invalid);
/// // Surrogate pair regions are not valid
/// assert_eq!(char_class::in_extended_repertoire(0xD800u32, b'\\'),
///     char_class::CharClass::Invalid);
/// // CL (G0 control) characters except LF, CR, FF, TAB and ESC are not valid
/// assert_eq!(char_class::in_extended_repertoire(0x01u8, b'\\'),
///     char_class::CharClass::Invalid);
/// // CR (G1 control) characters are not valid
/// assert_eq!(char_class::in_extended_repertoire(0x88u8, b'\\'),
///     char_class::CharClass::Invalid);
/// // 0x7F is not valid despite being in GR (G0 graphics)
/// assert_eq!(char_class::in_extended_repertoire(0x88u8, b'\\'),
///     char_class::CharClass::Invalid);
/// ```
pub fn in_extended_repertoire<T>(c: T, delimiter: u8) -> CharClass
where
    u32: From<T>,
{
    let c = u32::from(c);

    // 'c < GL_MAX' "less than", because DICOM forbids usage of 0x7F character!
    if delimiter != 0 && c == delimiter as u32 {
        CharClass::Delimiter
    } else if ((c >= GL_MIN as u32 && c < GL_MAX as u32)
        || (c >= GR_MIN as u32 && c <= UNI_MAX_LEGAL_UTF32))
        && !(c >= UNI_SUR_HIGH_MIN as u32 && c <= UNI_SUR_HIGH_MAX as u32)
        && !(c >= UNI_SUR_LOW_MIN as u32 && c <= UNI_SUR_LOW_MAX as u32)
    {
        CharClass::Default
    } else if c == CODE_LF as u32
        || c == CODE_CR as u32
        || c == CODE_FF as u32
        || c == CODE_TAB as u32
    {
        CharClass::Control
    } else {
        CharClass::Invalid
    }
}
