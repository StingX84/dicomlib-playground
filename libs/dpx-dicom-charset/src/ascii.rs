use crate::tables::constants::*;
use std::borrow::Cow;
use std::str;

pub(crate) fn try_decode_ascii(bytes: &[u8], codec_supports_ascii: bool) -> Option<Cow<'_, str>> {
    if bytes.is_empty() || (codec_supports_ascii && bytes.is_ascii()) {
        // SAFETY: empty or ASCII-only text is always a valid UTF-8 sequence.
        Some(Cow::Borrowed(unsafe {
            std::str::from_utf8_unchecked(bytes)
        }))
    } else {
        None
    }
}

pub(crate) fn try_decode_ascii_if_has_no_esc_codes(bytes: &[u8], codec_supports_ascii: bool) -> Option<Cow<'_, str>> {
    if bytes.is_empty() || (codec_supports_ascii && bytes.is_ascii() && !bytes.iter().any(|&c| c == CODE_ESC)) {
        // SAFETY: empty or ASCII-only text is always a valid UTF-8 sequence.
        Some(Cow::Borrowed(unsafe {
            std::str::from_utf8_unchecked(bytes)
        }))
    } else {
        None
    }
}

pub(crate) fn try_encode_ascii(str: &str, codec_supports_ascii: bool) -> Option<Cow<'_, [u8]>> {
    if str.is_empty() || (codec_supports_ascii && str.as_bytes().is_ascii()) {
        Some(Cow::Borrowed(str.as_bytes()))
    } else {
        None
    }
}

pub trait StringExt {
    /// Appends a `code_point` to the string.
    /// If `code_point` is invalid, it is replaced by `?` character.
    fn push_u32(&mut self, code_point: u32);
}

impl StringExt for String {
    #[inline]
    fn push_u32(&mut self, code_point: u32) {
        self.push(core::char::from_u32(code_point).unwrap_or(CHAR_ASCII_REPLACEMENT));
    }
}

pub trait SliceExt {
    #[must_use]
    fn trim_spaces_start(self) -> Self;
    #[must_use]
    fn trim_spaces_end(self) -> Self;
    #[must_use]
    fn trim_spaces(self) -> Self;
}

impl<'a> SliceExt for &'a [u8] {
    fn trim_spaces_start(self: &'a [u8]) -> &'a [u8] {
        let mut rv = self;
        while let Some(&c) = rv.first() {
            if !c.is_ascii_whitespace() {
                break;
            }
            rv = &rv[1..];
        }
        rv
    }

    fn trim_spaces_end(self: &'a [u8]) -> &'a [u8] {
        let mut rv = self;
        let mut len = rv.len();
        while let Some(&c) = rv.last() {
            if !c.is_ascii_whitespace() {
                break;
            }
            len -= 1;
            rv = &rv[..len];
        }
        rv
    }

    fn trim_spaces(self: &'a [u8]) -> &'a [u8] {
        self.trim_spaces_start().trim_spaces_end()
    }
}
