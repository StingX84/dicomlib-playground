#![cfg_attr(rustfmt, rustfmt_skip)]
//! Tables and function for `ISO_IR_13`

use crate::tables::{BackwardResult, ForwardResult};
// cSpell:disable

// Do not edit! This file was autogenerated with `gen_single_byte_tables.py`
// utility on 2023-02-06 by "stingx" on "DESKTOP-2IQN19A".

/// Code Table JIS X 0201: Katakana (`G1` in `ISO_IR 13`)
static ISO_IR_13: [u16; 96] = [
    0xFFFD, 0xFF61, 0xFF62, 0xFF63, 0xFF64, 0xFF65, 0xFF66, 0xFF67,
    0xFF68, 0xFF69, 0xFF6A, 0xFF6B, 0xFF6C, 0xFF6D, 0xFF6E, 0xFF6F,
    0xFF70, 0xFF71, 0xFF72, 0xFF73, 0xFF74, 0xFF75, 0xFF76, 0xFF77,
    0xFF78, 0xFF79, 0xFF7A, 0xFF7B, 0xFF7C, 0xFF7D, 0xFF7E, 0xFF7F,
    0xFF80, 0xFF81, 0xFF82, 0xFF83, 0xFF84, 0xFF85, 0xFF86, 0xFF87,
    0xFF88, 0xFF89, 0xFF8A, 0xFF8B, 0xFF8C, 0xFF8D, 0xFF8E, 0xFF8F,
    0xFF90, 0xFF91, 0xFF92, 0xFF93, 0xFF94, 0xFF95, 0xFF96, 0xFF97,
    0xFF98, 0xFF99, 0xFF9A, 0xFF9B, 0xFF9C, 0xFF9D, 0xFF9E, 0xFF9F,
    0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
    0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
    0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
    0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD, 0xFFFD,
];

/// Conversion single-byte -> unicode for `ISO_IR_13`
pub fn forward_g1_iso_ir_13(input: &[u8]) -> ForwardResult {
    let c = input[0];
    match c {
        // CL, GL
        ..=0x7F => (1, None),
        // CR
        0x80..=0x9F => (1, Some(c as u32)),
        // GR
        _ => match ISO_IR_13[(c - 0xA0) as usize] {
            0xFFFD => (1, None),
            c => (1, Some(c as u32)),
        },
    }
}

/// Conversion unicode -> single-byte for `ISO_IR_13`
pub fn backward_g1_iso_ir_13(output: &mut [u8], code: u32) -> BackwardResult {
    match code {
        // CL, GL, Invalid
        ..=0x7F | 0xFFFD | 0x10000.. => None,
        // CR
        0x80..=0x9F => {
            output[0] = code as u8;
            Some(1)
        },
        // GR
        _ => ISO_IR_13.iter()
            .position(|&c| c as u32 == code)
            .map(|index| {
                output[0] = (index + 0xA0) as u8;
                1
            }),
    }
}
