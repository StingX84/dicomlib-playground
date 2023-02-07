use crate::{
    ascii::{try_decode_ascii, try_encode_ascii, StringExt},
    tables::constants::*,
    term::CodecType,
    Codec, Term,
};
use std::borrow::Cow;

pub fn decode<'a>(bytes: &'a [u8], codec: &Codec) -> Cow<'a, str> {
    let term = codec.terms().first().copied().unwrap_or(Term::Unknown);
    let term_meta = term.meta();

    if let Some(rv) = try_decode_ascii(bytes, term_meta.is_ascii_compatible) {
        return rv;
    }

    let CodecType::NonIso2022(forward, _) = term_meta.mode
        else {
            panic!("Bug: unexpected term mode");
        };

    let mut rv = String::with_capacity(bytes.len().next_power_of_two());
    let mut input = bytes;

    while !input.is_empty() {
        let (consumed, code_point) = forward(input);
        match code_point {
            None => {
                let bad_input = &input[..consumed as usize];
                rv.push_str((codec.config.replacement_character_fn.0)(bad_input).as_ref());
            }
            Some(code_point) => {
                rv.push_u32(code_point);
            }
        };
        input = &input[consumed as usize..];
    }

    rv.into()
}

pub fn encode<'a>(string: &'a str, codec: &Codec) -> Cow<'a, [u8]> {
    let term = codec.terms().first().copied().unwrap_or(Term::Unknown);
    let term_meta = term.meta();

    if let Some(rv) = try_encode_ascii(string, term_meta.is_ascii_compatible) {
        return rv;
    }

    let CodecType::NonIso2022(_, backward) = term_meta.mode
        else {
            panic!("Bug: unexpected term mode");
        };

    let mut rv = Vec::<u8>::with_capacity(string.len().next_power_of_two());
    // None of our codecs could produce more than 4 chars, but to be extra safe for
    // possible future "extra" codecs, make it 16.
    let mut buffer = [0u8; 16];

    for code_point in string.chars() {
        match backward(&mut buffer, code_point as u32) {
            None => {
                if let Some(written) = backward(&mut buffer, CODE_ASCII_REPLACEMENT as u32) {
                    rv.extend_from_slice(&buffer[..written as usize]);
                }
                // ignore failure if "replacement" fails
            }
            Some(written) => {
                rv.extend_from_slice(&buffer[..written as usize]);
            }
        };
    }

    rv.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use Term::*;

    macro_rules! codec {
        ($term:expr) => {{
            &Codec::from_term_list(&[$term], Config::default())
        }};
    }

    #[test]
    #[should_panic]
    fn invalid_term_type_for_decode() {
        decode(b"\x80", codec!(IsoIr6));
    }

    #[test]
    #[should_panic]
    fn invalid_term_type_for_encode() {
        encode("\u{0080}", codec!(IsoIr6));
    }

    #[test]
    fn ascii_handled_correctly() {
        // Currently, all the non ISO-2022 encodings are ASCII compatible.
        assert!(matches!(encode("ABC", codec!(Gbk)), Cow::Borrowed(_)));
        assert!(matches!(encode("\u{80}", codec!(Gbk)), Cow::Owned(_)));
        assert!(matches!(decode(b"ABC", codec!(Gbk)), Cow::Borrowed(_)));
        assert!(matches!(decode(b"\x80", codec!(Gbk)), Cow::Owned(_)));
    }

    #[test]
    fn can_process_single_byte() {
        assert_eq!(
            encode("а\n区\t", codec!(NonDicomCp1251)).as_ref(),
            b"\xE0\n?\t"
        );
        assert_eq!(
            decode(b"\xE0\n\x98\t", codec!(NonDicomCp1251)).as_ref(),
            "а\n�\t"
        );
    }

    #[test]
    fn can_process_multi_byte() {
        assert_eq!(
            encode("а\n区\t", codec!(Gbk)).as_ref(),
            b"\xA7\xD1\x0A\xC7\xF8\x09"
        );
        assert_eq!(
            decode(b"\xA7\xD1\x0A\xC7\xF8\x09", codec!(Gbk)).as_ref(),
            "а\n区\t"
        );
    }
}

