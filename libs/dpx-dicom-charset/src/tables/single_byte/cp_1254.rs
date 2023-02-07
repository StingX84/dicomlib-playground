#![cfg_attr(rustfmt, rustfmt_skip)]
//! Tables and function for `CP_1254`

use crate::tables::{BackwardResult, ForwardResult};
// cSpell:disable

// Do not edit! This file was autogenerated with `gen_single_byte_tables.py`
// utility on 2023-02-06 by "stingx" on "DESKTOP-2IQN19A".

/// Code Table MS Turkish (non-standard `cp1254`)
static CP_1254: [u16; 128] = [
    0x20AC, 0xFFFD, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021,
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0xFFFD, 0xFFFD, 0xFFFD,
    0xFFFD, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0xFFFD, 0xFFFD, 0x0178,
    0x00A0, 0x00A1, 0x00A2, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7,
    0x00A8, 0x00A9, 0x00AA, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00AF,
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7,
    0x00B8, 0x00B9, 0x00BA, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0x00BF,
    0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C6, 0x00C7,
    0x00C8, 0x00C9, 0x00CA, 0x00CB, 0x00CC, 0x00CD, 0x00CE, 0x00CF,
    0x011E, 0x00D1, 0x00D2, 0x00D3, 0x00D4, 0x00D5, 0x00D6, 0x00D7,
    0x00D8, 0x00D9, 0x00DA, 0x00DB, 0x00DC, 0x0130, 0x015E, 0x00DF,
    0x00E0, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7,
    0x00E8, 0x00E9, 0x00EA, 0x00EB, 0x00EC, 0x00ED, 0x00EE, 0x00EF,
    0x011F, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00F5, 0x00F6, 0x00F7,
    0x00F8, 0x00F9, 0x00FA, 0x00FB, 0x00FC, 0x0131, 0x015F, 0x00FF,
];

/// Conversion single-byte -> unicode for `CP_1254`
pub fn forward_cp_1254(input: &[u8]) -> ForwardResult {
    let c = input[0];
    match c {
        // CL, GL
        ..=0x7f => (1, Some(c as u32)),
        // CR, GR
        _ => match CP_1254[(c - 0x80) as usize] {
            0xFFFD => (1, None),
            c => (1, Some(c as u32)),
        },
    }
}

/// Conversion unicode->single-byte for `CP_1254`
pub fn backward_cp_1254(output: &mut [u8], code: u32) -> BackwardResult {
    match code {
        // Invalid
        0xFFFD | 0x10000.. => None,
        // CL, GL
        ..=0x80 => {
            output[0] = code as u8;
            Some(1)
        },
        // CR, GR
        _ => CP_1254.iter()
            .position(|&c| c as u32 == code)
            .map(|index| {
                output[0] = (index + 0x80) as u8;
                1
            }),
    }
}