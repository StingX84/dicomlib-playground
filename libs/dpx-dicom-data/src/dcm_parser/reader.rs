use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use bytes::Bytes;
use flate2::read::DeflateDecoder;

use dpx_dicom_core::error::{IntoDicomErr, Result};
use dpx_dicom_core::{TagKey, TransferSyntax, ensure};

use super::core;
use super::input::Source;
use crate::DataSet;
use crate::dataset::DatasetKind;

/// Whether a DICOM File Meta header is expected ahead of the data set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeaderType {
    /// Detect a header heuristically (preamble + `DICM`).
    #[default]
    Auto,
    /// No header; the stream is a bare data set.
    NoHeader,
    /// A header is required.
    WithHeader,
}

/// How the header and data set are returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadMode {
    /// Header and data set as two separate data sets.
    #[default]
    Normal,
    /// Only the header (invalid when [`HeaderType::NoHeader`]).
    HeaderOnly,
    /// Header and data set merged into one; `header` is always `None`. Useful
    /// for mass scanning with a `tag_whitelist`.
    Flat,
}

/// Result of a read: the file meta header and/or the data set, per [`ReadMode`].
pub struct ReadOutput {
    /// The header data set. `None` if absent or `mode == Flat`.
    pub header: Option<DataSet>,
    /// The data set. `None` if reading stopped at the header or `mode == HeaderOnly`.
    pub dataset: Option<DataSet>,
}

/// Configurable DICOM stream reader. Set parameters with the builder methods,
/// then call one of the `parse_*` entry points.
#[derive(Debug, Clone, Default)]
pub struct DcmReader {
    xfer: Option<&'static TransferSyntax>,
    header: HeaderType,
    mode: ReadMode,
    tag_max: Option<TagKey>,
    /// Sorted, de-duplicated whitelist; only these tags are kept.
    tag_whitelist: Option<Vec<TagKey>>,
}

