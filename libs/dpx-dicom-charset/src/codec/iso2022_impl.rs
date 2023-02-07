use super::iso2022_simple_impl::get_tables;
use crate::{
    ascii::{try_decode_ascii_if_has_no_esc_codes, try_encode_ascii, StringExt},
    tables::{constants::*, Region, Table, ISO_TABLES},
    term::CodecType,
    Codec, Context,
};
use std::{borrow::Cow, ptr};

/// Checks if the specific code point should reset encoder or decoder state
///
/// According to [PS3.5 6.1.2.5.3] Requirements:
/// > If within a textual Value a character set other than the one specified in
/// > Value 1 of the Attribute Specific Character Set (0008,0005), or the
/// > Default Character Repertoire if Value 1 is missing, has been invoked, the
/// > character set specified in the Value 1, or the Default Character
/// > Repertoire if Value 1 is missing, shall be active in the following
/// > instances:
/// > - before the end of line (i.e., before the CR and/or LF)
/// > - before the end of a page (i.e., before the FF)
/// > - before any other Control Character other than ESC (e.g., before any TAB)
/// > - before the end of a Data Element Value (e.g., before the 05/12 character
/// >   code that separates multiple textual Data Element Values - 05/12
/// >   corresponds to "\" (BACKSLASH) in the case of default repertoire IR-6 or
/// >   "¥" (YEN SIGN) in the case of IR-14).
/// > - before the "^" and "=" delimiters separating name components and name
/// >   component groups in Data Elements with a VR of PN.
///
/// [PS3.5 6.1.2.5.3]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_6.html#sect_6.1.2.5.3
///     "PS3.5 \"6.1.2.5.3. Requirements\""
pub fn should_reset_tables(c: u8, extra_delimiters: [u8; 3]) -> bool {
    c <= CL_MAX || extra_delimiters.contains(&c)
}

/// Creates an array of extra characters
const fn make_extra_delimiters(is_pn: bool, has_value_separator: bool) -> [u8; 3] {
    if has_value_separator && is_pn {
        [CODE_VALUES_SEPARATOR, b'^', b'=']
    } else if has_value_separator {
        [CODE_VALUES_SEPARATOR, 0, 0]
    } else if is_pn {
        [b'^', b'=', 0]
    } else {
        [0, 0, 0]
    }
}

/// Extract an ESC sequence from the input string.
///
/// Returns `Ok`(esc_sequence_length) if successfully extracted or `None` in
/// case of error
///
/// Escape sequences take the form ESC I [I...] F. The intermediate
/// (I) bytes are from the range 0x20–0x2F, and the final (F) byte is
/// from the range 0x30–0x7E
fn extract_esc_sequence_length(bytes: &[u8]) -> Option<usize> {
    for (index, &c) in bytes.iter().enumerate() {
        match c {
            0x20..=0x2F => continue,
            0x30..=0x7E if index > 0 => return Some(index + 1),
            _ => return None,
        }
    }
    None
}

/// Searches [ISO_TABLES] for a specified ESC sequence limiting
/// the search only in tables supported in the provided `term_list`.
/// Returns Some(table) if found, None otherwise.
fn find_iso_table_by_esc_sequence_within_terms(
    sequence: &[u8],
    codec: &Codec,
) -> Option<&'static Table> {
    ISO_TABLES.iter()
    .find(|&&table| {
        if table.esc != sequence {
            return false;
        }

        // Check if this table used in one of codec's terms.
        codec.terms.iter().any(|term| {
            let tables = match term.meta().mode {
                CodecType::Iso2022NoExtensions(extended_version) => {
                    let CodecType::Iso2022WithExtensions(g0_table, g1_table) = extended_version.meta().mode
                        else {
                            panic!("Bug: Iso2022NoExtensions should point to Iso2022WithExtensions");
                        };
                    (g0_table, g1_table)
                },
                CodecType::Iso2022WithExtensions(g0_table, g1_table) => {
                    (g0_table, g1_table)
                },
                _ => {
                    panic!("Bug: Using ISO2022 codec with non ISO2022 term!");
                },
            };
            ptr::eq(tables.0, table) || ptr::eq(tables.1, table)
        })
    })
    .map(|&table| {
        // Replace with "modern" variant if allowed and available
        if codec.config.use_modern_code_page {
            if let Some(modern) = table.modern {
                return modern;
            }
        }
        table
    })
}

