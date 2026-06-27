#![cfg_attr(all(test, feature = "unstable"), feature(test))]
#![cfg_attr(feature = "unstable", debugger_visualizer(natvis_file = "../dpx_dicom_core.natvis"))]
#![deny(clippy::all)]

// Module declarations
pub mod config;
pub mod context;
pub mod error;
pub mod event;
pub mod tag;
#[rustfmt::skip]
pub mod tags;
pub mod uid;
#[rustfmt::skip]
pub mod uids;
pub mod network;
mod utils;
pub mod vr;

/// Re-exported for use by the [`config!`](crate::config!) macro. Not part of the
/// stable public API.
#[doc(hidden)]
pub use inventory as __inventory;

// Public re-exports
#[doc(no_inline)]
pub use context::{Context, ContextBuilder, ContextScope};
#[doc(no_inline)]
pub use error::{DicomError, ErrContext, ErrorKind, IntoDicomErr, KbEntry, Result, ToErrorKind};
#[doc(no_inline)]
pub use tag::Tag;
#[doc(no_inline)]
pub use tag::TagKey;
#[doc(no_inline)]
pub use uid::Uid;
#[doc(no_inline)]
pub use vr::Vr;

// Crate STD lib types
pub(crate) type Arc<T> = std::sync::Arc<T>;
pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V>;
pub(crate) type Map<K, V> = std::collections::BTreeMap<K, V>;
pub(crate) type Vec<T> = std::vec::Vec<T>;
