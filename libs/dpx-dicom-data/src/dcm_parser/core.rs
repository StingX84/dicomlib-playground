//! Sans-io parser core: recursive-descent over an in-memory byte buffer,
//! performing no I/O itself. The [`input`](super::input) drivers obtain the
//! buffer (mmap or read-into-memory); a future async wrapper feeds the same
//! core.
//!
//! Handles the File Meta header (preamble + group 0002), Explicit/Implicit VR
//! (LE/BE), defined and undefined-length sequences with Item / Item Delimitation
//! / Sequence Delimitation special attributes, encapsulated pixel data (Basic
//! Offset Table + fragments), transfer-syntax selection, a tag whitelist and an
//! early-stop tag. The Deflated Explicit VR LE transfer syntax is inflated by the
//! [`reader`](super::reader) before this core runs, so the core only ever parses
//! uncompressed bytes.
//!
//! Parsing is deliberately lenient to cover decades of non-conforming files: a
//! truncated, wrongly-sized, or out-of-order element ends its enclosing level
//! rather than failing the whole read; an Explicit-VR element whose VR bytes are
//! not a valid VR is re-read as Implicit VR; an undefined length on a non-SQ,
//! non-PixelData element is read as a sequence; a File Meta header without a
//! group length is delimited by scanning group 0002. Every such deviation is
//! reported via [`Parser::note`] at INFO (target `dpx_dicom::parse`), gated by
//! the same `disable_tracing` flag as element tracing.
//!
//! Under Implicit VR a tag whose dictionary lists more than one VR (PixelData
//! "OB or OW", PixelPaddingValue "US or SS", …) is stored with `Vr::Undefined`
//! and a flag is set; [`Parser::resolve_ambiguous`] then runs a single post-read
//! pass that rewrites those VRs from sibling values (Bits Allocated, Pixel
//! Representation). Only the VR label changes — OB/OW and US/SS share identical
//! on-wire bytes — so nothing is re-read or moved.

use std::cell::Cell;

use bytes::Bytes;
use dpx_dicom_core::error::Result;
use dpx_dicom_core::{Tag, TagKey, TransferSyntax, Vr, ensure, tags};
use tracing::{info, trace};

use super::input::Source;
use crate::dataset::{DataSet, DatasetKind};
use crate::item::{ElementMap, Item, PushNote};
use crate::value::{Element, PixelData, Stored, Value};
#[cfg(feature = "file_offsets")]
use crate::value::TagHeader;

const UNDEFINED_LENGTH: u32 = 0xFFFF_FFFF;
const PREAMBLE_LEN: usize = 128;
const META_START: usize = 132; // 128-byte preamble + "DICM"

/// Explicit-VR elements whose header uses 2 reserved bytes plus a 32-bit length.
fn is_long_form(vr: Vr) -> bool {
    matches!(
        vr,
        Vr::OB | Vr::OD | Vr::OF | Vr::OL | Vr::OV | Vr::OW | Vr::SQ | Vr::SV | Vr::UC | Vr::UN | Vr::UR | Vr::UT | Vr::UV
    )
}

fn vr_from_code(code: [u8; 2]) -> Vr {
    Vr::all().iter().find(|m| m.code == code).map(|m| m.vr).unwrap_or(Vr::UN)
}

fn is_vr_code(code: [u8; 2]) -> bool {
    Vr::all().iter().any(|m| m.code == code)
}

fn read_u16(buf: &[u8], at: usize, little_endian: bool) -> u16 {
    let a = [buf[at], buf[at + 1]];
    if little_endian { u16::from_le_bytes(a) } else { u16::from_be_bytes(a) }
}

fn read_u32(buf: &[u8], at: usize, little_endian: bool) -> u32 {
    let a = [buf[at], buf[at + 1], buf[at + 2], buf[at + 3]];
    if little_endian { u32::from_le_bytes(a) } else { u32::from_be_bytes(a) }
}

fn read_tag(buf: &[u8], at: usize, little_endian: bool) -> TagKey {
    TagKey::new(read_u16(buf, at, little_endian), read_u16(buf, at + 2, little_endian))
}

