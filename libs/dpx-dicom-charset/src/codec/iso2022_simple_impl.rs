use crate::{
    ascii::{try_decode_ascii, try_encode_ascii, StringExt},
    tables::{constants::*, Region, Table, TableKind, TABLE_G1_ALWAYS_IDENTITY},
    term::CodecType,
    Codec, Term,
};
use std::borrow::Cow;

pub fn get_tables(term: Term, codec: &Codec) -> (&'static Table, &'static Table) {
    fn int_get_tables(term: Term, codec: &Codec) -> Option<(&'static Table, &'static Table)> {
        let mut tables = match term.meta().mode {
            CodecType::Iso2022NoExtensions(extended_version) => {
                let CodecType::Iso2022WithExtensions(g0_table, g1_table) = extended_version.meta().mode
                    else {
                        return None;
                    };
                (g0_table, g1_table)
            }
            CodecType::Iso2022WithExtensions(g0_table, g1_table) => (g0_table, g1_table),
            _ => {
                return None;
            }
        };
        if codec.config().use_modern_code_page {
            if let Some(modern) = tables.0.modern {
                tables.0 = modern;
            }
            if let Some(modern) = tables.1.modern {
                tables.1 = modern;
            }
        }
        Some(tables)
    }

    let mut tables = int_get_tables(term, codec).expect("Bug: non ISO 2022 compatible Term");

    if matches!(term, Term::IsoIr6 | Term::Iso2022Ir6) && codec.terms.len() == 1 {
        debug_assert!(tables.1.kind == TableKind::Unassigned);

        if let Some(def_tables) = codec
            .config
            .set_g1_for_iso_ir_6
            .and_then(|term| int_get_tables(term, codec))
        {
            tables.1 = def_tables.1;
        } else {
            tables.1 = &TABLE_G1_ALWAYS_IDENTITY;
        }
    }

    debug_assert!(tables.0.region == Region::G0);
    debug_assert!(tables.1.region == Region::G1);

    tables
}

