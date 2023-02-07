use super::Codec;
use std::borrow::Cow;

pub fn decode<'a>(bytes: &'a [u8], codec: &Codec) -> Cow<'a, str> {
    let mut result = std::str::from_utf8(bytes);
    if let Ok(result) = result {
        return Cow::Borrowed(result);
    }

    let mut rv = String::with_capacity(bytes.len().next_power_of_two());
    let mut input = bytes;

    loop {
        match result {
            Ok(s) => {
                rv.push_str(s);
                break;
            }
            Err(e) => {
                let (valid, after_valid) = input.split_at(e.valid_up_to());
                // SAFETY: region being interpreted as UTF-8 was already checked by
                // the rust standard library for correctness.
                unsafe { rv.push_str(std::str::from_utf8_unchecked(valid)) }

                if let Some(invalid_sequence_length) = e.error_len() {
                    rv.push_str(
                        (codec.config.replacement_character_fn.0)(
                            &after_valid[..invalid_sequence_length],
                        )
                        .as_ref(),
                    );
                    input = &after_valid[invalid_sequence_length..]
                } else {
                    rv.push_str((codec.config.replacement_character_fn.0)(after_valid).as_ref());
                    break;
                }
            }
        }
        result = std::str::from_utf8(input);
    }

    rv.into()
}

pub fn encode(string: &str) -> Cow<'_, [u8]> {
    return Cow::Borrowed(string.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Config, Term};

    const UNI: &str = "Привет";
    const ASCII: &str = "ASCII\nonly";
    const BAD: &[u8] = b"Bad \xFF UTF-8\xD0";
    const BAD_CORRECTED: &str = "Bad � UTF-8�";

    // cSpell::ignore привет
    #[test]
    fn can_decode_utf_8() {
        let c = Codec::from_term_list(&[Term::IsoIr192], Config::default());
        assert_eq!(decode(UNI.as_bytes(), &c), UNI);
        assert_eq!(decode(BAD, &c), BAD_CORRECTED);

        assert!(matches!(decode(UNI.as_bytes(), &c), Cow::Borrowed(_)));
        assert!(matches!(decode(ASCII.as_bytes(), &c), Cow::Borrowed(_)));
        assert!(matches!(decode(BAD, &c), Cow::Owned(_)));
    }

    #[test]
    fn can_encode_utf_8() {
        assert_eq!(encode(UNI), UNI.as_bytes());
        assert!(matches!(encode(UNI), Cow::Borrowed(_)));
        assert!(matches!(encode(ASCII), Cow::Borrowed(_)));
    }
}