pub fn decode<'a>(bytes: &'a [u8], codec: &Codec, context: &Context) -> Cow<'a, str> {
    debug_assert!(!codec.terms.is_empty());
    let term = codec.terms().first().copied().unwrap();
    let term_meta = term.meta();

    if let Some(rv) = try_decode_ascii_if_has_no_esc_codes(bytes, term_meta.is_ascii_compatible) {
        return rv;
    }

    let (initial_g0, initial_g1) = get_tables(term, codec);

    let mut g0 = initial_g0;
    let mut g1 = initial_g1;

    let mut rv = String::with_capacity(bytes.len().next_power_of_two());
    let mut input = bytes;
    let extra_delimiters = make_extra_delimiters(context.is_pn, context.is_multi_valued);

    while let Some(&c) = input.first() {
        // Extract possible escape sequence
        if c == CODE_ESC {
            let esc_seq_length = match extract_esc_sequence_length(&input[1..]) {
                Some(l) => l,
                None => {
                    rv.push_str((codec.config.replacement_character_fn.0)(&input[..1]).as_ref());
                    input = &input[1..];
                    continue;
                }
            };

            let esc_sequence = &input[1..(esc_seq_length + 1)];

            if let Some(new_iso_table) =
                find_iso_table_by_esc_sequence_within_terms(esc_sequence, codec)
            {
                if new_iso_table.region == Region::G0 {
                    g0 = new_iso_table;
                } else {
                    g1 = new_iso_table;
                }
            } else {
                rv.push_str(
                    (codec.config.replacement_character_fn.0)(&input[..(esc_seq_length + 1)])
                        .as_ref(),
                );
            }
            input = &input[(esc_seq_length + 1)..];
            continue;
        };

        let (consumed, code_point) = if c < 0x80 {
            (g0.forward)(input)
        } else {
            (g1.forward)(input)
        };
        match code_point {
            None => {
                let bad_input = &input[..consumed as usize];
                rv.push_str((codec.config.replacement_character_fn.0)(bad_input).as_ref());
            }
            Some(code_point) => {
                rv.push_u32(code_point);
                if code_point < 0x7f && should_reset_tables(code_point as u8, extra_delimiters) {
                    g0 = initial_g0;
                    g1 = initial_g1;
                }
            }
        };
        input = &input[consumed as usize..];
    }

    rv.into()
}

