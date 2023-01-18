use snafu::{OptionExt, ResultExt, Snafu, ensure};

use std:: {
    io,
    path::PathBuf,
    path::Path,
};

// cSpell:ignore ggggeeee

// Public modules

/// Built-in list of all the known DICOM standard Attribute Tags
pub mod dicom;
/// Built-in static list of [Meta] descriptions for standard DICOM Tags.
///
/// This list is automatically registered in [`Dictionary`]
pub mod dicom_meta;
inventory::submit!{dicom_meta::ALL_TAGS_META}

// Built-in list of all DICONDE Attribute Tags not defined by DICOM standard.
pub mod diconde;
/// Built-in static list of [Meta] description for DICON attribute tags not defined by DICOM standard.
///
/// This list is NOT automatically registered in [`Dictionary`].
/// To enable all DICONDE-specific naming, you should manually
/// add this tag to a Dictionary with [add_static_list](Dictionary::add_static_list)
pub mod diconde_meta;

// Reexports
pub use tagkey_impl::TagKey;
pub use tag_impl::Tag;
pub use dict_impl::{Dictionary, Meta, Source, PrivateIdentificationAction, StaticMetaList, DictMetrics};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("missing opening brace for TagKey (expecting: `(gggg,eeee)`)"))]
    TagKeyMissingOpeningBrace,

    #[snafu(display("missing closing brace for Tag (expecting: `(gggg,eeee)`)"))]
    TagKeyMissingClosingBrace,

    #[snafu(display("not enough components for Tag (expecting: `(gggg,eeee)`)"))]
    TagKeyMissingComponents,

    #[snafu(display("unable to parse hexadecimal numeric in Tag: {source:?}"))]
    TagKeyContainsNonHexCharacters{source: std::num::ParseIntError},

    #[snafu(display("missing opening brace for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingOpeningBrace,

    #[snafu(display("missing closing brace for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingClosingBrace,

    #[snafu(display("missing opening double quote in creator part of a Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingCreatorOpeningQuote,

    #[snafu(display("missing opening double quote in creator part of a Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingCreatorClosingQuote,

    #[snafu(display("unable to parse Tag's creator: {message} (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagInvalidCreatorString{message: String},

    #[snafu(display("not enough components for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingComponents,

    #[snafu(display("unable to parse hexadecimal numeric in Tag: {source:?}"))]
    TagContainsNonHexCharacters{source: std::num::ParseIntError},

    #[snafu(display("unable to open file({})", source))]
    DictFileOpenFailed{file_name: PathBuf, source: io::Error},

    #[snafu(display("unable to read file({})", source))]
    DictFileReadFailed{source: io::Error},

    #[snafu(display("{msg} on line {line_number} pos {char_pos} in dictionary file"))]
    DictParseFailed{line_number: usize, char_pos: usize, msg: String},
}

// Private modules
mod tagkey_impl;
mod tag_impl;
mod dict_impl;
