#![allow(dead_code)]

// cSpell:ignore feff

/// Maximum allowed Unicode code point value
pub const UNI_MAX_LEGAL_UTF32: u32 = 0x0010FFFF;
/// Minimum value of High part of surrogate pair in UTF-16
pub const UNI_SUR_HIGH_MIN: u16 = 0xD800;
/// Maximum value of High part of surrogate pair in UTF-16
pub const UNI_SUR_HIGH_MAX: u16 = 0xDBFF;
/// Minimum value of Low part of surrogate pair in UTF-16
pub const UNI_SUR_LOW_MIN: u16 = 0xDC00;
/// Maximum value of Low part of surrogate pair in UTF-16
pub const UNI_SUR_LOW_MAX: u16 = 0xDFFF;
/// G0 control characters MIN
pub const CL_MIN: u8 = 0;
/// G0 control characters MAX
pub const CL_MAX: u8 = 0x1F;
/// G0 graphic characters MIN
pub const GL_MIN: u8 = 0x20;
/// G0 graphic characters MAX
pub const GL_MAX: u8 = 0x7F;
/// G1 control characters MIN
pub const CR_MIN: u8 = 0x80;
/// G1 control characters MAX
pub const CR_MAX: u8 = 0x9F;
/// G1 graphic characters MIN
pub const GR_MIN: u8 = 0xA0;
/// G1 graphic characters MAX
pub const GR_MAX: u8 = 0xFF;
/// Character `CR` 0x0D
pub const CODE_CR: u8 = b'\r';
/// Character `LF` 0x0A
pub const CODE_LF: u8 = b'\n';
/// Character `TAB` 0x09
pub const CODE_TAB: u8 = b'\t';
/// Character `FF` 0x0C
pub const CODE_FF: u8 = 0x0C;
/// Character `ESC` 0x1B
pub const CODE_ESC: u8 = 0x1B;
/// Character `\` 0x5C
pub const CODE_VALUES_SEPARATOR: u8 = b'\\';
/// Character `\` 0x5C
pub const CHAR_VALUES_SEPARATOR: char = '\\';
/// Replacement character in byte-encoded string
pub const CODE_ASCII_REPLACEMENT: u8 = b'?';
/// Replacement character in byte-encoded string
pub const CHAR_ASCII_REPLACEMENT: char = '?';
/// Replacement unicode character. Typically indicating an invalid character
pub const CODE_INVALID: u16 = 0xFFFD;
