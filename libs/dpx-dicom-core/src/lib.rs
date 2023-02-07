#![allow(dead_code)]
#![cfg_attr(
    feature = "unstable",
    feature(debugger_visualizer),
    debugger_visualizer(natvis_file = "../dpx_dicom_core.natvis"),
    feature(is_sorted),
    feature(test),
    feature(once_cell)
)]
#![deny(clippy::all)]
//#![warn(missing_docs)]

// Module declarations
pub mod config;
pub mod settings;
pub mod state;
pub mod tag;
pub mod tags;
pub mod uid;
mod utils;
pub mod vr;
pub mod uids;


// Public re-exports
#[doc(no_inline)]
pub use state::State;
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
pub(crate) type Cow<'lifetime, T> = std::borrow::Cow<'lifetime, T>;
pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V>;
pub(crate) type Map<K, V> = std::collections::BTreeMap<K, V>;
pub(crate) type RwLock<T> = std::sync::RwLock<T>;
pub(crate) type Vec<T> = std::vec::Vec<T>;