pub fn encode<'a>(string: &'a str, codec: &Codec, context: &Context) -> Cow<'a, [u8]> {
    debug_assert!(!codec.terms.is_empty());
    let term = codec.terms().first().copied().unwrap();
    let term_meta = term.meta();

    if let Some(rv) = try_encode_ascii(string, term_meta.is_ascii_compatible) {
        return rv;
    }

    let (initial_g0, initial_g1) = get_tables(term, codec);

    let mut g0 = initial_g0;
    let mut g1 = initial_g1;

    let extra_delimiters = make_extra_delimiters(context.is_pn, context.is_multi_valued);
    let mut rv = Vec::<u8>::with_capacity(string.len().next_power_of_two());
    let mut buffer = [0u8; 16];

    macro_rules! emit_esc {
        ($esc:expr) => {{
            if !$esc.is_empty() {
                rv.push(CODE_ESC);
                rv.extend_from_slice($esc);
            }
        }};
    }

    macro_rules! try_g0 {
        ($code_point:expr, $some_g0:expr, $switch_if_success:literal) => {{
            match ($some_g0.backward)(&mut buffer, $code_point as u32) {
                Some(written) => {
                    if $switch_if_success && !ptr::eq($some_g0, g0) {
                        emit_esc!($some_g0.esc);
                        g0 = $some_g0;
                    }
                    rv.extend_from_slice(&buffer[..written as usize]);
                    true
                }
                None => false,
            }
        }};
    }

    macro_rules! try_g1 {
        ($code_point:expr, $some_g1:expr, $switch_if_success:literal) => {{
            match ($some_g1.backward)(&mut buffer, $code_point as u32) {
                Some(written) => {
                    if $switch_if_success && !ptr::eq($some_g1, g1) {
                        emit_esc!($some_g1.esc);
                        g1 = $some_g1;
                    }
                    rv.extend_from_slice(&buffer[..written as usize]);
                    true
                }
                None => false,
            }
        }};
    }

    macro_rules! write_code_point {
        ($code_point:expr) => {{
            // Try to write with current g0 and g1
            if try_g0!($code_point, g0, false) {
                true
            } else if try_g1!($code_point, g1, false) {
                true
            }
            // Before search, try to "revert" to the default
            else if !ptr::eq(g0, initial_g0) && try_g0!($code_point, initial_g0, true) {
                true
            } else if !ptr::eq(g1, initial_g1) && try_g1!($code_point, initial_g1, true) {
                true
            }
            // Search for some code table, that could encode out character
            else {
                codec.terms.iter().any(|&checked_term| {
                    let (new_g0, new_g1) = get_tables(checked_term, codec);

                    (!ptr::eq(new_g0, g0) && try_g0!($code_point, new_g0, true))
                        || (!ptr::eq(new_g1, g1) && try_g1!($code_point, new_g1, true))
                })
            }
        }};
    }

    for code_point in string.chars() {
        if code_point as u32 <= GL_MAX as u32
            && should_reset_tables(code_point as u8, extra_delimiters)
        {
            if !ptr::eq(g0, initial_g0) {
                emit_esc!(initial_g0.esc);
            }
            g0 = initial_g0;

            if !ptr::eq(g1, initial_g1) {
                emit_esc!(initial_g1.esc);
            }
            g1 = initial_g1;
        }

        if !write_code_point!(code_point) && code_point != CHAR_ASCII_REPLACEMENT {
            write_code_point!(CHAR_ASCII_REPLACEMENT);
        }
    }

    // Standard does not require us to reset encoder state at the end of the
    // string, but we must do this if G0 contains 94x94 set, because such string
    // may not be space padded later. Currently, we will reset G0 always.
    if !ptr::eq(g0, initial_g0) {
        emit_esc!(initial_g0.esc);
    }

    rv.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use crate::Context;
    use crate::Term::*;

    macro_rules! codec {
        ( $($term:ident),* $(+ $member:ident = $value:expr)?) => {&{
            #[allow(unused_mut)]
            let mut config = Config::default();
            $(config.$member = $value;)?

            Codec {
                terms: vec![$($term),*],
                config,
                .. Default::default()
            }
        }};
    }
    macro_rules! context {
        ( $($member:ident = $value:expr),*) => {&{
            #[allow(unused_mut)]
            let mut context = Context::default();
            $(context.$member = $value;)*
            context
        }};
    }

    #[test]
    #[should_panic]
    fn too_few_terms_for_decoder() {
        decode(b"\x80", codec!(), context!());
    }

    #[test]
    #[should_panic]
    fn unsupported_term_for_decoder() {
        decode(b"\x80", codec!(IsoIr192), context!());
    }

    #[test]
    #[should_panic]
    fn too_few_terms_for_encoder() {
        encode("\u{80}", codec!(), context!());
    }

    #[test]
    fn esc_code_length_calculated_successfully() {
        for t in ISO_TABLES {
            assert!(!t.esc.is_empty());
            assert!(matches!(extract_esc_sequence_length(t.esc), Some(x) if x == t.esc.len()));
        }
    }

    #[test]
    fn invalid_esc_code_handled_properly() {
        assert_eq!(decode(b"\x1B", codec!(IsoIr6), context!()), "�");
        assert_eq!(decode(b"\x1B\x28", codec!(IsoIr6), context!()), "�\u{28}");
        assert_eq!(decode(b"\x1B\x28\x49", codec!(IsoIr6), context!()), "�");
        assert_eq!(
            decode(b"\x1B\x20\x21\x22\x2E\x7E", codec!(IsoIr6), context!()),
            "�"
        );
        assert_eq!(decode(b"\x1B\x28\x42", codec!(IsoIr6), context!()), "");
    }

    #[test]
    fn can_borrow_ascii() {
        assert!(
            matches!(decode(b"ASCII", codec!(IsoIr6), context!()), Cow::Borrowed(x) if x == "ASCII")
        );
        assert!(
            matches!(decode(b"\xAA", codec!(IsoIr6), context!()), Cow::Owned(x) if x == "\u{AA}")
        );
        assert!(
            matches!(encode("ASCII", codec!(IsoIr6), context!()), Cow::Borrowed(x) if x == b"ASCII")
        );
        assert!(
            matches!(encode("\u{AA}", codec!(IsoIr6), context!()), Cow::Owned(x) if x == b"\xAA")
        );
        // Decoder must not borrow if ESC character presents
        assert!(matches!(decode(b"\x1B", codec!(IsoIr6), context!()), Cow::Owned(x) if x == "�"));
        // Empty strings considered as a valid ASCII
        assert!(matches!(encode("", codec!(IsoIr6), context!()), Cow::Borrowed(x) if x == b""));
        assert!(matches!(decode(b"", codec!(IsoIr6), context!()), Cow::Borrowed(x) if x == ""));
    }

    #[test]
    fn decoder_resets_g0g1_on_delimiters() {
        // "\" and control chars are delimiters and G0/G1 should be reset after them
        assert_eq!(
            decode(
                b"\xC4\x1B\x2D\x4C\xC4\\\xC4\\\x1B\x2D\x4C\xC4\n\xC4",
                codec!(Iso2022Ir6, Iso2022Ir144),
                context!(is_multi_valued = true)
            ),
            "�Ф\\�\\Ф\n�"
        );
        // "=" and "^" are delimiters when VR==PN
        assert_eq!(
            decode(
                b"\xC4\x1B\x2D\x4C\xC4=\xC4\\\x1B\x2D\x4C\xC4^\xC4",
                codec!(Iso2022Ir6, Iso2022Ir144),
                context!(is_pn = true, is_multi_valued = true)
            ),
            "�Ф=�\\Ф^�"
        );
        // When VR has no special meaning for "\" character (i.e. single-valued),
        // this character should not reset G0/G1.
        assert_eq!(
            decode(
                b"\x1B\x2D\x4C\xC4\\\xC4",
                codec!(Iso2022Ir6, Iso2022Ir144),
                context!(is_multi_valued = false)
            ),
            "Ф\\Ф"
        );

        // ISO IR 14 can not encode "\" character, so input byte code "\" will
        // not be interpreted as a value separator and G0/G! wont be reset!
        assert_eq!(
            decode(
                b"\x1B\x28\x4A\x7E\\\x7E\n\x7E",
                codec!(Iso2022Ir6, Iso2022Ir13),
                context!(is_multi_valued = true)
            ),
            "\u{203E}¥\u{203E}\n\u{7E}"
        );
    }

    #[test]
    fn encoder_resets_g0g1_on_delimiters() {
        // "\" and control chars are delimiters and G0/G1 should be reset after them
        assert_eq!(
            encode(
                "Ф \\ Ф\nФ",
                codec!(Iso2022Ir6, Iso2022Ir144),
                context!(is_multi_valued = true)
            )
            .as_ref(),
            b"\x1B\x2D\x4C\xC4 \\ \x1B\x2D\x4C\xC4\n\x1B\x2D\x4C\xC4"
        );
        // "=" and "^" are delimiters when VR==PN
        assert_eq!(
            encode(
                "Ф ^ Ф = Ф",
                codec!(Iso2022Ir6, Iso2022Ir144),
                context!(is_pn = true, is_multi_valued = true)
            )
            .as_ref(),
            b"\x1B\x2D\x4C\xC4 ^ \x1B\x2D\x4C\xC4 = \x1B\x2D\x4C\xC4"
        );
        // When VR has no special meaning for "\" character (i.e. single-valued),
        // this character should not reset G0/G1.
        assert_eq!(
            encode(
                "Ф\\Ф",
                codec!(Iso2022Ir6, Iso2022Ir144),
                context!(is_multi_valued = false)
            )
            .as_ref(),
            b"\x1B\x2D\x4C\xC4\\\xC4"
        );

        // ISO IR 14 can not encode "\" character, so input byte code "\" will
        // not be interpreted as a value separator and G0/G! wont be reset!
        assert_eq!(
            encode(
                "\u{203E}¥\u{203E}\\\u{203E}\u{7E}",
                codec!(Iso2022Ir6, Iso2022Ir13),
                context!(is_multi_valued = true)
            )
            .as_ref(),
            &[
                0x1B, 0x28, 0x4A, // Switch to value 2 G0 (IR 14)
                0x7E, 0x5C, 0x7E, // Overline, Yen sign (but, byte code the same as "\" BACKSLASH), Overline
                0x1B, 0x28, 0x42, // Switch to value 1 G0 (IR 6), because current G0 can not encode "\" character.
                0x5C, // BACKSLASH character in IR 6
                0x1B, 0x28, 0x4A, // Switch to value 2 G0 (IR 14)
                0x7E, // Overline in IR 14
                0x1B, 0x28, 0x42, // Switch to value 1 G0 (IR 6), because current G0 can not encode \u7E
                0x7E, // \u7E char in IR 6
            ]
        );

        // Encoder should switch back to initial G0 at the end if last used G0
        // was 94x94 set. Currently, the encoder always switches back to initial G0.
        assert_eq!(
            encode("", codec!(Iso2022Ir6, Iso2022Ir87), context!()).as_ref(),
            b""
        );

    }
}
