#![cfg_attr(rustfmt, rustfmt_skip)]
//! Tables and function for `ISO_IR_166`

use crate::tables::{BackwardResult, ForwardResult};
// cSpell:disable

// Do not edit! This file was autogenerated with `gen_single_byte_tables.py`
// utility on 2023-02-06 by "stingx" on "DESKTOP-2IQN19A".

/// Code Table ISO/IEC 8859-11(TIS 620-2533(1990)) Thai (`G1` in `ISO_IR 166`)
static ISO_IR_166: [u16; 96] = [
    0x00A0, 0x0E01, 0x0E02, 0x0E03, 0x0E04, 0x0E05, 0x0E06, 0x0E07,
    0x0E08, 0x0E09, 0x0E0A, 0x0E0B, 0x0E0C, 0x0E0D, 0x0E0E, 0x0E0F,
    0x0E10, 0x0E11, 0x0E12, 0x0E13, 0x0E14, 0x0E15, 0x0E16, 0x0E17,
    0x0E18, 0x0E19, 0x0E1A, 0x0E1B, 0x0E1C, 0x0E1D, 0x0E1E, 0x0E1F,
    0x0E20, 0x0E21, 0x0E22, 0x0E23, 0x0E24, 0x0E25, 0x0E26, 0x0E27,
    0x0E28, 0x0E29, 0x0E2A, 0x0E2B, 0x0E2C, 0x0E2D, 0x0E2E, 0x0E2F,
    0x0E30, 0x0E31, 0x0E32, 0x0E33, 0x0E34, 0x0E35, 0x0E36, 0x0E37,
    0x0E38, 0x0E39, 0x0E3A, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0x0E3F,
    0x0E40, 0x0E41, 0x0E42, 0x0E43, 0x0E44, 0x0E45, 0x0E46, 0x0E47,
    0x0E48, 0x0E49, 0x0E4A, 0x0E4B, 0x0E4C, 0x0E4D, 0x0E4E, 0x0E4F,
    0x0E50, 0x0E51, 0x0E52, 0x0E53, 0x0E54, 0x0E55, 0x0E56, 0x0E57,
    0x0E58, 0x0E59, 0x0E5A, 0x0E5B, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
];

/// Conversion single-byte -> unicode for `ISO_IR_166`
pub fn forward_g1_iso_ir_166(input: &[u8]) -> ForwardResult {
    let c = input[0];
    match c {
        // CL, GL
        ..=0x7F => (1, None),
        // CR
        0x80..=0x9F => (1, Some(c as u32)),
        // GR
        _ => match ISO_IR_166[(c - 0xA0) as usize] {
            0xFFFD => (1, None),
            c => (1, Some(c as u32)),
        },
    }
}

/// Conversion unicode -> single-byte for `ISO_IR_166`
pub fn backward_g1_iso_ir_166(output: &mut [u8], code: u32) -> BackwardResult {
    match code {
        // CL, GL, Invalid
        ..=0x7F | 0xFFFD | 0x10000.. => None,
        // CR
        0x80..=0x9F => {
            output[0] = code as u8;
            Some(1)
        },
        // GR
        _ => ISO_IR_166.iter()
            .position(|&c| c as u32 == code)
            .map(|index| {
                output[0] = (index + 0xA0) as u8;
                1
            }),
    }
}
