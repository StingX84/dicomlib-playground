//! Sans-io serializer: writes an in-memory data set back to the DICOM binary
//! stream form under a target transfer syntax. The [`writer`](super::writer)
//! facade adds the File Meta header, group-length computation and deflation.
//!
//! Streaming by design (writes to any [`Write`]); only defined-length sequences
//! buffer a subtree to measure it. Raw `Mapped`/`Owned` values are written
//! through unchanged when the target byte order matches the stored one (the
//! fast path for open-modify-write), and byte-swapped per VR word width
//! otherwise.

use std::borrow::Cow;
use std::io::Write;

use dpx_dicom_core::error::{IntoDicomErr, Result};
use dpx_dicom_core::vr::Kind;
use dpx_dicom_core::{DicomTimeZoneOffset, TagKey, Vr, tags};

use dpx_dicom_core::TransferSyntax;

use crate::convert;
use crate::dataset::Shared;
use crate::item::Item;
use crate::value::{Element, PixelData, Stored, Value};

const UNDEFINED_LENGTH: u32 = 0xFFFF_FFFF;

/// Explicit-VR elements whose header uses 2 reserved bytes plus a 32-bit length.
fn is_long_form(vr: Vr) -> bool {
    matches!(
        vr,
        Vr::OB | Vr::OD | Vr::OF | Vr::OL | Vr::OV | Vr::OW | Vr::SQ | Vr::SV | Vr::UC | Vr::UN | Vr::UR | Vr::UT | Vr::UV
    )
}

/// The byte that pads a raw value to even length: space for text VRs, NUL for
/// `UI` and binary VRs (matches `convert::encode`).
fn pad_byte(vr: Vr) -> u8 {
    if matches!(vr.info().kind, Kind::Text { .. }) && vr != Vr::UI {
        b' '
    } else {
        0
    }
}

/// Byte width of one stored word for `vr` (for byte-order transcoding of raw
/// values). 1 means an opaque byte stream that never needs swapping.
fn word_width(vr: Vr) -> usize {
    match vr.info().kind {
        Kind::U16 | Kind::I16 => 2,
        Kind::U32 | Kind::I32 | Kind::F32 => 4,
        Kind::U64 | Kind::I64 | Kind::F64 => 8,
        _ => 1,
    }
}

/// Returns `raw` reordered from `from` to `to` byte order per the VR word width.
/// Borrows unchanged on the fast path (orders equal or opaque bytes).
fn transcode(raw: &[u8], vr: Vr, from_le: bool, to_le: bool) -> Cow<'_, [u8]> {
    let w = word_width(vr);
    if from_le == to_le || w == 1 {
        Cow::Borrowed(raw)
    } else {
        let mut v = raw.to_vec();
        for chunk in v.chunks_exact_mut(w) {
            chunk.reverse();
        }
        Cow::Owned(v)
    }
}

pub(crate) struct Serializer<'a, W: Write> {
    out: W,
    shared: &'a Shared,
    target: &'static TransferSyntax,
    /// Write SQ and items with undefined length + delimiters (else defined length).
    undefined_sq: bool,
}

impl<'a, W: Write> Serializer<'a, W> {
    pub(crate) fn new(out: W, shared: &'a Shared, target: &'static TransferSyntax, undefined_sq: bool) -> Self {
        Self { out, shared, target, undefined_sq }
    }

    fn put(&mut self, bytes: &[u8]) -> Result<()> {
        self.out.write_all(bytes).to_dicom_err_with(|| "writing data set".to_string())
    }
    fn put_u16(&mut self, v: u16) -> Result<()> {
        if self.target.is_little_endian { self.put(&v.to_le_bytes()) } else { self.put(&v.to_be_bytes()) }
    }
    fn put_u32(&mut self, v: u32) -> Result<()> {
        if self.target.is_little_endian { self.put(&v.to_le_bytes()) } else { self.put(&v.to_be_bytes()) }
    }
    fn put_tag(&mut self, tag: TagKey) -> Result<()> {
        self.put_u16(tag.group())?;
        self.put_u16(tag.element())
    }

    /// A special attribute (Item / Item Delimitation / Sequence Delimitation):
    /// tag plus a 4-byte length, no VR.
    fn put_delimiter(&mut self, tag: TagKey, length: u32) -> Result<()> {
        self.put_tag(tag)?;
        self.put_u32(length)
    }

    /// Element header: tag, VR (explicit only) and a 2- or 4-byte length.
    fn write_header(&mut self, tag: TagKey, vr: Vr, length: u32) -> Result<()> {
        self.put_tag(tag)?;
        if self.target.is_explicit_vr {
            self.put(&vr.code())?;
            if is_long_form(vr) {
                self.put(&[0, 0])?; // reserved
                self.put_u32(length)?;
            } else {
                self.put_u16(length as u16)?;
            }
        } else {
            self.put_u32(length)?;
        }
        Ok(())
    }