#[cfg(test)]
mod gb18030_tests {
    use super::*;
    use crate::Config;

    //cSpell:ignore fffd

    fn decode_gb18030(bytes: &[u8], expect: &str) {
        let codec = Codec::from_term_list(&[Term::Gb18030], Config::default());
        assert_eq!(decode(bytes, &codec).as_ref(), expect);
    }

    fn encode_gb18030(string: &str, expect: &[u8]) {
        let codec = Codec::from_term_list(&[Term::Gb18030], Config::default());
        assert_eq!(encode(string, &codec).as_ref(), expect);
    }

    fn encode_gbk(string: &str, expect: &[u8]) {
        let codec = Codec::from_term_list(&[Term::Gbk], Config::default());
        assert_eq!(encode(string, &codec).as_ref(), expect);
    }

    // Copied as-is from https://github.com/hsivonen/encoding_rs/blob/master/src/gb18030.rs
    #[test]
    fn gb18030_decode() {
        // Empty
        decode_gb18030(b"", &"");

        // ASCII
        decode_gb18030(b"\x61\x62", "\u{0061}\u{0062}");

        // euro
        decode_gb18030(b"\x80", "\u{20AC}");
        decode_gb18030(b"\xA2\xE3", "\u{20AC}");

        // two bytes
        decode_gb18030(b"\x81\x40", "\u{4E02}");
        decode_gb18030(b"\x81\x7E", "\u{4E8A}");
        decode_gb18030(b"\x81\x7F", "\u{FFFD}\u{007F}");
        decode_gb18030(b"\x81\x80", "\u{4E90}");
        decode_gb18030(b"\x81\xFE", "\u{4FA2}");
        decode_gb18030(b"\xFE\x40", "\u{FA0C}");
        decode_gb18030(b"\xFE\x7E", "\u{E843}");
        decode_gb18030(b"\xFE\x7F", "\u{FFFD}\u{007F}");
        decode_gb18030(b"\xFE\x80", "\u{4723}");
        decode_gb18030(b"\xFE\xFE", "\u{E4C5}");

        // The difference from the original GB18030
        decode_gb18030(b"\xA3\xA0", "\u{3000}");
        decode_gb18030(b"\xA1\xA1", "\u{3000}");

        // // 0xFF
        decode_gb18030(b"\xFF\x40", "\u{FFFD}\u{0040}");
        decode_gb18030(b"\xE3\xFF\x9A\x33", "\u{FFFD}\u{FFFD}"); // not \u{FFFD}\u{FFFD}\u{0033} !
        decode_gb18030(b"\xFF\x32\x9A\x33", "\u{FFFD}\u{0032}\u{FFFD}"); // not \u{FFFD}\u{0032}\u{FFFD}\u{0033} !
        decode_gb18030(b"\xFF\x40\x00", "\u{FFFD}\u{0040}\u{0000}");
        decode_gb18030(b"\xE3\xFF\x9A\x33\x00", "\u{FFFD}\u{FFFD}\u{0033}\u{0000}");
        decode_gb18030(
            b"\xFF\x32\x9A\x33\x00",
            "\u{FFFD}\u{0032}\u{FFFD}\u{0033}\u{0000}",
        );

        // Four bytes
        decode_gb18030(b"\x81\x30\x81\x30", "\u{0080}");
        decode_gb18030(b"\x81\x35\xF4\x37", "\u{E7C7}");
        decode_gb18030(b"\x81\x37\xA3\x30", "\u{2603}");
        decode_gb18030(b"\x94\x39\xDA\x33", "\u{1F4A9}");
        decode_gb18030(b"\xE3\x32\x9A\x35", "\u{10FFFF}");
        decode_gb18030(b"\xE3\x32\x9A\x36\x81\x30", "\u{FFFD}\u{FFFD}");
        decode_gb18030(b"\xE3\x32\x9A\x36\x81\x40", "\u{FFFD}\u{4E02}");
        decode_gb18030(b"\xE3\x32\x9A", "\u{FFFD}"); // not \u{FFFD}\u{0032}\u{FFFD} !
        decode_gb18030(b"\xE3\x32\x9A\x00", "\u{FFFD}\u{0032}\u{FFFD}\u{0000}");
    }