impl DcmReader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Use a known transfer syntax instead of detecting it.
    pub fn transfer_syntax(mut self, ts: &'static TransferSyntax) -> Self {
        self.xfer = Some(ts);
        self
    }
    pub fn header(mut self, header: HeaderType) -> Self {
        self.header = header;
        self
    }
    pub fn mode(mut self, mode: ReadMode) -> Self {
        self.mode = mode;
        self
    }
    /// Stop parsing once a tag greater than `tag` is reached.
    pub fn tag_max(mut self, tag: TagKey) -> Self {
        self.tag_max = Some(tag);
        self
    }
    /// Keep only these tags; parsing also stops past the largest of them.
    pub fn tag_whitelist(mut self, mut tags: Vec<TagKey>) -> Self {
        tags.sort_by_key(|k| k.0);
        tags.dedup();
        self.tag_whitelist = Some(tags);
        self
    }

    // --- Entry points ------------------------------------------------------

    /// Parses an in-memory buffer (zero-copy: the buffer becomes the master).
    pub fn parse_bytes(&self, data: Bytes) -> Result<ReadOutput> {
        self.run(Source::from_bytes(data, true))
    }

    /// Reads fully from any reader.
    pub fn parse_bufreader<R: Read>(&self, reader: R) -> Result<ReadOutput> {
        let source = Source::from_reader(reader).to_dicom_err_with(|| "reading stream".to_string())?;
        self.run(source)
    }

    /// Opens a file and reads it through a buffered reader (no mmap).
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<ReadOutput> {
        let path = path.as_ref();
        let file = File::open(path).to_dicom_err_with(|| format!("opening {}", path.display()))?;
        self.parse_bufreader(BufReader::new(file))
    }

    /// Memory-maps a file, falling back to [`parse_file`](Self::parse_file) when
    /// mapping is unavailable.
    pub fn parse_mmap(&self, path: impl AsRef<Path>) -> Result<ReadOutput> {
        let path = path.as_ref();
        match Source::mmap(path) {
            Ok(source) => self.run(source),
            Err(_) => self.parse_file(path),
        }
    }

    // --- Orchestration -----------------------------------------------------

    /// Effective stop tag: the *smaller* of `tag_max` and the whitelist
    /// maximum (either alone already bounds the walk). `TagKey::MAX` means
    /// "no limit" so the parser branches on a plain compare, not an `Option`.
    fn stop_after(&self) -> TagKey {
        let wl_max = self.tag_whitelist.as_ref().and_then(|v| v.last().copied());
        match (self.tag_max, wl_max) {
            (Some(a), Some(b)) => TagKey(a.0.min(b.0)),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => TagKey(u32::MAX),
        }
    }

    fn run(&self, source: Source) -> Result<ReadOutput> {
        let has_header = match self.header {
            HeaderType::NoHeader => false,
            HeaderType::Auto => core::has_dicm(&source.data),
            HeaderType::WithHeader => {
                ensure!(core::has_dicm(&source.data), InvalidData, "expected a DICOM File Meta header (DICM)");
                true
            }
        };

        let stop = self.stop_after();
        let wl = self.tag_whitelist.as_deref();

        if has_header {
            if let ReadMode::HeaderOnly = self.mode {
                let (header, _, _) = core::meta(&source, false)?;
                return Ok(ReadOutput { header: Some(header), dataset: None });
            }
            let (header, dataset_start, meta_ts) = core::meta(&source, false)?;
            let ts = self.xfer.unwrap_or(meta_ts);
            // For a Deflated transfer syntax the body is inflated into its own
            // buffer (with the uncompressed meta prefix kept in place) so the
            // parser core sees a plain Explicit VR LE data set.
            let (inflated, body_ts) = inflate_if_deflated(&source, dataset_start, ts)?;
            let body = inflated.as_ref().unwrap_or(&source);
            let body_end = body.data.len();
            return match self.mode {
                ReadMode::Normal => {
                    let dataset =
                        core::build_dataset(body, dataset_start, body_end, body_ts, DatasetKind::Dataset, stop, wl, false)?;
                    Ok(ReadOutput { header: Some(header), dataset: Some(dataset) })
                }
                ReadMode::Flat => {
                    let dataset = core::build_flat(body, dataset_start, body_ts, stop, wl, false)?;
                    Ok(ReadOutput { header: None, dataset: Some(dataset) })
                }
                ReadMode::HeaderOnly => unreachable!("handled above"),
            };
        }

        match self.mode {
            ReadMode::HeaderOnly => {
                ensure!(
                    !matches!(self.header, HeaderType::NoHeader),
                    InvalidData,
                    "HeaderOnly mode requires a header"
                );
                // Auto with no DICM: a valid bare data set was found heuristically.
                Ok(ReadOutput { header: None, dataset: None })
            }
            ReadMode::Normal | ReadMode::Flat => {
                let ts = self.xfer.unwrap_or_else(|| core::detect_transfer_syntax(&source.data));
                let (inflated, body_ts) = inflate_if_deflated(&source, 0, ts)?;
                let body = inflated.as_ref().unwrap_or(&source);
                let dataset =
                    core::build_dataset(body, 0, body.data.len(), body_ts, DatasetKind::Dataset, stop, wl, false)?;
                Ok(ReadOutput { header: None, dataset: Some(dataset) })
            }
        }
    }
}

