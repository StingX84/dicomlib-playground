//! DICOM binary stream serializer.
//!
//! The counterpart of [`dcm_parser`](crate::dcm_parser): the core is sans-io
//! ([`core`]); [`writer`] is the configurable [`DcmWriter`] facade that adds the
//! File Meta header and deflation. Sibling XML/JSON writers will live alongside.

mod core;
mod writer;

pub use writer::DcmWriter;
