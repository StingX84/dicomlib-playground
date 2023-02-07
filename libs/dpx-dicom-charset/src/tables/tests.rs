#![cfg_attr(rustfmt, rustfmt_skip)]

use crate::tables::constants::*;
use crate::tables::{single_byte::*, multi_byte::*, PfnBackward, PfnForward};
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;


pub fn read_test_data_file(file_name: &str) -> Vec<(usize, u32, u32)> {
    let file_name: PathBuf = [env!("CARGO_MANIFEST_DIR"), "test_files", file_name]
        .iter()
        .collect();

    let file = File::open(&file_name).expect("Test file exists");
    let mut reader = std::io::BufReader::new(file);

    let mut line = String::new();
    let mut rv = Vec::<(usize, u32, u32)>::new();

    while let Ok(size) = reader.read_line(&mut line) {
        if size == 0 {
            break;
        }
        let l = line.trim();
        if l.is_empty() || l.starts_with("#") {
            line.clear();
            continue;
        }

        let (first, second) = l.split_once("\t").expect("Two values separated by tab");

        assert!(first.starts_with("0x"));
        let length = (first.len() - 2) / 2;
        let first_u32 = u32::from_str_radix(first.trim_start_matches("0x"), 16).expect("Hex value");
        let second_u32 = u32::from_str_radix(second.trim_start_matches("0x"), 16).expect("Hex value");
        rv.push((length, first_u32, second_u32));

        line.clear();
    }
    rv
}

fn to_byte_string(bytes: &[u8]) -> String {
    fn hex(b: u8) -> [u8; 2] {
        const HEX: &[u8] = b"0123456789ABCDEF";
        [HEX[(b >> 4) as usize], HEX[(b & 0xF) as usize]]
    }
    let mut rv = Vec::<u8>::with_capacity((bytes.len() * 4 + 3).next_power_of_two());
    rv.extend_from_slice(b"b\"");
    for &b in bytes.iter() {
        rv.extend_from_slice(b"\\x");
        rv.extend_from_slice(&hex(b));
    }
    rv.push(b'"');
    unsafe { String::from_utf8_unchecked(rv) }
}

struct Dbg<T>(T);
impl std::fmt::Display for Dbg<u32> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:>04X}", self.0)
    }
}
impl std::fmt::Display for Dbg<&[u8]> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(to_byte_string(self.0).as_str())
    }
}
impl std::fmt::Display for Dbg<(u8, Option<u32>)> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            (len, None) => write!(f, "({}, None)", len),
            (len, Some(code)) => write!(f, "({}, Some(0x{:>04x}))", len, code),
        }
    }
}
impl std::fmt::Display for Dbg<Option<u8>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => f.write_str("None"),
            Some(len) => write!(f, "Some({})", len),
        }
    }
}

macro_rules! assert_forward {

    ($forward:expr, $encoded:expr, $expected:expr, $context:expr) => {{
        let l = $forward($encoded);
        let r: (u8, Option<u32>) = $expected;
        if l != $expected {
            panic!(
                "Assertion failed:\nforward({}) == {}\nActually got: {}\nContext: {}",
                Dbg($encoded), Dbg(r), Dbg(l), $context
            );
        }
    }};
}
macro_rules! assert_backward {
    ($backward:expr, $code:expr, None, $context:expr) => {{
        let mut encode_result = [0u8; 4];
        let l = $backward(&mut encode_result, $code);
        if let Some(len) = l {
            let result = &encode_result[..len as usize];
            panic!(
                "Assertion failed:\nbackward({}) == None\nActually got: {}\nContext: {}",
                Dbg($code), Dbg(result), $context
            );
        }
    }};
    ($backward:expr, $code:expr, $encoded:expr, $context:expr) => {{
        let mut encode_result = [0u8; 4];
        let result = $backward(&mut encode_result, $code);
        match result {
            None => panic!(
                "Assertion failed:\nbackward({}) == {}\nActually got: None\nContext: {}",
                Dbg($code), Dbg($encoded), $context
            ),
            Some(len) => {
                let result = &encode_result[..len as usize];
                if result != $encoded {
                    panic!(
                        "Assertion failed:\nbackward({}) == {}\nActually got: {}\nContext: {}",
                        Dbg($code), Dbg($encoded), Dbg(result), $context
                    );
                }
            },
        };
    }};
}

pub fn run(file_name: &str, forward: PfnForward, backward: PfnBackward) {
    let data = read_test_data_file(file_name);
    for &(size, pointer, code) in data.iter() {
        let encoded = &pointer.to_be_bytes()[(4 - size)..];

        if code == CODE_INVALID as u32 {
            assert_forward!(forward, encoded, (size as u8, None), file_name);
        } else {
            assert_forward!(forward, encoded, (size as u8, Some(code)), file_name);
            assert_backward!(backward, code, encoded, file_name);
        }
    }
}

