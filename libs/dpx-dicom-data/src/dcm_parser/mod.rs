//! DICOM binary stream parser.
//!
//! Sibling parsers for XML and JSON representations will live alongside this
//! module. The core is sans-io ([`core`]); [`input`] provides the byte sources
//! (mmap / read-into-memory); [`reader`] is the configurable [`DcmReader`] facade.

mod core;
mod input;
mod reader;

pub use reader::{DcmReader, HeaderType, ReadMode, ReadOutput};