/// The byte-order + VR-encoding state the recursive parser threads through one
/// level. Derived from the file's [`TransferSyntax`] but mutable per element:
/// the lenient reader flips `explicit_vr` off to retry a garbage VR as implicit.
#[derive(Clone, Copy)]
struct Decoder {
    little_endian: bool,
    explicit_vr: bool,
}

impl Decoder {
    fn from_ts(ts: &TransferSyntax) -> Self {
        Decoder { little_endian: ts.is_little_endian, explicit_vr: ts.is_explicit_vr }
    }
}

/// Decoding state for the File Meta group (0002), always Explicit VR Little Endian.
const META_LE: Decoder = Decoder { little_endian: true, explicit_vr: true };

/// Per-item file offsets collected while parsing encapsulated pixel data;
/// a zero-size `()` when `file_offsets` is disabled.
#[cfg(feature = "file_offsets")]
type PixelSpans = Vec<TagHeader>;
#[cfg(not(feature = "file_offsets"))]
type PixelSpans = ();

/// A [`TagHeader`] for an Item (FFFE,E000) carrying `content_len` content bytes.
#[cfg(feature = "file_offsets")]
fn item_header(offset: usize, content_len: u32) -> TagHeader {
    TagHeader { offset: offset as i64, tag: tags::Item.key, vr: None, length: (content_len, 4), size: Some(content_len as usize) }
}

/// Sibling values that disambiguate a dictionary VR, gathered per data-set level
/// and inherited by nested items (a nested element falls back to its ancestors).
#[derive(Default, Clone, Copy)]
struct AmbiguityCtx {
    /// (0028,0100) Bits Allocated — chooses OB vs OW for image pixel data.
    bits_allocated: Option<u16>,
    /// (0028,0103) Pixel Representation — chooses US vs SS.
    pixel_rep: Option<u16>,
    /// (5400,1004) Waveform Bits Allocated — chooses OB vs OW for waveform data.
    waveform_bits: Option<u16>,
}

/// A parsed element header (tag, VR, length), without the value content.
struct Header {
    tag: TagKey,
    vr: Vr,
    // Only consumed under `file_offsets` (recorded in `TagHeader`).
    #[cfg_attr(not(feature = "file_offsets"), allow(dead_code))]
    vr_code: Option<[u8; 2]>,
    length: (u32, usize),
    value_start: usize,
    undefined: bool,
}

/// Recursive-descent builder over a fully-buffered stream.
struct Parser<'a> {
    buf: &'a [u8],
    /// The same bytes as `buf`, kept as `Bytes` so encapsulated pixel-data
    /// fragments can be sliced zero-copy (shares the mmap/in-memory Arc).
    master: &'a Bytes,
    mapped: bool,
    /// Sorted whitelist; `None` keeps every (top-level) tag.
    whitelist: Option<&'a [TagKey]>,
    /// Stop once a top-level tag exceeds this; `TagKey::MAX` = no limit.
    stop_after: TagKey,
    disable_tracing: bool,
    /// Set when at least one element got an ambiguous (`Vr::Undefined`) VR that a
    /// post-read pass must resolve. Interior mutability keeps parsing on `&self`.
    ambiguous: Cell<bool>,
}

