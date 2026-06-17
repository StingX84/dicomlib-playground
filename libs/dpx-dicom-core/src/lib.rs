#![allow(dead_code)]
#![cfg_attr(feature = "unstable", feature(test))]
#![cfg_attr(feature = "unstable", debugger_visualizer(natvis_file = "../dpx_dicom_core.natvis"))]
#![deny(clippy::all)]

// Module declarations
pub mod config;
pub mod context;
pub mod error;
pub mod tag;
#[rustfmt::skip]
pub mod tags;
pub mod uid;
#[rustfmt::skip]
pub mod uids;
mod utils;
pub mod vr;

// Public re-exports
#[doc(no_inline)]
pub use context::{AssocDescription, Context, ContextBuilder, ContextScope};
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
pub(crate) type RwLock<T> = std::sync::RwLock<T>;
pub(crate) type Vec<T> = std::vec::Vec<T>;
