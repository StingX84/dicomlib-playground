//! Configurable [`DcmWriter`] facade over the sans-io [`Serializer`].

use std::io::Write;

use bytes::Bytes;
use flate2::{Compression, write::DeflateEncoder};

use dpx_dicom_core::error::{IntoDicomErr, Result};
use dpx_dicom_core::tags;

use dpx_dicom_core::TransferSyntax;

use super::core::Serializer;
use crate::DataSet;

const PREAMBLE: [u8; 128] = [0u8; 128];

/// Serializes a [`DataSet`] back to the DICOM binary stream form.
///
/// The target transfer syntax controls byte order, Explicit/Implicit VR and
/// deflation; raw values stored in a different byte order are transcoded, others
/// pass through unchanged. The File Meta header (when writing a full file) is
/// always Explicit VR Little Endian and its group length is recomputed; its
/// (0002,0010) Transfer Syntax UID is written verbatim from the supplied header,
/// so transcoding to a different syntax means updating that element first.
#[derive(Debug, Clone)]
pub struct DcmWriter {
    xfer: &'static TransferSyntax,
    undefined_sq: bool,
}

impl Default for DcmWriter {
    fn default() -> Self {
        Self { xfer: &TransferSyntax::ExplicitVRLittleEndian, undefined_sq: true }
    }
}