impl Parser<'_> {
    fn kept(&self, tag: TagKey) -> bool {
        self.whitelist.is_none_or(|wl| wl.binary_search_by(|k| k.0.cmp(&tag.0)).is_ok())
    }

    fn trace(&self, pos: usize, h: &Header) {
        if !self.disable_tracing {
            trace!(
                target: "dpx_dicom::parse",
                group = h.tag.group(), element = h.tag.element(),
                vr = ?h.vr, length = h.length.0, offset = pos,
                "element"
            );
        }
    }

    /// Reports a recoverable parsing anomaly (a file deviating from the standard)
    /// at INFO, gated by the same flag as element tracing.
    fn note(&self, args: std::fmt::Arguments) {
        if !self.disable_tracing {
            info!(target: "dpx_dicom::parse", "{args}");
        }
    }

    fn note_push(&self, tag: TagKey, pos: usize, note: PushNote) {
        let what = match note {
            PushNote::Duplicate => "duplicate",
            PushNote::OutOfOrder => "out-of-order",
        };
        self.note(format_args!("{what} tag ({:04X},{:04X}) at offset {pos}", tag.group(), tag.element()));
    }

    /// Parses the element header at `pos` under `ts`. Lenient about real-world
    /// deviations: delimitation items (group FFFE) never carry a VR even in
    /// Explicit VR; an Explicit-VR element whose "VR" bytes are not a valid VR
    /// is re-read as Implicit VR Little Endian (mixed encoding, as produced by
    /// some converters). Buffer bounds for the *value* are checked by the caller.
    fn read_header(&self, pos: usize, dec: Decoder) -> Result<Header> {
        let buf = self.buf;
        let order = dec.little_endian;
        ensure!(pos + 4 <= buf.len(), InvalidData, "truncated tag at offset {pos}");
        let tag = read_tag(buf, pos, order);
        let mut p = pos + 4;
        let is_delim = tag.group() == 0xFFFE;

        let (vr, vr_code, length_value, length_size) = if dec.explicit_vr && !is_delim {
            ensure!(p + 2 <= buf.len(), InvalidData, "truncated VR at offset {pos}");
            let code = [buf[p], buf[p + 1]];
            if !is_vr_code(code) {
                self.note(format_args!(
                    "non-standard VR {:02X}\\{:02X} for ({:04X},{:04X}) at offset {pos}; reading as Implicit VR",
                    code[0], code[1], tag.group(), tag.element()
                ));
                return self.read_header(pos, Decoder { explicit_vr: false, ..dec });
            }
            let vr = vr_from_code(code);
            p += 2;
            if is_long_form(vr) {
                ensure!(p + 6 <= buf.len(), InvalidData, "truncated long-form length at offset {pos}");
                p += 2; // reserved
                let len = read_u32(buf, p, order);
                p += 4;
                (vr, Some(code), len, 4usize)
            } else {
                ensure!(p + 2 <= buf.len(), InvalidData, "truncated length at offset {pos}");
                let len = read_u16(buf, p, order) as u32;
                p += 2;
                (vr, Some(code), len, 2usize)
            }
        } else {
            ensure!(p + 4 <= buf.len(), InvalidData, "truncated length at offset {pos}");
            let len = read_u32(buf, p, order);
            p += 4;
            // Implicit VR carries no VR on the wire — take it from the dictionary.
            // When the dictionary lists more than one VR (e.g. PixelData "OB or OW",
            // PixelPaddingValue "US or SS"), defer the choice: mark it ambiguous
            // and let the post-read pass resolve it from sibling values.
            let vr = if is_delim {
                Vr::UN
            } else {
                match Tag::new(tag, None).meta() {
                    Some(m) if m.vr.1 != Vr::Undefined => {
                        self.ambiguous.set(true);
                        Vr::Undefined
                    }
                    Some(m) => m.vr.0,
                    None => Vr::UN,
                }
            };
            (vr, None, len, 4usize)
        };

        let undefined = length_value == UNDEFINED_LENGTH;
        Ok(Header { tag, vr, vr_code, length: (length_value, length_size), value_start: p, undefined })
    }

    /// Scans the File Meta group (0002) from [`META_START`], returning the offset
    /// of the first element whose group is not 0x0002 (where the main data set
    /// begins). Used when the group length element is absent or unusable.
    fn scan_group_two(&self) -> usize {
        let mut pos = META_START;
        loop {
            let Ok(h) = self.read_header(pos, META_LE) else { break pos };
            if h.tag.group() != 0x0002 || h.undefined {
                break pos;
            }
            let end = h.value_start + h.length.0 as usize;
            if end <= pos || end > self.buf.len() {
                break pos;
            }
            pos = end;
        }
    }

    /// Resolves every element left with `Vr::Undefined` (an ambiguous dictionary
    /// VR read under Implicit VR) into a concrete VR, recursing into sequence
    /// items. Only the VR label is rewritten — the raw bytes are identical for
    /// OB/OW and US/SS, so nothing is moved. Run once after a read, gated by the
    /// `ambiguous` flag.
    fn resolve_ambiguous(&self, map: &mut ElementMap, dec: Decoder, inherited: &AmbiguityCtx) {
        let ctx = AmbiguityCtx {
            bits_allocated: self.sibling_u16(map, tags::BitsAllocated.key, dec).or(inherited.bits_allocated),
            pixel_rep: self.sibling_u16(map, tags::PixelRepresentation.key, dec).or(inherited.pixel_rep),
            waveform_bits: self.sibling_u16(map, tags::WaveformBitsAllocated.key, dec).or(inherited.waveform_bits),
        };
        for (key, el) in map.entries_mut() {
            if let Stored::Items(items) = &mut el.value {
                for item in items.iter_mut() {
                    self.resolve_ambiguous(&mut item.map, dec, &ctx);
                }
            } else if el.vr == Vr::Undefined {
                el.vr = self.disambiguate(*key, &ctx);
            }
        }
    }

    /// Picks the concrete VR for an ambiguous tag from its dictionary VR pair and
    /// the sibling context, mirroring DCMTK's `checkAndUpdateVR` (OB/OW by Bits
    /// Allocated, US/SS by Pixel Representation), defaulting to the wider/unsigned
    /// VR when the deciding sibling is absent.
    fn disambiguate(&self, tag: TagKey, ctx: &AmbiguityCtx) -> Vr {
        let Some(meta) = Tag::new(tag, None).meta() else { return Vr::UN };
        match (meta.vr.0, meta.vr.1) {
            (Vr::OB, Vr::OW) => {
                let bits = if tag.group() == 0x5400 { ctx.waveform_bits } else { ctx.bits_allocated };
                match bits {
                    Some(b) => {
                        if b > 8 {
                            Vr::OW
                        } else {
                            Vr::OB
                        }
                    }
                    None => {
                        self.note(format_args!(
                            "cannot resolve OB/OW for ({:04X},{:04X}): Bits Allocated absent; assuming OW",
                            tag.group(), tag.element()
                        ));
                        Vr::OW
                    }
                }
            }
            (Vr::US, Vr::SS) => match ctx.pixel_rep {
                Some(1) => Vr::SS,
                Some(_) => Vr::US,
                None => {
                    self.note(format_args!(
                        "cannot resolve US/SS for ({:04X},{:04X}): Pixel Representation absent; assuming US",
                        tag.group(), tag.element()
                    ));
                    Vr::US
                }
            },
            (v0, _) => {
                self.note(format_args!(
                    "unhandled ambiguous VR for ({:04X},{:04X}); assuming {v0:?}",
                    tag.group(), tag.element()
                ));
                v0
            }
        }
    }

    /// Reads a `US`-style 16-bit sibling value (raw, by transfer-syntax byte
    /// order) used to disambiguate VRs. `None` if absent or too short.
    fn sibling_u16(&self, map: &ElementMap, key: TagKey, dec: Decoder) -> Option<u16> {
        let bytes = match &map.get(key)?.value {
            Stored::Mapped(r) => self.buf.get(r.clone())?,
            Stored::Owned(b) => &b[..],
            _ => return None,
        };
        (bytes.len() >= 2).then(|| read_u16(bytes, 0, dec.little_endian))
    }

    /// Top-level (or File Meta) data set in `[start, end)`: applies the
    /// whitelist filter and the early-stop tag.
    fn dataset(&self, start: usize, end: usize, dec: Decoder) -> Result<ElementMap> {
        let mut map = ElementMap::default();
        let mut pos = start;
        while pos + 4 <= end {
            let tag = read_tag(self.buf, pos, dec.little_endian);
            if tag.0 > self.stop_after.0 {
                break;
            }
            match self.element(pos, dec) {
                Ok((etag, el, next)) => {
                    if self.kept(etag)
                        && let Some(note) = map.push_parsed(etag, el)
                    {
                        self.note_push(etag, pos, note);
                    }
                    if next <= pos {
                        break;
                    }
                    pos = next;
                }
                Err(e) => {
                    self.note(format_args!("stopping data set at offset {pos}: {e}"));
                    break;
                }
            }
        }
        Ok(map)
    }

    /// Parses one data element at `pos`, recursing into sequences. Returns the
    /// tag, the built element, and the offset just past it.
    fn element(&self, pos: usize, dec: Decoder) -> Result<(TagKey, Element, usize)> {
        let h = self.read_header(pos, dec)?;
        self.trace(pos, &h);

        // Undefined length is valid only for sequences and for encapsulated
        // PixelData (7FE0,0010). Anything else with undefined length is, per
        // PS3.5 6.2.2, an implicitly-encoded sequence (commonly a private SQ
        // read as VR UN) — parse it as one rather than as pixel data.
        let is_encapsulated_pixels = h.undefined && h.tag == tags::PixelData.key;
        if h.vr == Vr::SQ || (h.undefined && !is_encapsulated_pixels) {
            if h.vr != Vr::SQ {
                self.note(format_args!(
                    "undefined-length ({:04X},{:04X}) VR={:?} at offset {pos}; reading as a sequence",
                    h.tag.group(), h.tag.element(), h.vr
                ));
            }
            let (items, next) = self.sequence(&h, dec);
            #[cfg_attr(not(feature = "file_offsets"), allow(unused_mut))]
            let mut el = Element::new(Vr::SQ, Stored::Items(items));
            #[cfg(feature = "file_offsets")]
            {
                el.header.push(TagHeader {
                    offset: pos as i64,
                    tag: h.tag,
                    vr: h.vr_code,
                    length: h.length,
                    size: Some(next - pos),
                });
                if h.undefined {
                    el.header.push(TagHeader {
                        offset: (next - 8) as i64,
                        tag: tags::SequenceDelimitationItem.key,
                        vr: None,
                        length: (0, 4),
                        size: None,
                    });
                }
            }
            return Ok((h.tag, el, next));
        }

        if is_encapsulated_pixels {
            #[cfg(feature = "file_offsets")]
            let (px, next, spans) = self.encapsulated(&h, pos, dec);
            #[cfg(not(feature = "file_offsets"))]
            let (px, next, _spans) = self.encapsulated(&h, pos, dec);
            #[cfg_attr(not(feature = "file_offsets"), allow(unused_mut))]
            let mut el = Element::new(h.vr, Stored::Native(Value::Pixels(Box::new(px))));
            #[cfg(feature = "file_offsets")]
            {
                el.header = spans;
            }
            return Ok((h.tag, el, next));
        }

        let value_end = h.value_start + h.length.0 as usize;
        ensure!(
            value_end <= self.buf.len(),
            InvalidData,
            "value of {} bytes for ({:04X},{:04X}) exceeds buffer at offset {pos}",
            h.length.0,
            h.tag.group(),
            h.tag.element()
        );
        let value = if self.mapped {
            Stored::Mapped(h.value_start..value_end)
        } else {
            Stored::Owned(Bytes::copy_from_slice(&self.buf[h.value_start..value_end]))
        };
        #[cfg_attr(not(feature = "file_offsets"), allow(unused_mut))]
        let mut el = Element::new(h.vr, value);
        #[cfg(feature = "file_offsets")]
        el.header.push(TagHeader {
            offset: pos as i64,
            tag: h.tag,
            vr: h.vr_code,
            length: h.length,
            size: Some(value_end - pos),
        });
        Ok((h.tag, el, value_end))
    }

    /// Parses encapsulated (compressed) pixel data: an undefined-length element
    /// at `el_offset` whose value is a Basic Offset Table item followed by
    /// fragment items, terminated by a Sequence Delimitation Item. Fragments are
    /// sliced zero-copy from the master buffer. Returns the pixel data, the
    /// offset just past the delimiter, and (under `file_offsets`) one
    /// [`TagHeader`] per discovered item: the pixel-data element, the Basic
    /// Offset Table, each fragment, and the Sequence Delimitation Item.
    fn encapsulated(&self, h: &Header, el_offset: usize, dec: Decoder) -> (PixelData, usize, PixelSpans) {
        let order = dec.little_endian;
        let mut pos = h.value_start;
        let mut bot = Vec::new();
        let mut fragments: Vec<Bytes> = Vec::new();
        #[cfg(feature = "file_offsets")]
        let mut spans: Vec<TagHeader> = Vec::new();

        // First item is the Basic Offset Table (it may be empty). Some files
        // omit it; tolerate that and treat what follows as the first fragment.
        if pos + 8 <= self.buf.len() && read_tag(self.buf, pos, order) == tags::Item.key {
            let bot_len = read_u32(self.buf, pos + 4, order);
            let bot_end = pos + 8 + bot_len as usize;
            if bot_len != UNDEFINED_LENGTH && bot_end <= self.buf.len() {
                #[cfg(feature = "file_offsets")]
                spans.push(item_header(pos, bot_len));
                pos += 8;
                bot.reserve(bot_len as usize / 4);
                while pos + 4 <= bot_end {
                    bot.push(read_u32(self.buf, pos, order));
                    pos += 4;
                }
                pos = bot_end;
            } else {
                self.note(format_args!("malformed Basic Offset Table at offset {pos}; treating as fragments"));
            }
        } else {
            self.note(format_args!("missing Basic Offset Table at offset {pos}"));
        }

        loop {
            if pos + 8 > self.buf.len() {
                self.note(format_args!("encapsulated PixelData truncated, missing delimiter at offset {pos}"));
                break;
            }
            let tag = read_tag(self.buf, pos, order);
            let len = read_u32(self.buf, pos + 4, order);
            if tag == tags::SequenceDelimitationItem.key {
                #[cfg(feature = "file_offsets")]
                spans.push(TagHeader { offset: pos as i64, tag, vr: None, length: (0, 4), size: None });
                pos += 8;
                break;
            }
            if tag != tags::Item.key || len == UNDEFINED_LENGTH {
                self.note(format_args!(
                    "expected a fragment item (FFFE,E000) but found ({:04X},{:04X}) at offset {pos}; ending PixelData",
                    tag.group(), tag.element()
                ));
                break;
            }
            let start = pos + 8;
            let end = start + len as usize;
            if end > self.buf.len() {
                self.note(format_args!("pixel-data fragment exceeds the buffer at offset {pos}; truncating"));
                break;
            }
            #[cfg(feature = "file_offsets")]
            spans.push(item_header(pos, len));
            fragments.push(self.master.slice(start..end));
            pos = end;
        }

        #[cfg(feature = "file_offsets")]
        {
            let mut all = Vec::with_capacity(spans.len() + 1);
            all.push(TagHeader {
                offset: el_offset as i64,
                tag: h.tag,
                vr: h.vr_code,
                length: h.length,
                size: Some(pos - el_offset),
            });
            all.append(&mut spans);
            (PixelData::Encapsulated { bot, fragments }, pos, all)
        }
        #[cfg(not(feature = "file_offsets"))]
        {
            let _ = el_offset;
            (PixelData::Encapsulated { bot, fragments }, pos, ())
        }
    }

    /// Parses the items of the sequence whose header is `h`. Returns the items
    /// and the offset just past the sequence (including any delimiter). Lenient:
    /// a missing delimiter, a truncated item header, or an unexpected tag ends
    /// the sequence so the parent can carry on.
    fn sequence(&self, h: &Header, dec: Decoder) -> (Vec<Item>, usize) {
        let order = dec.little_endian;
        let defined_end = (h.value_start + h.length.0 as usize).min(self.buf.len());
        let mut items = Vec::new();
        let mut pos = h.value_start;
        loop {
            if !h.undefined && pos >= defined_end {
                break;
            }
            if pos + 8 > self.buf.len() {
                if h.undefined {
                    self.note(format_args!("sequence truncated, missing delimiter at offset {pos}"));
                }
                break;
            }
            let item_tag = read_tag(self.buf, pos, order);
            let item_len = read_u32(self.buf, pos + 4, order);
            if item_tag == tags::SequenceDelimitationItem.key {
                pos += 8;
                break;
            }
            if item_tag != tags::Item.key {
                self.note(format_args!(
                    "expected an Item (FFFE,E000) but found ({:04X},{:04X}) at offset {pos}; ending sequence",
                    item_tag.group(), item_tag.element()
                ));
                break;
            }
            let (item, next) = self.item(pos, item_len, dec);
            items.push(item);
            if next <= pos {
                break;
            }
            pos = next;
        }
        (items, pos)
    }

    /// Parses one sequence item starting at `item_start` (its Item tag), whose
    /// declared length is `item_len`. Returns the item and the offset past it.
    /// Lenient: errors inside the item content stop it without aborting the
    /// surrounding sequence.
    fn item(&self, item_start: usize, item_len: u32, dec: Decoder) -> (Item, usize) {
        let undefined = item_len == UNDEFINED_LENGTH;
        let content_start = item_start + 8;
        let mut map = ElementMap::default();
        let mut pos = content_start;
        let defined_end = (content_start + item_len as usize).min(self.buf.len());

        let mut delim_consumed = false;
        let content_end = loop {
            if !undefined && pos >= defined_end {
                break pos;
            }
            if pos + 4 > self.buf.len() {
                break pos;
            }
            if undefined && read_tag(self.buf, pos, dec.little_endian) == tags::ItemDelimitationItem.key {
                delim_consumed = true;
                break pos;
            }
            match self.element(pos, dec) {
                Ok((etag, el, next)) => {
                    if let Some(note) = map.push_parsed(etag, el) {
                        self.note_push(etag, pos, note);
                    }
                    if next <= pos {
                        break pos;
                    }
                    pos = next;
                }
                Err(e) => {
                    self.note(format_args!("stopping item at offset {pos}: {e}"));
                    break pos;
                }
            }
        };
        // For a defined-length item resume past its declared length to stay
        // aligned; for undefined length, past the Item Delimitation Item if seen.
        let next = if undefined {
            if delim_consumed { content_end + 8 } else { content_end }
        } else {
            defined_end
        };

        #[cfg_attr(not(feature = "file_offsets"), allow(unused_mut))]
        let mut item = Item::from_map(map);
        #[cfg(feature = "file_offsets")]
        {
            item.header.push(TagHeader {
                offset: item_start as i64,
                tag: tags::Item.key,
                vr: None,
                length: (item_len, 4),
                size: Some(content_end - content_start),
            });
            if delim_consumed {
                item.header.push(TagHeader {
                    offset: content_end as i64,
                    tag: tags::ItemDelimitationItem.key,
                    vr: None,
                    length: (0, 4),
                    size: None,
                });
            }
        }
        (item, next)
    }
}

