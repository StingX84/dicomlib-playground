// cSpell::ignore theader tbody

//! # Purpose:
//! Text encoding support for `dpx-dicom` library.
//!
//! # Topics:
//! - [Supported encodings](_doc::supported_encodings)
//! - [Control characters](_doc::control_characters)
//! - [Codec] - main structure of the crate
//!
//! All the compatibility features, that deviates from The Standard requirements
//! may be found in [Config] members documentation.
//!
//! ## Examples:
//! ```
//! use dpx_dicom_charset::{Codec, Config, Context};
//!
//! // Hello (Nǐ hǎo) in chinese with "GB2312" in G1:
//! let codec = Codec::from_specific_character_set("\\ISO 2022 IR 58".as_bytes(), Config::default());
//! assert_eq!(
//!     codec.decode(b"\x1B\x24\x29\x41\xC4\xE3\xBA\xC3", &Context::default()).as_ref(),
//!     "你好"
//!     );
//!
//! // Hello (Privet) in russian with "ISO 8859-5":
//! let codec = Codec::from_specific_character_set("ISO_IR 144".as_bytes(), Config::default());
//! assert_eq!(
//!     codec.encode("Привет", &Context::default()).as_ref(),
//!     b"\xBF\xE0\xD8\xD2\xD5\xE2"
//!     );
//!
//! // Hello (Barev) in armenian with "UTF-8":
//! let codec = Codec::from_specific_character_set("ISO_IR 192".as_bytes(), Config::default());
//! assert_eq!(
//!     codec.encode("Բարեւ", &Context::default()).as_ref(),
//!     b"\xD4\xB2\xD5\xA1\xD6\x80\xD5\xA5\xD6\x82"
//!     );
//! ```
//!
//! ## Features:
//! - `encoding_rs` - Enables dependency on `encoding_rs` crate for additional
//!   encodings support.

/// Documentation topics
pub mod _doc {
    pub mod builtin_terms;
    pub mod special_characters;
    pub mod iso_ir_char_sets;
    pub mod supported_encodings;
}

pub mod ascii;
pub mod char_class;
mod codec;
pub(crate) mod tables;
mod term;

pub use codec::Codec;
pub use codec::Config;
pub use codec::Context;
pub use term::Term;
pub use term::TermKind;
pub use term::TermMatchedWith;
pub use term::TermMeta;