pub fn run_single_byte(file_name: &str, forward: PfnForward, backward: PfnBackward) {
    run(file_name, forward, backward);
    // Overflow and invalid "code" should not work
    for code in [0xFFFD, 0x10000, 0xFFFFFFFF] {
        assert_backward!(backward, code, None, file_name);
    }
}

#[test]
fn all_characters_supported_single_byte_non_standard() {
    run_single_byte("cp_1250.txt", forward_cp_1250, backward_cp_1250);
    run_single_byte("cp_1251.txt", forward_cp_1251, backward_cp_1251);
    run_single_byte("cp_1252.txt", forward_cp_1252, backward_cp_1252);
    run_single_byte("cp_1253.txt", forward_cp_1253, backward_cp_1253);
    run_single_byte("cp_1254.txt", forward_cp_1254, backward_cp_1254);
    run_single_byte("cp_1255.txt", forward_cp_1255, backward_cp_1255);
    run_single_byte("cp_1256.txt", forward_cp_1256, backward_cp_1256);
    run_single_byte("cp_1257.txt", forward_cp_1257, backward_cp_1257);
    run_single_byte("cp_1258.txt", forward_cp_1258, backward_cp_1258);
    run_single_byte("cp_866.txt", forward_cp_866, backward_cp_866);
    run_single_byte("koi8_r.txt", forward_koi8_r, backward_koi8_r);
}

#[test]
fn all_characters_supported_single_byte_iso2022_compatible() {
    // g0
    run_single_byte("iso_ir_6.txt", forward_g0_iso_ir_6, backward_g0_iso_ir_6);
    run_single_byte("iso_ir_14.txt", forward_g0_iso_ir_14, backward_g0_iso_ir_14);
    // g1
    run_single_byte("iso_ir_13.txt", forward_g1_iso_ir_13, backward_g1_iso_ir_13);
    run_single_byte("iso_ir_101.txt", forward_g1_iso_ir_101, backward_g1_iso_ir_101);
    run_single_byte("iso_ir_109.txt", forward_g1_iso_ir_109, backward_g1_iso_ir_109);
    run_single_byte("iso_ir_110.txt", forward_g1_iso_ir_110, backward_g1_iso_ir_110);
    run_single_byte("iso_ir_126.txt", forward_g1_iso_ir_126, backward_g1_iso_ir_126);
    run_single_byte("iso_ir_127.txt", forward_g1_iso_ir_127, backward_g1_iso_ir_127);
    run_single_byte("iso_ir_138.txt", forward_g1_iso_ir_138, backward_g1_iso_ir_138);
    run_single_byte("iso_ir_144.txt", forward_g1_iso_ir_144, backward_g1_iso_ir_144);
    run_single_byte("iso_ir_166.txt", forward_g1_iso_ir_166, backward_g1_iso_ir_166);
    run_single_byte("iso_ir_203.txt", forward_g1_iso_ir_203, backward_g1_iso_ir_203);
    run_single_byte("iso_ir_227.txt", forward_g1_iso_ir_227, backward_g1_iso_ir_227);
    run_single_byte("iso_ir_234.txt", forward_g1_iso_ir_234, backward_g1_iso_ir_234);
}

#[test]
fn all_characters_supported_multi_byte_iso2022_compatible() {
    run("jisx0208.txt", forward_g0_jisx0208, backward_g0_jisx0208);
    run("jisx0212.txt", forward_g0_jisx0212, backward_g0_jisx0212);
    run("ksx1001.txt", forward_g1_ksx1001, backward_g1_ksx1001);
    run("gb2312.txt", forward_g1_gb2312, backward_g1_gb2312);
}

#[test]
fn all_characters_supported_multi_byte_non_iso2022() {
    run("gb18030.txt", forward_gb18030, backward_gb18030);
}

#[test]
fn invalid_always_fails() {
    for byte in 0x00_u8..=0xFF {
        assert!(matches!(forward_invalid(&[byte]), (1, None)))
    }
    let mut dummy = [0x00_u8; 1];
    for wc in [0x00_u32, 0x20, 0x55, 0x80, 0xF0, 0xFF, 0x100, 0xFFFF] {
        assert!(matches!(backward_invalid(&mut dummy, wc), None))
    }
}

#[test]
fn identity_works_as_expected() {
    for byte in 0x00_u8..=0xFF {
        assert_eq!(forward_identity(&[byte]), (1, Some(byte as u32)));
    }
    let mut buffer = [0x00_u8; 1];
    for wc in '\u{00}'..='\u{FF}' {
        assert_eq!(backward_identity(&mut buffer, wc as u32), Some(1));
        assert_eq!(buffer[0] as u32, wc as u32);
    }
    assert_eq!(backward_identity(&mut buffer, 0x100), None);
}