fn assemble(source: &Source, map: ElementMap, xfer: &'static TransferSyntax, kind: DatasetKind) -> Result<DataSet> {
    let master = if source.mapped { source.data.clone() } else { Bytes::new() };
    let mut ds = DataSet::parsed(master, xfer, kind);
    *ds.root_mut() = Item::from_map(map);
    ds.sync_context()?;
    Ok(ds)
}

/// Parses a data set from `[start, end)` under `ts`, keeping only whitelisted
/// tags and stopping after `stop_after`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_dataset(
    source: &Source,
    start: usize,
    end: usize,
    ts: &'static TransferSyntax,
    kind: DatasetKind,
    stop_after: TagKey,
    whitelist: Option<&[TagKey]>,
    disable_tracing: bool,
) -> Result<DataSet> {
    let parser = Parser { buf: &source.data, master: &source.data, mapped: source.mapped, whitelist, stop_after, disable_tracing, ambiguous: Cell::new(false) };
    let dec = Decoder::from_ts(ts);
    let mut map = parser.dataset(start, end, dec)?;
    if parser.ambiguous.get() {
        parser.resolve_ambiguous(&mut map, dec, &AmbiguityCtx::default());
    }
    assemble(source, map, ts, kind)
}

/// Builds one flat data set from the File Meta group and the main data set
/// (different encodings) merged into a single map.
pub(crate) fn build_flat(
    source: &Source,
    meta_end: usize,
    ts: &'static TransferSyntax,
    stop_after: TagKey,
    whitelist: Option<&[TagKey]>,
    disable_tracing: bool,
) -> Result<DataSet> {
    let parser = Parser { buf: &source.data, master: &source.data, mapped: source.mapped, whitelist, stop_after, disable_tracing, ambiguous: Cell::new(false) };
    let dec = Decoder::from_ts(ts);
    let mut map = parser.dataset(META_START, meta_end, META_LE)?;
    // Main data set tags all sort after the (0002,xxxx) meta tags.
    for (key, el) in parser.dataset(meta_end, source.data.len(), dec)?.into_entries() {
        map.push_parsed(key, el);
    }
    if parser.ambiguous.get() {
        parser.resolve_ambiguous(&mut map, dec, &AmbiguityCtx::default());
    }
    assemble(source, map, ts, DatasetKind::Dataset)
}

