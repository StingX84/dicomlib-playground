use std::ops::Range;

use bytes::Bytes;
use dpx_dicom_core::{DicomDate, DicomDateTime, DicomTime, Tag, TagKey, Vr};

use crate::item::Item;

/// On-disk header of one parsed file component, for GUI inspection / hex dump.
///
/// A normal attribute contributes one header; an `SQ`/`UN`/pixel-data attribute
/// may contribute a second for its trailing Sequence Delimitation Item, and a
/// sequence item contributes its Item (and optional Item Delimitation Item).
/// "Special attributes" are the delimiters themselves: Item (FFFE,E000), Item
/// Delimitation Item (FFFE,E00D), Sequence Delimitation Item (FFFE,E0DD).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagHeader {
    /// Offset of the first byte of this header from the file start.
    pub offset: i64,
    /// Tag key field read from the file (at `offset`, 4 bytes).
    pub tag: TagKey,
    /// VR bytes read from the file (at `offset + 4`, 2 bytes). `None` for
    /// implicit encoding and for special attributes.
    pub vr: Option<[u8; 2]>,
    /// The length field as `(value, size_in_bytes)`. `value` may be `0xFFFF_FFFF`
    /// (undefined length); `size` is 2 or 4.
    pub length: (u32, usize),
    /// Total byte length of the attribute, including any trailing delimitation
    /// item for `SQ`/`UN`/pixel data; for an Item, the size of the embedded
    /// data set. `None` only for Item / Sequence Delimitation Items.
    pub size: Option<usize>,
}

/// A stored attribute: its Value Representation plus its value, and (under the
/// `file_offsets` feature) the on-disk headers it was parsed from.
#[derive(Debug, Clone)]
pub(crate) struct Element {
    pub vr: Vr,
    pub value: Stored,
    #[cfg(feature = "file_offsets")]
    pub header: Vec<TagHeader>,
}

impl Element {
    pub(crate) fn new(vr: Vr, value: Stored) -> Self {
        Self {
            vr,
            value,
            #[cfg(feature = "file_offsets")]
            header: Vec::new(),
        }
    }
}

/// Storage form of a single attribute value.
///
/// `Mapped`/`Owned` hold raw on-wire bytes in the dataset's charset and byte
/// order; `Native` holds a logical, charset- and endianness-agnostic value as
/// written by the user. The split lets values read from a file stay as
/// zero-copy slices until they are actually decoded.
#[derive(Debug, Clone)]
pub(crate) enum Stored {
    /// Slice into the root's master buffer (memory-mapped file). No per-element
    /// reference count; resolved against the root on access.
    Mapped(Range<usize>),
    /// Owned bytes: user-supplied or produced by an edit.
    Owned(Bytes),
    /// Logical value written through the typed/dynamic API, not yet encoded.
    Native(Value),
    /// Nested sequence items (`VR = SQ`).
    Items(Vec<Item>),
}

/// Dynamic logical value of an attribute.
///
/// Coarse-grained on purpose: the exact VR is kept on the element, not encoded
/// in the arm. Multi-valued attributes use a single `'\'`-joined [`String`] for
/// text and [`OneOrMany`] for binary numerics, avoiding allocation for the
/// common single-value case.
#[derive(Debug, Clone)]
pub enum Value {
    /// Text VRs (SH LO ST LT UT UC PN UR AE CS UI ...). Multi-values are
    /// `'\'`-joined and split lazily.
    Str(String),
    /// Signed integer VRs (IS SS SL SV).
    Int(OneOrMany<i64>),
    /// Unsigned integer VRs (US UL UV).
    UInt(OneOrMany<u64>),
    /// Floating VRs (DS FL FD).
    Float(OneOrMany<f64>),
    /// `AT` — attribute tags.
    Tags(OneOrMany<Tag>),
    /// `DA`.
    Date(DicomDate),
    /// `TM`.
    Time(DicomTime),
    /// `DT`.
    DateTime(DicomDateTime),
    /// Binary VRs (OB OW OD OF OL OV UN) other than pixel data.
    Bytes(Bytes),
    /// Pixel data (7FE0,0010); boxed to keep [`Value`] small.
    Pixels(Box<PixelData>),
}

/// Pixel data, native (contiguous) or encapsulated (offset table + fragments).
#[derive(Debug, Clone)]
pub enum PixelData {
    /// Uncompressed, single contiguous blob.
    Native(Bytes),
    /// Encapsulated: Basic Offset Table plus per-fragment slices.
    Encapsulated { bot: Vec<u32>, fragments: Vec<Bytes> },
}

/// A single value inline, or many on the heap. Avoids a `Vec` allocation for
/// the common `VM = 1` case.
#[derive(Debug, Clone)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    /// Returns the first value.
    pub fn first(&self) -> &T {
        match self {
            OneOrMany::One(v) => v,
            // ponytail: `Many` is never constructed empty; enforced at the write boundary.
            OneOrMany::Many(v) => &v[0],
        }
    }

    /// Number of values held.
    pub fn len(&self) -> usize {
        match self {
            OneOrMany::One(_) => 1,
            OneOrMany::Many(v) => v.len(),
        }
    }

    /// Always at least one value.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Iterates over the held values.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        match self {
            OneOrMany::One(v) => std::slice::from_ref(v).iter(),
            OneOrMany::Many(v) => v.iter(),
        }
    }
}