    /// Writes the elements of `item` in stored order.
    pub(crate) fn elements(&mut self, item: &Item) -> Result<()> {
        for (key, el) in item.map.entries() {
            self.element(*key, el)?;
        }
        Ok(())
    }

    /// Writes the elements of `item` except `skip` (used to drop the File Meta
    /// group length, which is recomputed).
    pub(crate) fn elements_skipping(&mut self, item: &Item, skip: TagKey) -> Result<()> {
        for (key, el) in item.map.entries() {
            if *key != skip {
                self.element(*key, el)?;
            }
        }
        Ok(())
    }

    /// Writes the root data set, auto-stamping (0008,0201) Timezone Offset From
    /// UTC when the dataset has no explicit one and the configured default is not
    /// local. Inserted in ascending tag order.
    pub(crate) fn root(&mut self, item: &Item) -> Result<()> {
        let tz = self.shared.default_tz();
        let stamp = !self.shared.has_root_tz() && !matches!(tz, DicomTimeZoneOffset::Local);
        let mut done = !stamp;
        for (key, el) in item.map.entries() {
            if !done && key.0 > tags::TimezoneOffsetFromUTC.key.0 {
                self.write_timezone(tz)?;
                done = true;
            }
            self.element(*key, el)?;
        }
        if !done {
            self.write_timezone(tz)?;
        }
        Ok(())
    }

    fn write_timezone(&mut self, tz: DicomTimeZoneOffset) -> Result<()> {
        let mut buf = Vec::new();
        tz.to_dicom(&mut buf);
        if buf.len() % 2 == 1 {
            buf.push(b' ');
        }
        self.write_header(tags::TimezoneOffsetFromUTC.key, Vr::SH, buf.len() as u32)?;
        self.put(&buf)
    }

    fn element(&mut self, tag: TagKey, el: &Element) -> Result<()> {
        match &el.value {
            Stored::Items(items) => self.sequence(tag, el.vr, items),
            Stored::Native(Value::Pixels(px)) => self.pixels(tag, el.vr, px),
            _ => self.primitive(tag, el),
        }
    }

    fn primitive(&mut self, tag: TagKey, el: &Element) -> Result<()> {
        let shared = self.shared;
        let body: Cow<[u8]> = match &el.value {
            Stored::Mapped(r) => transcode(&shared.master()[r.clone()], el.vr, shared.is_little_endian(), self.target.is_little_endian),
            Stored::Owned(b) => transcode(b, el.vr, shared.is_little_endian(), self.target.is_little_endian),
            Stored::Native(v) => {
                let mut buf = Vec::new();
                convert::encode(shared, self.target.is_little_endian, el.vr, v, &mut buf)?;
                Cow::Owned(buf)
            }
            Stored::Items(_) => unreachable!("handled by element()"),
        };
        // DICOM values are even-length. `Native` is already padded by `encode`;
        // raw values from a malformed file may be odd — pad so the stream stays
        // aligned for the next element.
        let odd = body.len() % 2 == 1;
        let length = body.len() + usize::from(odd);
        self.write_header(tag, el.vr, length as u32)?;
        self.put(&body)?;
        if odd {
            self.put(&[pad_byte(el.vr)])?;
        }
        Ok(())
    }

    fn sequence(&mut self, tag: TagKey, vr: Vr, items: &[Item]) -> Result<()> {
        if self.undefined_sq {
            self.write_header(tag, vr, UNDEFINED_LENGTH)?;
            for item in items {
                self.put_delimiter(tags::Item.key, UNDEFINED_LENGTH)?;
                self.elements(item)?;
                self.put_delimiter(tags::ItemDelimitationItem.key, 0)?;
            }
            self.put_delimiter(tags::SequenceDelimitationItem.key, 0)
        } else {
            let mut body = Vec::new();
            {
                let mut sub = Serializer::new(&mut body, self.shared, self.target, self.undefined_sq);
                for item in items {
                    let mut content = Vec::new();
                    Serializer::new(&mut content, sub.shared, sub.target, sub.undefined_sq).elements(item)?;
                    sub.put_delimiter(tags::Item.key, content.len() as u32)?;
                    sub.put(&content)?;
                }
            }
            self.write_header(tag, vr, body.len() as u32)?;
            self.put(&body)
        }
    }

    fn pixels(&mut self, tag: TagKey, vr: Vr, px: &PixelData) -> Result<()> {
        match px {
            PixelData::Native(b) => {
                self.write_header(tag, vr, b.len() as u32)?;
                self.put(b)
            }
            PixelData::Encapsulated { bot, fragments } => {
                self.write_header(tag, vr, UNDEFINED_LENGTH)?;
                self.put_delimiter(tags::Item.key, (bot.len() * 4) as u32)?;
                for &offset in bot {
                    self.put_u32(offset)?;
                }
                for fragment in fragments {
                    self.put_delimiter(tags::Item.key, fragment.len() as u32)?;
                    self.put(fragment)?;
                }
                self.put_delimiter(tags::SequenceDelimitationItem.key, 0)
            }
        }
    }
}
