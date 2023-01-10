use snafu::{OptionExt, ResultExt, Snafu};

use std:: {
    io,
    path::PathBuf,
    path::Path,
};

// cSpell:ignore ggggeeee

// Reexports
pub use tagkey_impl::TagKey;
pub use tag_impl::Tag;
pub use dict_impl::{Dictionary, Level};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("TagKey invalid separator (expecting `(gggg,eeee)`, `(gggg:eeee)`, `gggg,eeee`, `gggg:eeee`, `ggggeeee`)"))]
    TagKeyInBracesMissingSeparator,

    #[snafu(display("TagKey contains non-hex characters  (expecting `(gggg,eeee)`, `(gggg:eeee)`, `gggg,eeee`, `gggg:eeee`, `ggggeeee`)"))]
    TagKeyContainsNonHexCharacters{source: std::num::ParseIntError},

    #[snafu(display("invalid TagKey format (expecting `(gggg,eeee)`, `(gggg:eeee)`, `gggg,eeee`, `gggg:eeee`, `ggggeeee`)"))]
    UnrecognizedTagKeyFormat,

    #[snafu(display("Tag Dictionary reader got error opening dictionary file: {}", source))]
    FailedToOpenTagDictionaryFile{file_name: PathBuf, source: io::Error},

    #[snafu(display("Tag Dictionary reader got error reding dictionary file: {}", source))]
    FailedToReadTagDictionaryFile{source: io::Error},
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// Private modules
mod tagkey_impl;
mod tag_impl;
mod dict_impl;
