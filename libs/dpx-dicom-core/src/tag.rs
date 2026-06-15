//! Attribute [Tag], [TagKey] and associated structures

use crate::error::{DicomError, Result};
use std::path::Path;

// cSpell:ignore ggggeeee

// Private modules
mod dict_impl;
mod meta_impl;
mod tag_impl;
mod tagkey_impl;

mod generic_meta;

/// Built-in static list of [Meta] descriptions for standard DICOM Tags.
///
/// This list is automatically registered in [`Dictionary`](crate::tag::Dictionary)
#[cfg(feature = "static_dictionary")]
mod dicom_meta;

/// Built-in static list of [Meta] description for DICOM attribute tags not defined by DICOM standard.
///
/// This list is NOT automatically registered in [`Dictionary`](crate::tag::Dictionary).
/// To enable all DICONDE-specific naming, you should manually
/// add this tag to a Dictionary with [add_static_list](crate::tag::Dictionary::add_static_list)
#[cfg(feature = "static_dictionary")]
mod diconde_meta;

/// A list of [Meta] structures for all of the [generic](mod@crate::tags::generic) attributes
pub use generic_meta::ALL_TAGS_META as META_LIST_GENERIC;

/// A list of [Meta] structures for all of the attributes in this module
#[cfg(feature = "static_dictionary")]
pub use dicom_meta::ALL_TAGS_META as META_LIST_DICOM;

/// A list of [Meta] structures for all of the [diconde](mod@crate::tags::diconde) attributes
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
