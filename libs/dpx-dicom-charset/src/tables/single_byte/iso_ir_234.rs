#![cfg_attr(rustfmt, rustfmt_skip)]
//! Tables and function for `ISO_IR_234`

use crate::tables::{BackwardResult, ForwardResult};
// cSpell:disable

// Do not edit! This file was autogenerated with `gen_single_byte_tables.py`
// utility on 2023-02-06 by "stingx" on "DESKTOP-2IQN19A".

/// Code Table ISO/IEC 8859-8 Hebrew (`G1` in `ISO_IR 138` if [Config::use_modern_code_page](crate::Config::use_modern_code_page))
static ISO_IR_234: [u16; 96] = [
    0x00A0, 0xFFFD, 0x00A2, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7,
    0x00A8, 0x00A9, 0x00D7, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00AF,
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7,
    0x00B8, 0x00B9, 0x00F7, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0xFFFD,
    0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
    0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
    0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
    0xFFFD, 0x20AC, 0x20AA, 0x202D, 0x202E, 0x202C, 0xFFFD, 0x2017,
    0x05D0, 0x05D1, 0x05D2, 0x05D3, 0x05D4, 0x05D5, 0x05D6, 0x05D7,
    0x05D8, 0x05D9, 0x05DA, 0x05DB, 0x05DC, 0x05DD, 0x05DE, 0x05DF,
    0x05E0, 0x05E1, 0x05E2, 0x05E3, 0x05E4, 0x05E5, 0x05E6, 0x05E7,
    0x05E8, 0x05E9, 0x05EA, 0x202A, 0x202B, 0x200E, 0x200F, 0xFFFD,
];

/// Conversion single-byte -> unicode for `ISO_IR_234`
pub fn forward_g1_iso_ir_234(input: &[u8]) -> ForwardResult {
    let c = input[0];
    match c {
        // CL, GL
        ..=0x7F => (1, None),
        // CR
        0x80..=0x9F => (1, Some(c as u32)),
        // GR
        _ => match ISO_IR_234[(c - 0xA0) as usize] {
            0xFFFD => (1, None),
            c => (1, Some(c as u32)),
        },
    }
}

/// Conversion unicode -> single-byte for `ISO_IR_234`
pub fn backward_g1_iso_ir_234(output: &mut [u8], code: u32) -> BackwardResult {
    match code {
        // CL, GL, Invalid
        ..=0x7F | 0xFFFD | 0x10000.. => None,
        // CR
        0x80..=0x9F => {
            output[0] = code as u8;
            Some(1)
        },
        // GR
        _ => ISO_IR_234.iter()
            .position(|&c| c as u32 == code)
            .map(|index| {
                output[0] = (index + 0xA0) as u8;
                1
            }),
    }
}