    // Copied as-is from https://github.com/hsivonen/encoding_rs/blob/master/src/gb18030.rs
    #[test]
    fn gb18030_encode() {
        // Empty
        encode_gb18030("", b"");

        // ASCII
        encode_gb18030("\u{0061}\u{0062}", b"\x61\x62");

        // euro
        encode_gb18030("\u{20AC}", b"\xA2\xE3");

        // two bytes
        encode_gb18030("\u{4E02}", b"\x81\x40");
        encode_gb18030("\u{4E8A}", b"\x81\x7E");
        if !cfg!(miri) {
            // Miri is too slow
            encode_gb18030("\u{4E90}", b"\x81\x80");
            encode_gb18030("\u{4FA2}", b"\x81\xFE");
            encode_gb18030("\u{FA0C}", b"\xFE\x40");
            encode_gb18030("\u{E843}", b"\xFE\x7E");
            encode_gb18030("\u{4723}", b"\xFE\x80");
            encode_gb18030("\u{E4C5}", b"\xFE\xFE");
        }

        // The difference from the original GB18030
        encode_gb18030("\u{E5E5}", b"?"); // encoding_rs specific &#58853;
        encode_gb18030("\u{3000}", b"\xA1\xA1");

        // Four bytes
        encode_gb18030("\u{0080}", b"\x81\x30\x81\x30");
        encode_gb18030("\u{E7C7}", b"\x81\x35\xF4\x37");
        if !cfg!(miri) {
            // Miri is too slow
            encode_gb18030("\u{2603}", b"\x81\x37\xA3\x30");
            encode_gb18030("\u{1F4A9}", b"\x94\x39\xDA\x33");
            encode_gb18030("\u{10FFFF}", b"\xE3\x32\x9A\x35");
        }

        // Edge cases
        encode_gb18030("\u{00F7}", b"\xA1\xC2");
    }

    // Copied as-is from https://github.com/hsivonen/encoding_rs/blob/master/src/gb18030.rs
    #[test]
    fn gbk_encode() {
        // Empty
        encode_gbk("", b"");

        // ASCII
        encode_gbk("\u{0061}\u{0062}", b"\x61\x62");

        // euro
        encode_gbk("\u{20AC}", b"\x80");

        // two bytes
        encode_gbk("\u{4E02}", b"\x81\x40");
        encode_gbk("\u{4E8A}", b"\x81\x7E");
        if !cfg!(miri) {
            // Miri is too slow
            encode_gbk("\u{4E90}", b"\x81\x80");
            encode_gbk("\u{4FA2}", b"\x81\xFE");
            encode_gbk("\u{FA0C}", b"\xFE\x40");
            encode_gbk("\u{E843}", b"\xFE\x7E");
            encode_gbk("\u{4723}", b"\xFE\x80");
            encode_gbk("\u{E4C5}", b"\xFE\xFE");
        }

        // The difference from the original gb18030
        encode_gbk("\u{E5E5}", b"?"); // encoding_rs specific &#58853;
        encode_gbk("\u{3000}", b"\xA1\xA1");

        // Four bytes
        encode_gbk("\u{0080}", b"?"); // encoding_rs specific &#128;
        encode_gbk("\u{E7C7}", b"?"); // encoding_rs specific &#59335;
        if !cfg!(miri) {
            // Miri is too slow
            encode_gbk("\u{2603}", b"?"); // encoding_rs specific &#9731;
            encode_gbk("\u{1F4A9}", b"?"); // encoding_rs specific &#128169;
            encode_gbk("\u{10FFFF}", b"?"); // encoding_rs specific &#1114111;
        }

        // Edge cases
        encode_gbk("\u{00F7}", b"\xA1\xC2");
    }
}