/// If `ts` is a Deflated transfer syntax, inflates the raw-DEFLATE body at
/// `[body_start..]` into a fresh buffer that keeps the uncompressed `[..body_start]`
/// prefix (the File Meta header) in place, so callers can parse it at the same
/// offsets as a plain Explicit VR LE stream. Returns `(None, ts)` unchanged when
/// not deflated. The streaming [`DeflateDecoder`] reads from the inner slice,
/// leaving room for a future bytes-read progress callback wrapped around it.
fn inflate_if_deflated(
    source: &Source,
    body_start: usize,
    ts: &'static TransferSyntax,
) -> Result<(Option<Source>, &'static TransferSyntax)> {
    if !ts.is_compressed {
        return Ok((None, ts));
    }
    let mut combined = source.data[..body_start].to_vec();
    DeflateDecoder::new(&source.data[body_start..])
        .read_to_end(&mut combined)
        .to_dicom_err_with(|| "inflating deflated data set".to_string())?;
    let inflated = Source::from_bytes(Bytes::from(combined), true);
    Ok((Some(inflated), &TransferSyntax::ExplicitVRLittleEndian))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{PixelData, Value};
    use dpx_dicom_core::{Tag, TagKey, tags};

    fn sample() -> Bytes {
        let mut buf = Vec::new();
        // PatientName (0010,0010) PN "Doe^John"
        buf.extend_from_slice(&[0x10, 0x00, 0x10, 0x00]);
        buf.extend_from_slice(b"PN");
        buf.extend_from_slice(&8u16.to_le_bytes());
        buf.extend_from_slice(b"Doe^John");
        // Rows (0028,0010) US 512
        buf.extend_from_slice(&[0x28, 0x00, 0x10, 0x00]);
        buf.extend_from_slice(b"US");
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&512u16.to_le_bytes());
        Bytes::from(buf)
    }

    #[test]
    fn read_headerless_dataset() {
        let out = DcmReader::new().parse_bytes(sample()).expect("read");
        assert!(out.header.is_none());
        let ds = out.dataset.expect("dataset");
        assert_eq!(ds.get::<String>(&tags::PatientName).expect("PN"), "Doe^John");
        assert_eq!(ds.get::<u16>(&tags::Rows).expect("Rows"), 512);
    }

    #[test]
    fn whitelist_filters_and_stops() {
        let rows = tags::Rows.key;
        let out = DcmReader::new().tag_whitelist(vec![rows]).parse_bytes(sample()).expect("read");
        let ds = out.dataset.expect("dataset");
        assert_eq!(ds.len(), 1);
        assert!(!ds.contains(&tags::PatientName));
        assert_eq!(ds.get::<u16>(&tags::Rows).expect("Rows"), 512);
    }

    #[test]
    fn header_required_but_absent_errors() {
        let out = DcmReader::new().header(HeaderType::WithHeader).parse_bytes(sample());
        assert!(out.is_err());
    }

    /// Builds a minimal valid file: 128-byte preamble, `DICM`, File Meta group
    /// (group length + Explicit VR LE transfer syntax), then the data set.
    fn sample_with_header() -> Bytes {
        let ts_uid = b"1.2.840.10008.1.2.1\0"; // padded to even length (20)
        let mut meta_body = Vec::new();
        meta_body.extend_from_slice(&[0x02, 0x00, 0x10, 0x00]); // (0002,0010)
        meta_body.extend_from_slice(b"UI");
        meta_body.extend_from_slice(&(ts_uid.len() as u16).to_le_bytes());
        meta_body.extend_from_slice(ts_uid);

        let mut buf = vec![0u8; 128];
        buf.extend_from_slice(b"DICM");
        // (0002,0000) UL group length
        buf.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]);
        buf.extend_from_slice(b"UL");
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(&(meta_body.len() as u32).to_le_bytes());
        buf.extend_from_slice(&meta_body);
        // main data set (Explicit VR LE)
        buf.extend_from_slice(&sample());
        Bytes::from(buf)
    }

    /// Like `sample_with_header` but declares the Deflated Explicit VR LE
    /// transfer syntax and raw-DEFLATE compresses the main data set.
    fn sample_deflated() -> Bytes {
        use flate2::{Compression, write::DeflateEncoder};
        use std::io::Write;

        let ts_uid = b"1.2.840.10008.1.2.1.99\0"; // 24 bytes, even
        let mut meta_body = Vec::new();
        meta_body.extend_from_slice(&[0x02, 0x00, 0x10, 0x00]);
        meta_body.extend_from_slice(b"UI");
        meta_body.extend_from_slice(&(ts_uid.len() as u16).to_le_bytes());
        meta_body.extend_from_slice(ts_uid);

        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&sample()).unwrap();
        let deflated = enc.finish().unwrap();

        let mut buf = vec![0u8; 128];
        buf.extend_from_slice(b"DICM");
        buf.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]); // (0002,0000) UL group length
        buf.extend_from_slice(b"UL");
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(&(meta_body.len() as u32).to_le_bytes());
        buf.extend_from_slice(&meta_body);
        buf.extend_from_slice(&deflated);
        Bytes::from(buf)
    }

    #[test]
    fn read_deflated_dataset() {
        let out = DcmReader::new().parse_bytes(sample_deflated()).expect("read");
        assert_eq!(
            out.header.expect("header").get::<String>(&tags::TransferSyntaxUID).expect("TS"),
            "1.2.840.10008.1.2.1.99"
        );
        let ds = out.dataset.expect("dataset");
        assert_eq!(ds.get::<String>(&tags::PatientName).expect("PN"), "Doe^John");
        assert_eq!(ds.get::<u16>(&tags::Rows).expect("Rows"), 512);
    }

    #[test]
    fn read_deflated_flat() {
        let out = DcmReader::new().mode(ReadMode::Flat).parse_bytes(sample_deflated()).expect("read");
        let ds = out.dataset.expect("dataset");
        assert!(ds.contains(&tags::TransferSyntaxUID)); // uncompressed meta tag
        assert!(ds.contains(&tags::PatientName)); // inflated data-set tag
    }

    #[test]
    fn read_with_file_meta_header() {
        let out = DcmReader::new().parse_bytes(sample_with_header()).expect("read");
        let header = out.header.expect("header");
        assert_eq!(header.kind(), DatasetKind::MetaInfo);
        assert_eq!(
            header.get::<String>(&tags::TransferSyntaxUID).expect("TS uid"),
            "1.2.840.10008.1.2.1"
        );
        let ds = out.dataset.expect("dataset");
        assert_eq!(ds.get::<String>(&tags::PatientName).expect("PN"), "Doe^John");
        assert_eq!(ds.get::<u16>(&tags::Rows).expect("Rows"), 512);
    }

    /// Validation against a real file (e.g. from gdcmdata). Path via env var so
    /// no external data is committed. Run: `DCM_TEST_FILE=path cargo test -p
    /// dpx-dicom-data --features static_dictionary -- --ignored`.
    #[test]
    #[ignore = "requires a local DICOM file via DCM_TEST_FILE"]
    fn scan_real_file_sop_instance_uid() {
        let path = std::env::var("DCM_TEST_FILE").expect("set DCM_TEST_FILE");
        let sop = tags::SOPInstanceUID.key;
        let out = DcmReader::new().tag_whitelist(vec![sop]).parse_mmap(&path).expect("scan");
        let uid = out.dataset.expect("dataset").get::<String>(&Tag::new(sop, None)).expect("SOPInstanceUID");
        assert!(!uid.is_empty());
        eprintln!("SOPInstanceUID = {uid}");

        // Full Normal read (exercises sequences end-to-end).
        let out = DcmReader::new().parse_mmap(&path).expect("full read");
        let ds = out.dataset.expect("dataset");
        assert!(!ds.is_empty());
        eprintln!("header={} dataset attrs={}", out.header.is_some(), ds.len());
    }

    #[test]
    fn flat_mode_merges_header_and_dataset() {
        let out = DcmReader::new().mode(ReadMode::Flat).parse_bytes(sample_with_header()).expect("read");
        assert!(out.header.is_none());
        let ds = out.dataset.expect("dataset");
        assert!(ds.contains(&tags::TransferSyntaxUID)); // meta tag present
        assert!(ds.contains(&tags::PatientName)); // dataset tag present
    }

    /// A headerless Explicit VR LE data set with a single sequence
    /// (ReferencedSeriesSequence) holding one item with a SeriesInstanceUID.
    fn sample_sequence(undefined: bool) -> Bytes {
        let mut content = Vec::new(); // the item's nested data set
        content.extend_from_slice(&[0x20, 0x00, 0x0E, 0x00]); // (0020,000E) SeriesInstanceUID
        content.extend_from_slice(b"UI");
        content.extend_from_slice(&4u16.to_le_bytes());
        content.extend_from_slice(b"1.2\0");

        let mut items = Vec::new();
        items.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]); // (FFFE,E000) Item
        if undefined {
            items.extend_from_slice(&u32::MAX.to_le_bytes());
            items.extend_from_slice(&content);
            items.extend_from_slice(&[0xFE, 0xFF, 0x0D, 0xE0]); // (FFFE,E00D) Item Delimitation
            items.extend_from_slice(&0u32.to_le_bytes());
        } else {
            items.extend_from_slice(&(content.len() as u32).to_le_bytes());
            items.extend_from_slice(&content);
        }

        let mut buf = Vec::new();
        buf.extend_from_slice(&[0x08, 0x00, 0x15, 0x11]); // (0008,1115) ReferencedSeriesSequence
        buf.extend_from_slice(b"SQ");
        buf.extend_from_slice(&[0, 0]); // reserved
        if undefined {
            buf.extend_from_slice(&u32::MAX.to_le_bytes());
            buf.extend_from_slice(&items);
            buf.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0]); // (FFFE,E0DD) Sequence Delimitation
            buf.extend_from_slice(&0u32.to_le_bytes());
        } else {
            buf.extend_from_slice(&(items.len() as u32).to_le_bytes());
            buf.extend_from_slice(&items);
        }
        Bytes::from(buf)
    }

    fn check_sequence(undefined: bool) {
        let out = DcmReader::new().parse_bytes(sample_sequence(undefined)).expect("read");
        let ds = out.dataset.expect("dataset");
        let seq = ds.sequence(&tags::ReferencedSeriesSequence).expect("sequence");
        assert_eq!(seq.len(), 1);
        let item = seq.item(0).expect("item 0");
        assert_eq!(item.get::<String>(&tags::SeriesInstanceUID).expect("uid"), "1.2");
    }

    #[test]
    fn defined_length_sequence() {
        check_sequence(false);
    }

    #[test]
    fn undefined_length_sequence() {
        check_sequence(true);
    }

    /// A headerless Explicit VR LE data set with encapsulated PixelData
    /// (7FE0,0010) OB, undefined length: an empty Basic Offset Table followed by
    /// two fragments, terminated by a Sequence Delimitation Item.
    fn sample_encapsulated() -> Bytes {
        let mut buf = Vec::new();
        // (7FE0,0010) OB, reserved, undefined length
        buf.extend_from_slice(&[0xE0, 0x7F, 0x10, 0x00]);
        buf.extend_from_slice(b"OB");
        buf.extend_from_slice(&[0, 0]);
        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        // Empty Basic Offset Table item
        buf.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]);
        buf.extend_from_slice(&0u32.to_le_bytes());
        // Fragment 1: "ABCD"
        buf.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]);
        buf.extend_from_slice(&4u32.to_le_bytes());
        buf.extend_from_slice(b"ABCD");
        // Fragment 2: "EF"
        buf.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]);
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(b"EF");
        // Sequence Delimitation Item
        buf.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0]);
        buf.extend_from_slice(&0u32.to_le_bytes());
        Bytes::from(buf)
    }

    #[test]
    fn encapsulated_pixel_data() {
        let out = DcmReader::new().parse_bytes(sample_encapsulated()).expect("read");
        let ds = out.dataset.expect("dataset");
        match ds.value(&tags::PixelData).expect("pixel data") {
            Value::Pixels(px) => match *px {
                PixelData::Encapsulated { bot, fragments } => {
                    assert!(bot.is_empty());
                    assert_eq!(fragments.len(), 2);
                    assert_eq!(&fragments[0][..], b"ABCD");
                    assert_eq!(&fragments[1][..], b"EF");
                }
                PixelData::Native(_) => panic!("expected encapsulated pixel data"),
            },
            other => panic!("expected Value::Pixels, got {other:?}"),
        }

        #[cfg(feature = "file_offsets")]
        {
            // element + empty BOT + 2 fragments + Sequence Delimitation Item
            let hdr = ds.headers(&tags::PixelData);
            assert_eq!(hdr.len(), 5);
            // pixel-data element
            assert_eq!(hdr[0].offset, 0);
            assert_eq!(hdr[0].tag, tags::PixelData.key);
            assert_eq!(hdr[0].vr, Some(*b"OB"));
            assert_eq!(hdr[0].length, (u32::MAX, 4));
            // empty Basic Offset Table item
            assert_eq!(hdr[1].tag, tags::Item.key);
            assert_eq!(hdr[1].offset, 12);
            assert_eq!(hdr[1].size, Some(0));
            // fragment 1 ("ABCD")
            assert_eq!(hdr[2].tag, tags::Item.key);
            assert_eq!(hdr[2].offset, 20);
            assert_eq!(hdr[2].size, Some(4));
            // fragment 2 ("EF")
            assert_eq!(hdr[3].tag, tags::Item.key);
            assert_eq!(hdr[3].offset, 32);
            assert_eq!(hdr[3].size, Some(2));
            // Sequence Delimitation Item
            assert_eq!(hdr[4].tag, tags::SequenceDelimitationItem.key);
            assert_eq!(hdr[4].offset, 42);
            assert_eq!(hdr[4].size, None);
        }
    }

    /// An undefined-length element with a non-SQ VR (here a private tag read as
    /// UN) must be parsed as a sequence, not as encapsulated pixel data.
    #[test]
    fn undefined_length_un_parses_as_sequence() {
        let private = TagKey::new(0x0009, 0x0010);
        let mut buf = Vec::new();
        // (0009,0010) UN, reserved, undefined length
        buf.extend_from_slice(&[0x09, 0x00, 0x10, 0x00]);
        buf.extend_from_slice(b"UN");
        buf.extend_from_slice(&[0, 0]);
        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        // one undefined-length item holding a SOPInstanceUID
        buf.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]);
        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        buf.extend_from_slice(&[0x08, 0x00, 0x18, 0x00]); // (0008,0018) UI
        buf.extend_from_slice(b"UI");
        buf.extend_from_slice(&4u16.to_le_bytes());
        buf.extend_from_slice(b"1.2\0");
        buf.extend_from_slice(&[0xFE, 0xFF, 0x0D, 0xE0]); // Item Delimitation
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0]); // Sequence Delimitation
        buf.extend_from_slice(&0u32.to_le_bytes());

        let out = DcmReader::new().parse_bytes(Bytes::from(buf)).expect("read");
        let ds = out.dataset.expect("dataset");
        let seq = ds.sequence(&Tag::new(private, None)).expect("sequence");
        assert_eq!(seq.len(), 1);
        assert_eq!(seq.item(0).expect("item").get::<String>(&tags::SOPInstanceUID).expect("uid"), "1.2");
    }

    /// Out-of-order tags must not panic; lookups still succeed afterwards.
    #[test]
    fn out_of_order_tags_recovered() {
        let mut buf = Vec::new();
        // Rows (0028,0010) first — higher tag
        buf.extend_from_slice(&[0x28, 0x00, 0x10, 0x00]);
        buf.extend_from_slice(b"US");
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&512u16.to_le_bytes());
        // PatientName (0010,0010) second — lower tag
        buf.extend_from_slice(&[0x10, 0x00, 0x10, 0x00]);
        buf.extend_from_slice(b"PN");
        buf.extend_from_slice(&8u16.to_le_bytes());
        buf.extend_from_slice(b"Doe^John");

        let out = DcmReader::new().parse_bytes(Bytes::from(buf)).expect("read");
        let ds = out.dataset.expect("dataset");
        assert_eq!(ds.len(), 2);
        assert_eq!(ds.get::<String>(&tags::PatientName).expect("PN"), "Doe^John");
        assert_eq!(ds.get::<u16>(&tags::Rows).expect("Rows"), 512);
    }

    /// Builds a headerless Implicit VR LE data set: optional Bits Allocated /
    /// Pixel Representation context, then native PixelData and a PixelPaddingValue.
    fn implicit_with_context(bits_allocated: Option<u16>, pixel_rep: Option<u16>) -> Bytes {
        let mut buf = Vec::new();
        let elem = |buf: &mut Vec<u8>, g: u16, e: u16, val: &[u8]| {
            buf.extend_from_slice(&g.to_le_bytes());
            buf.extend_from_slice(&e.to_le_bytes());
            buf.extend_from_slice(&(val.len() as u32).to_le_bytes());
            buf.extend_from_slice(val);
        };
        if let Some(b) = bits_allocated {
            elem(&mut buf, 0x0028, 0x0100, &b.to_le_bytes()); // BitsAllocated
        }
        if let Some(r) = pixel_rep {
            elem(&mut buf, 0x0028, 0x0103, &r.to_le_bytes()); // PixelRepresentation
        }
        elem(&mut buf, 0x0028, 0x0120, &[0, 0]); // PixelPaddingValue (US or SS)
        elem(&mut buf, 0x7FE0, 0x0010, b"ABCD"); // PixelData native (OB or OW)
        Bytes::from(buf)
    }

    fn read_implicit(bytes: Bytes) -> DataSet {
        DcmReader::new()
            .transfer_syntax(&TransferSyntax::ImplicitVRLittleEndian)
            .parse_bytes(bytes)
            .expect("read")
            .dataset
            .expect("dataset")
    }

    #[test]
    fn implicit_pixeldata_resolves_to_ow_when_16_bit() {
        let ds = read_implicit(implicit_with_context(Some(16), Some(0)));
        assert_eq!(ds.vr(&tags::PixelData), Some(dpx_dicom_core::Vr::OW));
        assert_eq!(ds.vr(&tags::PixelPaddingValue), Some(dpx_dicom_core::Vr::US));
    }

    #[test]
    fn implicit_pixeldata_resolves_to_ob_when_8_bit() {
        let ds = read_implicit(implicit_with_context(Some(8), Some(1)));
        assert_eq!(ds.vr(&tags::PixelData), Some(dpx_dicom_core::Vr::OB));
        assert_eq!(ds.vr(&tags::PixelPaddingValue), Some(dpx_dicom_core::Vr::SS));
    }

    #[test]
    fn implicit_ambiguous_defaults_when_context_absent() {
        // No Bits Allocated / Pixel Representation: fall back to OW / US.
        let ds = read_implicit(implicit_with_context(None, None));
        assert_eq!(ds.vr(&tags::PixelData), Some(dpx_dicom_core::Vr::OW));
        assert_eq!(ds.vr(&tags::PixelPaddingValue), Some(dpx_dicom_core::Vr::US));
    }

    /// A File Meta header lacking the (0002,0000) group length is read by
    /// scanning group 0002 up to the first non-0002 tag.
    #[test]
    fn file_meta_without_group_length() {
        let ts_uid = b"1.2.840.10008.1.2.1\0";
        let mut buf = vec![0u8; 128];
        buf.extend_from_slice(b"DICM");
        // (0002,0010) TransferSyntaxUID directly, no group length element
        buf.extend_from_slice(&[0x02, 0x00, 0x10, 0x00]);
        buf.extend_from_slice(b"UI");
        buf.extend_from_slice(&(ts_uid.len() as u16).to_le_bytes());
        buf.extend_from_slice(ts_uid);
        buf.extend_from_slice(&sample());

        let out = DcmReader::new().parse_bytes(Bytes::from(buf)).expect("read");
        assert_eq!(
            out.header.expect("header").get::<String>(&tags::TransferSyntaxUID).expect("TS"),
            "1.2.840.10008.1.2.1"
        );
        let ds = out.dataset.expect("dataset");
        assert_eq!(ds.get::<String>(&tags::PatientName).expect("PN"), "Doe^John");
        assert_eq!(ds.get::<u16>(&tags::Rows).expect("Rows"), 512);
    }
}
