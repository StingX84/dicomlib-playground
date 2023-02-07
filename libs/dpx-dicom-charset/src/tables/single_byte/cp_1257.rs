#![cfg_attr(rustfmt, rustfmt_skip)]
//! Tables and function for `CP_1257`

use crate::tables::{BackwardResult, ForwardResult};
// cSpell:disable

// Do not edit! This file was autogenerated with `gen_single_byte_tables.py`
// utility on 2023-02-06 by "stingx" on "DESKTOP-2IQN19A".

/// Code Table MS Baltic (non-standard `cp1257`)
static CP_1257: [u16; 128] = [
    0x20AC, 0xFFFD, 0x201A, 0xFFFD, 0x201E, 0x2026, 0x2020, 0x2021,
    0xFFFD, 0x2030, 0xFFFD, 0x2039, 0xFFFD, 0x00A8, 0x02C7, 0x00B8,
    0xFFFD, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0xFFFD, 0x2122, 0xFFFD, 0x203A, 0xFFFD, 0x00AF, 0x02DB, 0xFFFD,
    0x00A0, 0xFFFD, 0x00A2, 0x00A3, 0x00A4, 0xFFFD, 0x00A6, 0x00A7,
    0x00D8, 0x00A9, 0x0156, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00C6,
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7,
    0x00F8, 0x00B9, 0x0157, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0x00E6,
    0x0104, 0x012E, 0x0100, 0x0106, 0x00C4, 0x00C5, 0x0118, 0x0112,
    0x010C, 0x00C9, 0x0179, 0x0116, 0x0122, 0x0136, 0x012A, 0x013B,
    0x0160, 0x0143, 0x0145, 0x00D3, 0x014C, 0x00D5, 0x00D6, 0x00D7,
    0x0172, 0x0141, 0x015A, 0x016A, 0x00DC, 0x017B, 0x017D, 0x00DF,
    0x0105, 0x012F, 0x0101, 0x0107, 0x00E4, 0x00E5, 0x0119, 0x0113,
    0x010D, 0x00E9, 0x017A, 0x0117, 0x0123, 0x0137, 0x012B, 0x013C,
    0x0161, 0x0144, 0x0146, 0x00F3, 0x014D, 0x00F5, 0x00F6, 0x00F7,
    0x0173, 0x0142, 0x015B, 0x016B, 0x00FC, 0x017C, 0x017E, 0x02D9,
];

/// Conversion single-byte -> unicode for `CP_1257`
pub fn forward_cp_1257(input: &[u8]) -> ForwardResult {
    let c = input[0];
    match c {
        // CL, GL
        ..=0x7f => (1, Some(c as u32)),
        // CR, GR
        _ => match CP_1257[(c - 0x80) as usize] {
            0xFFFD => (1, None),
            c => (1, Some(c as u32)),
        },
    }
}

/// Conversion unicode->single-byte for `CP_1257`
pub fn backward_cp_1257(output: &mut [u8], code: u32) -> BackwardResult {
    match code {
        // Invalid
        0xFFFD | 0x10000.. => None,
        // CL, GL
        ..=0x80 => {
            output[0] = code as u8;
            Some(1)
        },
        // CR, GR
        _ => CP_1257.iter()
            .position(|&c| c as u32 == code)
            .map(|index| {
                output[0] = (index + 0x80) as u8;
                1
            }),
    }
}