impl DcmWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Target transfer syntax for the data set body (default: Explicit VR LE).
    pub fn transfer_syntax(mut self, ts: &'static TransferSyntax) -> Self {
        self.xfer = ts;
        self
    }

    /// Whether sequences and items use undefined length with delimiters
    /// (default `true`, faster — no length backpatch) or computed defined length.
    pub fn undefined_sequences(mut self, undefined: bool) -> Self {
        self.undefined_sq = undefined;
        self
    }

    /// Transfer syntax used for the body bytes (deflation is applied by the
    /// stream wrapper, so the inner framing is plain Explicit VR LE).
    fn body_ts(&self) -> &'static TransferSyntax {
        if self.xfer.is_compressed { &TransferSyntax::ExplicitVRLittleEndian } else { self.xfer }
    }

    /// Writes the data set body (no File Meta header) to `w`.
    pub fn write_dataset<W: Write>(&self, ds: &DataSet, w: W) -> Result<()> {
        self.write_body(ds, w)
    }

    /// Writes a full file: 128-byte preamble, `DICM`, the File Meta header (from
    /// `header`, group length recomputed) and the data set body.
    pub fn write_file<W: Write>(&self, header: &DataSet, ds: &DataSet, mut w: W) -> Result<()> {
        let (meta_shared, meta_root) = header.context();
        let mut meta_body = Vec::new();
        Serializer::new(&mut meta_body, meta_shared, &TransferSyntax::ExplicitVRLittleEndian, false)
            .elements_skipping(meta_root, tags::FileMetaInformationGroupLength.key)?;

        let io = |r: std::io::Result<()>| r.to_dicom_err_with(|| "writing File Meta header".to_string());
        io(w.write_all(&PREAMBLE))?;
        io(w.write_all(b"DICM"))?;
        // (0002,0000) UL group length, Explicit VR LE.
        io(w.write_all(&[0x02, 0x00, 0x00, 0x00]))?;
        io(w.write_all(b"UL"))?;
        io(w.write_all(&4u16.to_le_bytes()))?;
        io(w.write_all(&(meta_body.len() as u32).to_le_bytes()))?;
        io(w.write_all(&meta_body))?;

        self.write_body(ds, w)
    }

    /// Serializes the data set body to bytes (no File Meta header).
    pub fn to_bytes(&self, ds: &DataSet) -> Result<Bytes> {
        let mut buf = Vec::new();
        self.write_body(ds, &mut buf)?;
        Ok(Bytes::from(buf))
    }

    /// Serializes a full file (preamble + meta + body) to bytes.
    pub fn to_file_bytes(&self, header: &DataSet, ds: &DataSet) -> Result<Bytes> {
        let mut buf = Vec::new();
        self.write_file(header, ds, &mut buf)?;
        Ok(Bytes::from(buf))
    }

    fn write_body<W: Write>(&self, ds: &DataSet, w: W) -> Result<()> {
        let (shared, root) = ds.context();
        let ts = self.body_ts();
        if self.xfer.is_compressed {
            let mut enc = DeflateEncoder::new(w, Compression::default());
            Serializer::new(&mut enc, shared, ts, self.undefined_sq).root(root)?;
            enc.finish().to_dicom_err_with(|| "deflating data set".to_string())?;
            Ok(())
        } else {
            Serializer::new(w, shared, ts, self.undefined_sq).root(root)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DcmReader, HeaderType};
    use dpx_dicom_core::tags;

    /// One Explicit VR LE element; `vr` decides short vs long form.
    fn el(buf: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], val: &[u8]) {
        buf.extend_from_slice(&g.to_le_bytes());
        buf.extend_from_slice(&e.to_le_bytes());
        buf.extend_from_slice(vr);
        let long = matches!(vr, b"OB" | b"OW" | b"SQ" | b"UN" | b"UT" | b"UC" | b"UR" | b"OF" | b"OD" | b"OL" | b"OV" | b"SV" | b"UV");
        if long {
            buf.extend_from_slice(&[0, 0]);
            buf.extend_from_slice(&(val.len() as u32).to_le_bytes());
        } else {
            buf.extend_from_slice(&(val.len() as u16).to_le_bytes());
        }
        buf.extend_from_slice(val);
    }

    fn sample() -> Vec<u8> {
        let mut b = Vec::new();
        el(&mut b, 0x0010, 0x0010, b"PN", b"Doe^John");
        el(&mut b, 0x0010, 0x0020, b"LO", b"ID-1\0"); // odd -> padded by builder? keep even
        el(&mut b, 0x0028, 0x0010, b"US", &512u16.to_le_bytes());
        b
    }

    fn read(bytes: Bytes, ts: &'static TransferSyntax) -> DataSet {
        DcmReader::new()
            .header(HeaderType::NoHeader)
            .transfer_syntax(ts)
            .parse_bytes(bytes)
            .expect("read")
            .dataset
            .expect("dataset")
    }

    fn read_le(bytes: Bytes) -> DataSet {
        read(bytes, &TransferSyntax::ExplicitVRLittleEndian)
    }

    #[test]
    fn roundtrip_body() {
        let ds = read_le(Bytes::from(sample()));
        let out = DcmWriter::new().to_bytes(&ds).expect("write");
        let ds2 = read_le(out);
        assert_eq!(ds2.get::<String>(&tags::PatientName).unwrap(), "Doe^John");
        assert_eq!(ds2.get::<u16>(&tags::Rows).unwrap(), 512);
    }

    #[test]
    fn roundtrip_full_file_preserves_transfer_syntax() {
        // Build a file with a File Meta header (Explicit VR LE).
        let ts_uid = b"1.2.840.10008.1.2.1\0";
        let mut meta = Vec::new();
        el(&mut meta, 0x0002, 0x0010, b"UI", ts_uid);
        let mut file = vec![0u8; 128];
        file.extend_from_slice(b"DICM");
        file.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]);
        file.extend_from_slice(b"UL");
        file.extend_from_slice(&4u16.to_le_bytes());
        file.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        file.extend_from_slice(&meta);
        file.extend_from_slice(&sample());

        let out = DcmReader::new().parse_bytes(Bytes::from(file)).expect("read");
        let (header, ds) = (out.header.unwrap(), out.dataset.unwrap());
        let written = DcmWriter::new().to_file_bytes(&header, &ds).expect("write");

        let out2 = DcmReader::new().parse_bytes(written).expect("reread");
        assert_eq!(
            out2.header.unwrap().get::<String>(&tags::TransferSyntaxUID).unwrap(),
            "1.2.840.10008.1.2.1"
        );
        let ds2 = out2.dataset.unwrap();
        assert_eq!(ds2.get::<String>(&tags::PatientName).unwrap(), "Doe^John");
        assert_eq!(ds2.get::<u16>(&tags::Rows).unwrap(), 512);
    }

    fn sample_with_sequence() -> Vec<u8> {
        let mut item = Vec::new();
        el(&mut item, 0x0020, 0x000E, b"UI", b"1.2\0");
        let mut sq = Vec::new();
        sq.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]); // Item
        sq.extend_from_slice(&(item.len() as u32).to_le_bytes());
        sq.extend_from_slice(&item);
        let mut b = Vec::new();
        el(&mut b, 0x0008, 0x1115, b"SQ", &sq); // ReferencedSeriesSequence, defined length
        b
    }

    fn check_sequence_roundtrip(undefined: bool) {
        let ds = read_le(Bytes::from(sample_with_sequence()));
        let out = DcmWriter::new().undefined_sequences(undefined).to_bytes(&ds).expect("write");
        let ds2 = read_le(out);
        let seq = ds2.sequence(&tags::ReferencedSeriesSequence).expect("seq");
        assert_eq!(seq.len(), 1);
        assert_eq!(seq.item(0).unwrap().get::<String>(&tags::SeriesInstanceUID).unwrap(), "1.2");
    }

    #[test]
    fn roundtrip_sequence_undefined() {
        check_sequence_roundtrip(true);
    }

    #[test]
    fn roundtrip_sequence_defined() {
        check_sequence_roundtrip(false);
    }

    #[test]
    fn roundtrip_encapsulated_pixeldata() {
        let mut b = Vec::new();
        b.extend_from_slice(&[0xE0, 0x7F, 0x10, 0x00]); // (7FE0,0010) OB undefined
        b.extend_from_slice(b"OB");
        b.extend_from_slice(&[0, 0]);
        b.extend_from_slice(&u32::MAX.to_le_bytes());
        b.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]); // empty BOT
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]); // fragment
        b.extend_from_slice(&4u32.to_le_bytes());
        b.extend_from_slice(b"ABCD");
        b.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0]); // SeqDelim
        b.extend_from_slice(&0u32.to_le_bytes());

        let ds = read_le(Bytes::from(b));
        let out = DcmWriter::new().to_bytes(&ds).expect("write");
        let ds2 = read_le(out);
        match ds2.value(&tags::PixelData).unwrap() {
            crate::Value::Pixels(px) => match *px {
                crate::PixelData::Encapsulated { fragments, .. } => {
                    assert_eq!(fragments.len(), 1);
                    assert_eq!(&fragments[0][..], b"ABCD");
                }
                _ => panic!("expected encapsulated"),
            },
            _ => panic!("expected pixels"),
        }
    }

    #[test]
    fn roundtrip_deflated() {
        let ds = read_le(Bytes::from(sample()));
        // Need a File Meta header to carry the deflated TS UID.
        let mut meta = Vec::new();
        el(&mut meta, 0x0002, 0x0010, b"UI", b"1.2.840.10008.1.2.1.99\0");
        let mut file = vec![0u8; 128];
        file.extend_from_slice(b"DICM");
        file.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]);
        file.extend_from_slice(b"UL");
        file.extend_from_slice(&4u16.to_le_bytes());
        file.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        file.extend_from_slice(&meta);
        let header = DcmReader::new()
            .mode(crate::ReadMode::HeaderOnly)
            .parse_bytes(Bytes::from(file))
            .expect("meta")
            .header
            .unwrap();

        let written = DcmWriter::new()
            .transfer_syntax(&TransferSyntax::DeflatedExplicitVRLittleEndian)
            .to_file_bytes(&header, &ds)
            .expect("write deflated");

        let out = DcmReader::new().parse_bytes(written).expect("reread");
        let ds2 = out.dataset.unwrap();
        assert_eq!(ds2.get::<String>(&tags::PatientName).unwrap(), "Doe^John");
        assert_eq!(ds2.get::<u16>(&tags::Rows).unwrap(), 512);
    }

    /// `encode` coerces a value's logical type into the target VR: string into a
    /// binary integer/float, float into a decimal/integer string, etc.
    #[test]
    fn encode_coerces_across_types() {
        let mut ds = DataSet::new();
        ds.set(&tags::Rows, "512").unwrap(); // Str -> US (binary u16)
        ds.set(&tags::SliceThickness, 2.5_f64).unwrap(); // Float -> DS (decimal text)
        ds.set(&tags::InstanceNumber, 7.0_f64).unwrap(); // Float -> IS (integer text)
        ds.set(&tags::DiffusionBValue, "1.25").unwrap(); // Str -> FD (binary f64)
        ds.set(&tags::ExaminedBodyThickness, "3").unwrap(); // Str -> FL (binary f32)

        let bytes = DcmWriter::new().to_bytes(&ds).unwrap();
        let ds2 = read_le(bytes);
        assert_eq!(ds2.get::<u16>(&tags::Rows).unwrap(), 512);
        assert!((ds2.get::<f64>(&tags::SliceThickness).unwrap() - 2.5).abs() < 1e-9);
        assert_eq!(ds2.get::<i32>(&tags::InstanceNumber).unwrap(), 7);
        assert!((ds2.get::<f64>(&tags::DiffusionBValue).unwrap() - 1.25).abs() < 1e-9);
        assert!((ds2.get::<f32>(&tags::ExaminedBodyThickness).unwrap() - 3.0).abs() < 1e-6);
    }

    #[test]
    fn transcode_little_to_big_endian() {
        let ds = read_le(Bytes::from(sample())); // stored LE
        let out = DcmWriter::new()
            .transfer_syntax(&TransferSyntax::ExplicitVRBigEndian)
            .to_bytes(&ds)
            .expect("write BE");
        // Rows (US, 512=0x0200) must be byte-swapped on the wire.
        let ds2 = read(out, &TransferSyntax::ExplicitVRBigEndian);
        assert!(!ds2.is_little_endian());
        assert_eq!(ds2.get::<u16>(&tags::Rows).unwrap(), 512);
        assert_eq!(ds2.get::<String>(&tags::PatientName).unwrap(), "Doe^John");
    }
}