pub fn decode<'a>(bytes: &'a [u8], codec: &Codec) -> Cow<'a, str> {
    debug_assert_eq!(codec.terms.len(), 1);
    let term = codec.terms().first().copied().unwrap();
    let term_meta = term.meta();

    if let Some(rv) = try_decode_ascii(bytes, term_meta.is_ascii_compatible) {
        return rv;
    }

    let (
        &Table {
            forward: forward_g0,
            ..
        },
        &Table {
            forward: forward_g1,
            ..
        },
    ) = get_tables(term, codec);

    let mut rv = String::with_capacity(bytes.len().next_power_of_two());
    let mut input = bytes;

    while let Some(&c) = input.first() {
        let (consumed, code_point) = if c < 0x80 {
            forward_g0(input)
        } else {
            forward_g1(input)
        };
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
    debug_assert_eq!(codec.terms.len(), 1);
    let term = codec.terms().first().copied().unwrap_or(Term::Unknown);
    let term_meta = term.meta();

    if let Some(rv) = try_encode_ascii(string, term_meta.is_ascii_compatible) {
        return rv;
    }

    let (
        &Table {
            backward: backward_g0,
            ..
        },
        &Table {
            backward: backward_g1,
            ..
        },
    ) = get_tables(term, codec);

    let mut rv = Vec::<u8>::with_capacity(string.len().next_power_of_two());
    let mut buffer = [0u8; 4];

    macro_rules! try_g0 {
        ($code_point:expr) => {{
            match backward_g0(&mut buffer, $code_point as u32) {
                Some(written) => {
                    rv.extend_from_slice(&buffer[..written as usize]);
                    true
                }
                None => false,
            }
        }};
    }

    macro_rules! try_g1 {
        ($code_point:expr) => {{
            match backward_g1(&mut buffer, $code_point as u32) {
                Some(written) => {
                    rv.extend_from_slice(&buffer[..written as usize]);
                    true
                }
                None => false,
            }
        }};
    }

    for code_point in string.chars() {
        // Clippy wants us to collapse all this "if" into a single "&& .. && .."
        // which is far less readable and hides our intension.
        #[allow(clippy::collapsible_if)]
        if !try_g0!(code_point) && !try_g1!(code_point) {
            if code_point != CHAR_ASCII_REPLACEMENT {
                if !try_g0!(CHAR_ASCII_REPLACEMENT) {
                    try_g1!(CHAR_ASCII_REPLACEMENT);
                }
            }
        }
    }

    rv.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::*;
    use crate::Config;
    use crate::Term::*;

    macro_rules! codec {
        ( $($term:ident),* $(+ $member:ident = $value:expr)?) => {&{
            #[allow(unused_mut)]
            let mut config = Config::new();
            $(config.$member = $value;)?

            Codec {
                terms: vec![$($term),*],
                config,
                .. Default::default()
            }
        }};
    }

    #[test]
    #[should_panic]
    fn too_few_terms_for_decoder() {
        decode(b"\x80", codec!());
    }

    #[test]
    #[should_panic]
    fn too_many_terms_for_decoder() {
        decode(b"\x80", codec!(Iso2022Ir6, Iso2022Ir100));
    }

    #[test]
    #[should_panic]
    fn unsupported_term_for_decoder() {
        decode(b"\x80", codec!(IsoIr192));
    }

    #[test]
    #[should_panic]
    fn too_few_terms_for_encoder() {
        encode("\u{80}", codec!());
    }

    #[test]
    #[should_panic]
    fn too_many_terms_for_encoder() {
        encode("\u{80}", codec!(Iso2022Ir6, Iso2022Ir100));
    }

    #[test]
    #[should_panic]
    fn unsupported_term_for_encoder() {
        encode("\u{80}", codec!(IsoIr192));
    }

    #[test]
    fn can_use_modern_code_page() {
        let (_, g1) = get_tables(IsoIr126, codec!(IsoIr126 + use_modern_code_page = true));
        assert!(std::ptr::eq(g1, &TABLE_G1_ISO_IR_227));
        let (_, g1) = get_tables(IsoIr126, codec!(IsoIr126 + use_modern_code_page = false));
        assert!(std::ptr::eq(g1, &TABLE_G1_ISO_IR_126));
    }

    #[test]
    fn can_designate_g1_in_iso_ir_6() {
        // Default configuration designates "identity" to ISO_IR 6 single-valued
        let (g0, g1) = get_tables(IsoIr6, codec!(IsoIr6));
        assert!(std::ptr::eq(g0, &TABLE_G0_ISO_IR_6));
        assert!(std::ptr::eq(g1, &TABLE_G1_ALWAYS_IDENTITY));
        // This will not work for multi-valued terms
        let (g0, g1) = get_tables(IsoIr6, codec!(IsoIr6, IsoIr100));
        assert!(std::ptr::eq(g0, &TABLE_G0_ISO_IR_6));
        assert!(std::ptr::eq(g1, &TABLE_G1_ALWAYS_INVALID));
        // We can override identity table with some other table
        let (g0, g1) = get_tables(
            IsoIr6,
            codec!(IsoIr6 + set_g1_for_iso_ir_6 = Some(IsoIr100)),
        );
        assert!(std::ptr::eq(g0, &TABLE_G0_ISO_IR_6));
        assert!(std::ptr::eq(g1, &TABLE_G1_ISO_IR_100));
        // But, some Terms does not specifies G1, so it can remain not designated
        let (g0, g1) = get_tables(
            IsoIr6,
            codec!(IsoIr6 + set_g1_for_iso_ir_6 = Some(Iso2022Ir159)),
        );
        assert!(std::ptr::eq(g0, &TABLE_G0_ISO_IR_6));
        assert!(std::ptr::eq(g1, &TABLE_G1_ALWAYS_INVALID));
        // Overridden term should support ISO-2022. Else, it will take no effect.
        let (g0, g1) = get_tables(
            IsoIr6,
            codec!(IsoIr6 + set_g1_for_iso_ir_6 = Some(IsoIr192)),
        );
        assert!(std::ptr::eq(g0, &TABLE_G0_ISO_IR_6));
        assert!(std::ptr::eq(g1, &TABLE_G1_ALWAYS_IDENTITY));
        let (g0, g1) = get_tables(IsoIr6, codec!(IsoIr6 + set_g1_for_iso_ir_6 = Some(Gb18030)));
        assert!(std::ptr::eq(g0, &TABLE_G0_ISO_IR_6));
        assert!(std::ptr::eq(g1, &TABLE_G1_ALWAYS_IDENTITY));
        let (g0, g1) = get_tables(
            IsoIr6,
            codec!(IsoIr6 + set_g1_for_iso_ir_6 = Some(NonDicomCp1250)),
        );
        assert!(std::ptr::eq(g0, &TABLE_G0_ISO_IR_6));
        assert!(std::ptr::eq(g1, &TABLE_G1_ALWAYS_IDENTITY));
    }


    #[test]
    fn can_borrow_ascii() {
        assert!(matches!(decode(b"ASCII", codec!(IsoIr6)), Cow::Borrowed(x) if x == "ASCII"));
        assert!(matches!(decode(b"\xAA", codec!(IsoIr6)), Cow::Owned(x) if x == "\u{AA}"));
        assert!(matches!(encode("ASCII", codec!(IsoIr6)), Cow::Borrowed(x) if x == b"ASCII"));
        assert!(matches!(encode("\u{AA}", codec!(IsoIr6)), Cow::Owned(x) if x == b"\xAA"));
    }

    #[test]
    fn can_decode() {
        // Note, here we've got an analog to "EUC-CN"
        assert_eq!(
            decode(
                b"1a\n2\x80\n3\xA1\xA1\n4\xA1Z\xA1",
                codec!(IsoIr6 + set_g1_for_iso_ir_6 = Some(Iso2022Ir58))
            ),
            "1a\n2�\n3\u{3000}\n4�Z�"
        );
    }

    #[test]
    fn can_encode() {
        assert_eq!(
            encode(
                "1a\n\u{3000}\n�",
                codec!(IsoIr6 + set_g1_for_iso_ir_6 = Some(Iso2022Ir58))
            )
            .as_ref(),
            b"1a\n\xA1\xA1\n?"
        );
    }
}
