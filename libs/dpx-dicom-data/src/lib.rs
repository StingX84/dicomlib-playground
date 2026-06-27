#![deny(clippy::all)]

//! In-memory DICOM data set model.
//!
//! A [`DataSet`] owns its attributes with raw values stored either as zero-copy
//! slices into a memory-mapped file ([`Stored::Mapped`](value::Stored)) or as
//! owned bytes, and decodes them to logical [`Value`]s on demand. Character-set
//! and timezone context lives on the root only; nested [`Item`]s are
//! context-free data governed by their owning root.

mod adapt;
mod convert;
mod dataset;
mod dcm_parser;
mod dcm_writer;
mod item;
mod sequence;
mod value;

pub use convert::{FromNumber, FromValue, IntoValue};
pub use dataset::{DataSet, DatasetKind, DatasetRole};
pub use dcm_parser::{DcmReader, HeaderType, ReadMode, ReadOutput};
pub use dcm_writer::DcmWriter;
pub use dpx_dicom_core::TransferSyntax;
pub use item::Item;
pub use sequence::{ItemMut, ItemRef, Sequence, SequenceRef};
pub use value::{OneOrMany, PixelData, TagHeader, Value};
