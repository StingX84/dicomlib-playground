#![warn(missing_docs)]

//! Unescape the given string.
//! This is the opposite operation of [`std::ascii::escape_default`].

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

// cSpell:ignore Unescaper unescaping uffff

/// Unescaper's `Error`.
#[allow(missing_docs)]
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug)]
pub enum Error {
    IncompleteStr {
        pos: usize,
    },
    InvalidChar {
        char: char,
        pos: usize,
    },
    ParseInt {
        pos: usize,
        source: ::std::num::ParseIntError,
    },
    NotAllowedChar {
        char: char,
        pos: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompleteStr { pos } => write!(f, "incomplete str, break at {pos}"),
            Self::InvalidChar { char, pos } => write!(f, "invalid char, {char:?} break at {pos}"),
            Self::ParseInt { pos, .. } => write!(f, "parse int error, break at {pos}"),
            Self::NotAllowedChar { char, pos } => {
                write!(f, "not allowed char, {char:?} break at {pos}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseInt { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Unescaper struct which holding the chars cache for unescaping.
#[derive(Debug)]
pub struct Unescaper {
    /// [`str`] cache, in reverse order.
    pub chars: Vec<char>,
    pub validator: Option<fn(char) -> bool>,
}

impl Unescaper {
    /// Build a new [`Unescaper`] from the given [`str`].
    pub fn new(s: &str, v: Option<fn(char) -> bool>) -> Self {
        Self {
            chars: s.chars().rev().collect(),
            validator: v,
        }
    }

    /// Unescape the given [`str`].
    pub fn unescape(&mut self) -> Result<String> {
        let chars_count = self.chars.len();
        let offset = |mut e, remaining_count| {
            let (Error::IncompleteStr { pos }
            | Error::InvalidChar { pos, .. }
            | Error::ParseInt { pos, .. }
            | Error::NotAllowedChar { pos, .. }) = &mut e;
            *pos += chars_count - remaining_count - 1;
            e
        };
        let mut unescaped = String::new();

        while let Some(c) = self.chars.pop() {
            let current_pos = chars_count - self.chars.len() - 1;
            let c = if c != '\\' {
                Ok(c)
            } else {
                let c = self.chars.pop().ok_or(Error::IncompleteStr { pos: current_pos })?;
                let c = match c {
                    'b' => '\u{0008}',
                    'f' => '\u{000c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\'' => '\'',
                    '\"' => '\"',
                    '\\' => '\\',
                    'u' => self
                        .unescape_unicode_internal()
                        .map_err(|e| offset(e, self.chars.len()))?,
                    'x' => self.unescape_byte_internal().map_err(|e| offset(e, self.chars.len()))?,
                    _ => self
                        .unescape_octal_internal(c)
                        .map_err(|e| offset(e, self.chars.len()))?,
                };
                Ok(c)
            }?;

            if let Some(v) = self.validator
                && !v(c)
            {
                return Err(Error::NotAllowedChar {
                    pos: current_pos,
                    char: c,
                });
            }
            unescaped.push(c);
        }

        Ok(unescaped)
    }

    fn unescape_unicode_internal(&mut self) -> Result<char> {
        let c = self.chars.pop().ok_or(Error::IncompleteStr { pos: 0 })?;
        let mut unicode = String::new();

        // \u + { + regex(d*) + }
        if c == '{' {
            while let Some(n) = self.chars.pop() {
                if n == '}' {
                    break;
                }

                unicode.push(n);
            }
        }
        // \u + regex(d{4})
        else {
            // [0, 65536), 16^4
            unicode.push(c);

            for i in 0usize..3 {
                let c = self.chars.pop().ok_or(Error::IncompleteStr { pos: i })?;

                unicode.push(c);
            }
        }

        let code = u16::from_str_radix(&unicode, 16).map_err(|source| Error::ParseInt { pos: 0, source })?;
        char::from_u32(code as u32).ok_or_else(|| Error::InvalidChar {
            char: unicode.chars().last().unwrap_or('\0'),
            pos: 0,
        })
    }

    fn unescape_byte_internal(&mut self) -> Result<char> {
        let mut byte = String::new();

        // [0, 256), 16^2
        for i in 0usize..2 {
            let c = self.chars.pop().ok_or(Error::IncompleteStr { pos: i })?;

            byte.push(c);
        }

        Ok(u8::from_str_radix(&byte, 16).map_err(|source| Error::ParseInt { pos: 0, source })? as char)
    }

    fn unescape_octal_internal(&mut self, c: char) -> Result<char> {
        let mut octal = String::new();
        let mut try_push_next = |octal: &mut String| {
            if let Some(c) = self
                .chars
                .last()
                .cloned()
                .filter(|c| c.is_digit(8))
                .and_then(|_| self.chars.pop())
            {
                octal.push(c);
            }
        };

        match c {
            // decimal [0, 256) == octal [0, 400)
            // 0 <= first digit < 4
            // \ + regex(d{1,3})
            '0' | '1' | '2' | '3' => {
                octal.push(c);

                (0..2).for_each(|_| try_push_next(&mut octal));
            }
            // \ + regex(d{1,2})
            '4' | '5' | '6' | '7' => {
                octal.push(c);

                try_push_next(&mut octal);
            }
            _ => {
                return Err(Error::InvalidChar { char: c, pos: 0 });
            }
        }

        Ok(u8::from_str_radix(&octal, 8).map_err(|source| Error::ParseInt { pos: 0, source })? as char)
    }
}

/// Unescape the given [`str`].
pub fn unescape(s: &str) -> Result<String> {
    Unescaper::new(s, None).unescape()
}

pub fn unescape_with_validator(s: &str, v: fn(char) -> bool) -> Result<String> {
    Unescaper::new(s, Some(v)).unescape()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! unescape_assert_eq {
        ($l:expr, $r:expr) => {
            assert_eq!(unescape($l).unwrap(), $r);
        };
    }

    macro_rules! unescape_assert_err {
        ($l:expr, $r:expr) => {
            assert_eq!(unescape($l).unwrap_err(), $r);
        };
    }

    macro_rules! unescape_assert_err_str {
        ($s:expr, $e:expr) => {{
            let e = unescape($s).unwrap_err();

            assert_eq!(e.to_string(), $e);
        }};
    }

    #[test]
    fn error() {
        unescape_assert_err!(r"\", Error::IncompleteStr { pos: 0usize });
        unescape_assert_err!(r"\0\", Error::IncompleteStr { pos: 2usize });

        unescape_assert_err!(r"\{}", Error::InvalidChar { char: '{', pos: 1 });
        unescape_assert_err!(r"\0\{}", Error::InvalidChar { char: '{', pos: 3 });

        unescape_assert_err_str!(r"\u{g}", "parse int error, break at 4");
        unescape_assert_err_str!(r"\0\u{g}", "parse int error, break at 6");
    }

    #[test]
    fn unescape_unicode() {
        unescape_assert_eq!(r"\u0000", "\0");
        unescape_assert_eq!(r"\u0009", "\t");
        unescape_assert_eq!(r"\u000a", "\n");
        unescape_assert_eq!(r"\uffff", "\u{ffff}");
        unescape_assert_eq!(r"\u0000XavierJane", "\0XavierJane");

        unescape_assert_eq!(r"\u{0}", "\0");
        unescape_assert_eq!(r"\u{9}", "\t");
        unescape_assert_eq!(r"\u{a}", "\n");
        unescape_assert_eq!(r"\u{ffff}", "\u{ffff}");
        unescape_assert_eq!(r"\u{0}XavierJane", "\0XavierJane");
    }

    #[test]
    fn unescape_byte() {
        unescape_assert_eq!(r"\x00", "\x00");
        unescape_assert_eq!(r"\x09", "\t");
        unescape_assert_eq!(r"\x0a", "\n");
        unescape_assert_eq!(r"\x7f", "\x7f");
        unescape_assert_eq!(r"\x00XavierJane", "\x00XavierJane");
    }

    #[test]
    fn unescape_octal() {
        unescape_assert_eq!(r"\0", "\0");
        unescape_assert_eq!(r"\11", "\t");
        unescape_assert_eq!(r"\12", "\n");
        unescape_assert_eq!(r"\377", "\u{00ff}");
        unescape_assert_eq!(r"\0XavierJane", "\0XavierJane");
    }
}
