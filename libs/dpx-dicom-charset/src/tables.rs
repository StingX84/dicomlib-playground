pub mod constants;
pub mod multi_byte;
pub mod single_byte;

#[cfg(test)]
#[cfg(not(miri))]
pub(crate) mod tests;

/// Return type for byte-string to unicode conversion functions.
///
/// - Tuple arg 0 - number of bytes consumed
/// - Tuple arg 1 - `Some`(`unicode code point`) if input was recognized, `None`
///   if not.
pub type ForwardResult = (u8, Option<u32>);

/// Return type for unicode to byte-string conversion functions.
///
/// `Some`(number of bytes written) on success or `None` if input code point is
/// not supported.
pub type BackwardResult = Option<u8>;

/// byte-string to unicode decoder function type
///
/// This function takes as much bytes as required to decode a single character.
///
/// # Params:
/// - _input_ - Input bytes. Must contain at least one byte.
///
/// # Returns:
/// Tuple of:
/// - _.0_ -  Number of bytes consumed (even if the character has not been
///   recognized). Always greater than 0.
/// - _.1_ - `Some`(`unicode code point`) if character was recognized, `None` if
///   not
pub type PfnForward = fn(input: &[u8]) -> ForwardResult;

/// unicode to byte-string encoder function type
///
/// # Params:
/// - _output_ - Buffer for the output string. Should be large enough for the
///  encoding. Currently, no encoding will produce more than 4 bytes, but for
///  "future-proof" compatibility, provide 16 bytes.
/// - _code_ - Unicode code point to write. Typically, `char as u32`
///
/// # Returns:
/// `Some(number of bytes written)` on success or `None` on failure.
///
/// # Panics:
/// If output buffer is not long enough. Note: if you disable array index checks
/// in the compiler, this may lead to buffer overflows in some encodings if
/// `output` is less than 4 bytes. Always make sure to provide at least 4 bytes.
pub type PfnBackward = fn(output: &mut [u8], code: u32) -> BackwardResult;

/// ISO-2022 code region (`GL` or `GR`) this code page should be designated to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Region {
    G0,
    G1,
}

/// The type of the ISO-2022 character table
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    /// "dummy" tables containing only invalid code points.
    ///
    /// Such tables used when a `G0` or `G1` region is not designated.
    Unassigned,
    SingleByte,
    MultiByte,
}

#[derive(Clone)]
/// Defines a table for G0 or G1
pub struct Table {
    pub kind: TableKind,
    /// ISO 2022 Escape Sequence used to designate this code page to it's [`region`]
    pub esc: &'static [u8],
    /// Designation [Region]
    pub region: Region,
    /// Modern code-page alternative.
    ///
    /// Used if [crate::Config::use_modern_code_page] set to true. Makes sense
    /// only for ISO-2022 compatible encodings.
    pub modern: Option<&'static Table>,
    /// Decoder function
    pub forward: PfnForward,
    /// Encoder function
    pub backward: PfnBackward,
}