/// Whether `data` begins with a DICOM File Meta preamble + `DICM` magic.
pub(crate) fn has_dicm(data: &[u8]) -> bool {
    data.len() >= META_START && &data[PREAMBLE_LEN..META_START] == b"DICM"
}

/// Heuristic transfer syntax for a headerless stream: assume Little Endian and
/// pick Explicit VR if the bytes after the first tag look like a VR.
pub(crate) fn detect_transfer_syntax(data: &[u8]) -> &'static TransferSyntax {
    if data.len() >= 6 && is_vr_code([data[4], data[5]]) {
        &TransferSyntax::ExplicitVRLittleEndian
    } else {
        &TransferSyntax::ImplicitVRLittleEndian
    }
}

/// Parses the File Meta Information (group 0002): builds the header data set,
/// and returns the offset where the main data set begins plus its transfer
/// syntax (from (0002,0010)).
pub(crate) fn meta(source: &Source, disable_tracing: bool) -> Result<(DataSet, usize, &'static TransferSyntax)> {
    let buf = &source.data;
    ensure!(has_dicm(buf), InvalidData, "missing DICM File Meta preamble");

    let parser = Parser { buf, master: &source.data, mapped: source.mapped, whitelist: None, stop_after: TagKey(u32::MAX), disable_tracing, ambiguous: Cell::new(false) };

    // Prefer the File Meta group length (0002,0000) when present and sane;
    // otherwise scan group 0002 to find where the main data set begins. Many
    // older/converted files omit the group length element entirely.
    let meta_end = match parser.read_header(META_START, META_LE) {
        Ok(gl) if gl.tag == tags::FileMetaInformationGroupLength.key && gl.length.0 == 4 => {
            let group_length = read_u32(buf, gl.value_start, true) as usize;
            let end = gl.value_start + 4 + group_length;
            if end <= buf.len() {
                end
            } else {
                parser.note(format_args!("File Meta group length exceeds the buffer; scanning group 0002"));
                parser.scan_group_two()
            }
        }
        _ => {
            parser.note(format_args!("File Meta has no (0002,0000) group length; scanning group 0002"));
            parser.scan_group_two()
        }
    };

    let header = assemble(
        source,
        parser.dataset(META_START, meta_end, META_LE)?,
        &TransferSyntax::ExplicitVRLittleEndian,
        DatasetKind::MetaInfo,
    )?;
    let ts = match header.get::<String>(&tags::TransferSyntaxUID) {
        Ok(uid) => TransferSyntax::from_uid(uid.trim()).unwrap_or_else(|| {
            parser.note(format_args!("unknown TransferSyntaxUID (0002,0010) {:?}; assuming Explicit VR LE", uid.trim()));
            &TransferSyntax::ExplicitVRLittleEndian
        }),
        Err(_) => {
            parser.note(format_args!("missing TransferSyntaxUID (0002,0010); detecting from the data set"));
            detect_transfer_syntax(&buf[meta_end..])
        }
    };
    Ok((header, meta_end, ts))
}
