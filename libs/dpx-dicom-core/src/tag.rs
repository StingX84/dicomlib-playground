//! Attribute [Tag], [TagKey] and associated structures

use snafu::{ensure, OptionExt, ResultExt, Snafu};
use std::{io, path::Path, path::PathBuf};

// cSpell:ignore ggggeeee

// Private modules
mod dict_impl;
mod meta_impl;
mod tag_impl;
mod tagkey_impl;

mod generic_meta;

/// Built-in static list of [Meta](crate::tag::Meta) descriptions for standard DICOM Tags.
///
/// This list is automatically registered in [`Dictionary`](crate::tag::Dictionary)
#[cfg(feature = "static_dictionary")]
mod dicom_meta;

/// Built-in static list of [Meta](crate::tag::Meta) description for DICOM attribute tags not defined by DICOM standard.
///
/// This list is NOT automatically registered in [`Dictionary`](crate::tag::Dictionary).
/// To enable all DICONDE-specific naming, you should manually
/// add this tag to a Dictionary with [add_static_list](crate::tag::Dictionary::add_static_list)
#[cfg(feature = "static_dictionary")]
mod diconde_meta;

/// A list of [Meta](crate::tag::Meta) structures for all of the [generic](mod@crate::tags::generic) attributes
pub use generic_meta::ALL_TAGS_META as META_LIST_GENERIC;

/// A list of [Meta](crate::tag::Meta) structures for all of the attributes in this module
#[cfg(feature = "static_dictionary")]
pub use dicom_meta::ALL_TAGS_META as META_LIST_DICOM;

/// A list of [Meta](crate::tag::Meta) structures for all of the [diconde](mod@crate::tags::diconde) attributes
#[cfg(feature = "static_dictionary")]
pub use diconde_meta::ALL_TAGS_META as META_LIST_DICONDE;

// Register standard tags only if compiled in
#[cfg(feature = "static_dictionary")]
inventory::submit! {META_LIST_DICOM}

// Reexports
pub use dict_impl::{DictMetrics, Dictionary};
pub use meta_impl::{Meta, PrivateIdentificationAction, Source, StaticMetaList};
pub use tag_impl::Tag;
pub use tagkey_impl::TagKey;

/// Result type for fallible function of this [module](crate::tag).
pub type Result<T, E = Error> = ::core::result::Result<T, E>;

/// Enumeration with errors from this [module](crate::tag).
#[derive(Debug, Snafu)]
#[allow(missing_docs)]
pub enum Error {
    #[snafu(display("missing opening brace for TagKey (expecting: `(gggg,eeee)`)"))]
    TagKeyMissingOpeningBrace,

    #[snafu(display("missing closing brace for Tag (expecting: `(gggg,eeee)`)"))]
    TagKeyMissingClosingBrace,

    #[snafu(display("not enough components for Tag (expecting: `(gggg,eeee)`)"))]
    TagKeyMissingComponents,

    #[snafu(display("unable to parse hexadecimal numeric in Tag: {source:?}"))]
    TagKeyContainsNonHexCharacters { source: std::num::ParseIntError },

    #[snafu(display("missing opening brace for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingOpeningBrace,

    #[snafu(display("missing closing brace for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingClosingBrace,

    #[snafu(display("missing opening double quote in creator part of a Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingCreatorOpeningQuote,

    #[snafu(display("missing opening double quote in creator part of a Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingCreatorClosingQuote,

    #[snafu(display(
        "unable to parse Tag's creator: {message} (expecting: `(gggg,eeee[,\"creator\"])`)"
    ))]
    TagInvalidCreatorString { message: String },

    #[snafu(display("not enough components for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"))]
    TagMissingComponents,

    #[snafu(display("unable to parse hexadecimal numeric in Tag: {source:?}"))]
    TagContainsNonHexCharacters { source: std::num::ParseIntError },

    #[snafu(display("{msg} at pos {char_pos}"))]
    MetaParseFailed { char_pos: usize, msg: String },

    #[snafu(display("unable to open file({})", source))]
    DictFileOpenFailed {
        file_name: PathBuf,
        source: io::Error,
    },

    #[snafu(display("unable to read file({})", source))]
    DictFileReadFailed { source: io::Error },

    #[snafu(display("{msg} on line {line_number} pos {char_pos} in dictionary file"))]
    DictParseFailed {
        line_number: usize,
        char_pos: usize,
        msg: String,
    },
}