macro_rules! declare_tables {
    ($(
        $(#[$docs:meta])*
        pub static $keyword:ident = { $kind:ident, $esc:literal, $region:expr, $modern:expr, $forward:expr, $backward:expr };
    )*) => {
        use multi_byte::*;
        use single_byte::*;
        use Region::*;
        use TableKind::*;
        $(
            $(#[$docs])*
            pub static $keyword: Table = Table {
                kind: $kind,
                esc: $esc,
                region: $region,
                modern: $modern,
                forward: $forward,
                backward: $backward,
            };
        )*
    };
}

declare_tables! {
    /// `ISO-IR 6` 94set in `G0`
    pub static TABLE_G0_ISO_IR_6 = { SingleByte, b"\x28\x42", G0, None, forward_g0_iso_ir_6, backward_g0_iso_ir_6 };
    /// `ISO-IR 13` 94set in `G1`
    pub static TABLE_G1_ISO_IR_13 = { SingleByte, b"\x29\x49", G1, None, forward_g1_iso_ir_13, backward_g1_iso_ir_13 };
    /// `ISO-IR 14` 96set in `G0`
    pub static TABLE_G0_ISO_IR_14 = { SingleByte, b"\x28\x4A", G0, None, forward_g0_iso_ir_14, backward_g0_iso_ir_14 };
    /// `ISO-IR 100` 96set in `G1`
    pub static TABLE_G1_ISO_IR_100 = { SingleByte, b"\x2D\x41", G1, None, forward_g1_iso_ir_100, backward_g1_iso_ir_100 };
    /// `ISO-IR 101` 96set in `G1`
    pub static TABLE_G1_ISO_IR_101 = { SingleByte, b"\x2D\x42", G1, None, forward_g1_iso_ir_101, backward_g1_iso_ir_101 };
    /// `ISO-IR 109` 96set in `G1`
    pub static TABLE_G1_ISO_IR_109 = { SingleByte, b"\x2D\x43", G1, None, forward_g1_iso_ir_109, backward_g1_iso_ir_109 };
    /// `ISO-IR 110` 96set in `G1`
    pub static TABLE_G1_ISO_IR_110 = { SingleByte, b"\x2D\x44", G1, None, forward_g1_iso_ir_110, backward_g1_iso_ir_110 };
    /// `ISO-IR 126` 96set in `G1`
    pub static TABLE_G1_ISO_IR_126 = { SingleByte, b"\x2D\x46", G1, Some(&TABLE_G1_ISO_IR_227), forward_g1_iso_ir_126, backward_g1_iso_ir_126 };
    /// `ISO-IR 127` 96set in `G1`
    pub static TABLE_G1_ISO_IR_127 = { SingleByte, b"\x2D\x47", G1, None, forward_g1_iso_ir_127, backward_g1_iso_ir_127 };
    /// `ISO-IR 138` 96set in `G1`
    pub static TABLE_G1_ISO_IR_138 = { SingleByte, b"\x2D\x48", G1, Some(&TABLE_G1_ISO_IR_234), forward_g1_iso_ir_138, backward_g1_iso_ir_138 };
    /// `ISO-IR 144` 96set in `G1`
    pub static TABLE_G1_ISO_IR_144 = { SingleByte, b"\x2D\x4C", G1, None, forward_g1_iso_ir_144, backward_g1_iso_ir_144 };
    /// `ISO-IR 148` 96set in `G1`
    pub static TABLE_G1_ISO_IR_148 = { SingleByte, b"\x2D\x4D", G1, None, forward_g1_iso_ir_148, backward_g1_iso_ir_148 };
    /// `ISO-IR 166` 96set in `G1`
    pub static TABLE_G1_ISO_IR_166 = { SingleByte, b"\x2D\x54", G1, None, forward_g1_iso_ir_166, backward_g1_iso_ir_166 };
    /// `ISO-IR 203` 96set in `G1`
    pub static TABLE_G1_ISO_IR_203 = { SingleByte, b"\x2D\x62", G1, None, forward_g1_iso_ir_203, backward_g1_iso_ir_203 };
    /// **non-standard** `ISO-IR 227` 96set in `G1` (modern variant of [TABLE_G1_ISO_IR_126])
    pub static TABLE_G1_ISO_IR_227 = { SingleByte, b"\x2D\x46" /*b"\x2D\x69"*/, G1, None, forward_g1_iso_ir_227, backward_g1_iso_ir_227 };
    /// **non-standard** `ISO-IR 234` 96set in `G1` (modern variant of [TABLE_G1_ISO_IR_138])
    pub static TABLE_G1_ISO_IR_234 = { SingleByte, b"\x2D\x48" /*b"\x2D\x6A"*/, G1, None, forward_g1_iso_ir_234, backward_g1_iso_ir_234 };

    // 94x94 multi-byte
    /// `ISO-IR 87` 94x94set in `G0`
    pub static TABLE_G0_ISO_IR_87 = { MultiByte, b"\x24\x42", G0, None, forward_g0_jisx0208, backward_g0_jisx0208 };
    /// `ISO-IR 159` 94x94set in `G0`
    pub static TABLE_G0_ISO_IR_159 = { MultiByte, b"\x24\x28\x44", G0, None, forward_g0_jisx0212, backward_g0_jisx0212 };
    /// `ISO-IR 149` 94x94set in `G1`
    pub static TABLE_G1_ISO_IR_149 = { MultiByte, b"\x24\x29\x43", G1, None, forward_g1_ksx1001, backward_g1_ksx1001 };
    /// `ISO-IR 58` 94x94set in `G1`
    pub static TABLE_G1_ISO_IR_58 = { MultiByte, b"\x24\x29\x41", G1, None, forward_g1_gb2312, backward_g1_gb2312 };

    // Virtual tables
    /// "virtual" table, that always fails to encode or decode characters
    pub static TABLE_G0_ALWAYS_INVALID = { Unassigned, b"", G0, None, forward_invalid, backward_invalid };
    /// "virtual" table, that always fails to encode or decode characters
    pub static TABLE_G1_ALWAYS_INVALID = { Unassigned, b"", G1, None, forward_invalid, backward_invalid };
    /// "virtual" table, that translates bytes to unicode 1 to 1. Used when processing single-valued `ISO_IR 6`.
    pub static TABLE_G1_ALWAYS_IDENTITY = { Unassigned, b"", G1, None, forward_identity, backward_identity };
}

/// List of all the standard ISO-2022 extension-enabled encodings.
pub static ISO_TABLES: [&Table; 18] = [
    &TABLE_G0_ISO_IR_6,
    &TABLE_G1_ISO_IR_13,
    &TABLE_G0_ISO_IR_14,
    &TABLE_G1_ISO_IR_100,
    &TABLE_G1_ISO_IR_101,
    &TABLE_G1_ISO_IR_109,
    &TABLE_G1_ISO_IR_110,
    &TABLE_G1_ISO_IR_126,
    &TABLE_G1_ISO_IR_127,
    &TABLE_G1_ISO_IR_138,
    &TABLE_G1_ISO_IR_144,
    &TABLE_G1_ISO_IR_148,
    &TABLE_G1_ISO_IR_166,
    &TABLE_G1_ISO_IR_203,
    &TABLE_G0_ISO_IR_87,
    &TABLE_G0_ISO_IR_159,
    &TABLE_G1_ISO_IR_149,
    &TABLE_G1_ISO_IR_58,
];
