use std::borrow::Cow;

use bytes::Bytes;
use dpx_dicom_core::error::Result;
use dpx_dicom_core::{DicomTimeZoneOffset, Tag, Vr, tags};
use dpx_dicom_charset::Codec;

use dpx_dicom_core::TransferSyntax;

use crate::convert::{FromValue, IntoValue};
use crate::item::{Item, read_accessors, write_accessors};
use crate::sequence::{Sequence, SequenceRef};
use crate::value::{TagHeader, Value};

/// What top-level data the [`DataSet`] represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DatasetKind {
    /// An ordinary data set.
    #[default]
    Dataset,
    /// A File Meta Information header (always Explicit VR Little Endian).
    MetaInfo,
}

/// Processing role of the data set. Affects DA/DT/TM handling (query matching).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DatasetRole {
    /// Ordinary storage data set.
    #[default]
    Storage,
    /// A Query/Retrieve identifier (C-FIND-RQ / C-MOVE-RQ / C-GET-RQ).
    QueryRetrieve,
}

/// Root-only context shared across the whole data set tree. Nested
/// [`Item`]s do not carry it; they are interpreted under their owning root.
pub(crate) struct Shared {
    kind: DatasetKind,
    /// Backing buffer for `Stored::Mapped` slices (a memory-mapped file, or
    /// empty for in-memory datasets). One reference count for the whole file.
    master: Bytes,
    /// Transfer syntax this data set was read under (or will default to). Only
    /// its endianness affects raw-value interpretation; the rest is informational.
    xfer: &'static TransferSyntax,
    role: DatasetRole,
    resolve_private: bool,
    /// Suppresses TRACE element-discovery events even when the subscriber is
    /// active. ponytail: a stub for the future `dicom.dataset.*` config option.
    disable_tracing: bool,
    /// Fallback timezone when (0008,0201) is absent (from configuration).
    default_tz: DicomTimeZoneOffset,
    /// Fallback charset when (0008,0005) is absent (from configuration).
    default_charset: Codec,
    /// Cache derived from (0008,0005). `None` falls back to `default_charset`.
    /// Kept in sync by mutators; rebuild via [`DataSet::sync_context`].
    charset: Option<Codec>,
    /// Cache derived from (0008,0201). `None` means the attribute is absent.
    root_tz: Option<DicomTimeZoneOffset>,
}

impl Shared {
    pub(crate) fn master(&self) -> &Bytes {
        &self.master
    }
    pub(crate) fn charset(&self) -> &Codec {
        self.charset.as_ref().unwrap_or(&self.default_charset)
    }
    pub(crate) fn is_little_endian(&self) -> bool {
        self.xfer.is_little_endian
    }
    pub(crate) fn xfer(&self) -> &'static TransferSyntax {
        self.xfer
    }
    pub(crate) fn role(&self) -> DatasetRole {
        self.role
    }
    /// Explicit (0008,0201) if present, else the configured default.
    pub(crate) fn effective_tz(&self) -> DicomTimeZoneOffset {
        self.root_tz.unwrap_or(self.default_tz)
    }
    pub(crate) fn default_tz(&self) -> DicomTimeZoneOffset {
        self.default_tz
    }
    /// Whether (0008,0201) was present in the data set.
    pub(crate) fn has_root_tz(&self) -> bool {
        self.root_tz.is_some()
    }
    /// Whether (0008,0005) has already established a charset.
    pub(crate) fn has_charset(&self) -> bool {
        self.charset.is_some()
    }
    pub(crate) fn default_charset(&self) -> &Codec {
        &self.default_charset
    }
    pub(crate) fn set_charset(&mut self, codec: Codec) {
        self.charset = Some(codec);
    }
}

/// An in-memory DICOM data set: root context plus the top-level attribute map.
pub struct DataSet {
    shared: Shared,
    root: Item,
}

impl Default for DataSet {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSet {
    /// Creates an empty data set with default context.
    ///
    // ponytail: defaults are hardcoded; once the `dicom.dataset.*` config keys
    // exist they will be pulled from a `Context` here.
    pub fn new() -> Self {
        Self {
            shared: Shared {
                kind: DatasetKind::Dataset,
                master: Bytes::new(),
                xfer: &TransferSyntax::ExplicitVRLittleEndian,
                role: DatasetRole::Storage,
                resolve_private: true,
                disable_tracing: false,
                default_tz: DicomTimeZoneOffset::Local,
                default_charset: Codec::new(),
                charset: None,
                root_tz: None,
            },
            root: Item::default(),
        }
    }

    /// Splits the data set into its context and root item (for adaptation).
    pub(crate) fn into_parts(self) -> (Shared, Item) {
        (self.shared, self.root)
    }

    /// Builds an empty data set whose context is seeded by the parser: a master
    /// buffer for `Mapped` slices, a byte order, and a kind.
    pub(crate) fn parsed(master: Bytes, xfer: &'static TransferSyntax, kind: DatasetKind) -> Self {
        let mut ds = Self::new();
        ds.shared.master = master;
        ds.shared.xfer = xfer;
        ds.shared.kind = kind;
        ds
    }

    /// Mutable access to the root item, for the parser to populate.
    pub(crate) fn root_mut(&mut self) -> &mut Item {
        &mut self.root
    }

    fn ctx(&self) -> (&Shared, &Item) {
        (&self.shared, &self.root)
    }
    /// Root context and attributes, for the serializer.
    pub(crate) fn context(&self) -> (&Shared, &Item) {
        (&self.shared, &self.root)
    }
    fn ctx_mut(&mut self) -> (&Shared, &mut Item) {
        (&self.shared, &mut self.root)
    }

    /// Auto-records (0008,0005) before a non-ASCII translatable text value is
    /// stored at the root, switching the data set to its configured
    /// `default_charset`. Once a charset is established (the attribute is
    /// present), this is inert. If the configured default is itself ASCII-only it
    /// cannot represent the text, so nothing is stamped (best effort).
    fn before_set(&mut self, tag: &Tag, value: &Value) {
        if self.shared.has_charset() {
            return;
        }
        let Value::Str(s) = value else { return };
        let translatable = matches!(
            crate::item::vr_for_write(tag).map(|vr| vr.info().kind),
            Ok(dpx_dicom_core::vr::Kind::Text { translatable: true, .. })
        );
        if !translatable {
            return;
        }
        let codec = self.shared.default_charset();
        // Plain-ASCII text under an ASCII-compatible codec is ISO_IR 6 and needs
        // no (0008,0005) declaration (same rule as `try_encode_ascii`). Anything
        // else needs the configured charset's repertoire.
        if s.is_empty() || (codec.is_ascii_compatible() && s.is_ascii()) {
            return;
        }
        let scs = codec.specific_character_set();
        // A default/unnamed codec (e.g. bare ISO_IR 6) has nothing to declare, so
        // the non-ASCII text is stored best-effort without recording a charset.
        if scs.is_empty() {
            return;
        }
        let codec = codec.clone();
        self.shared.set_charset(codec);
        self.root.map.insert(
            tags::SpecificCharacterSet.key,
            crate::value::Element::new(Vr::CS, crate::value::Stored::Native(Value::Str(scs))),
        );
    }

    read_accessors!();
    write_accessors!();

    /// Number of attributes at the top level.
    pub fn len(&self) -> usize {
        self.root.map.len()
    }
    /// Whether the top level has no attributes.
    pub fn is_empty(&self) -> bool {
        self.root.map.is_empty()
    }
    pub fn kind(&self) -> DatasetKind {
        self.shared.kind
    }
    /// Whether raw values are little-endian (per the transfer syntax read under).
    pub fn is_little_endian(&self) -> bool {
        self.shared.is_little_endian()
    }
    /// The transfer syntax this data set was read under (or its default).
    pub fn transfer_syntax(&self) -> &'static TransferSyntax {
        self.shared.xfer()
    }
    pub fn role(&self) -> DatasetRole {
        self.shared.role
    }
    pub fn set_role(&mut self, role: DatasetRole) {
        self.shared.role = role;
    }
    pub fn resolves_private_tags(&self) -> bool {
        self.shared.resolve_private
    }
    pub fn set_resolve_private_tags(&mut self, on: bool) {
        self.shared.resolve_private = on;
    }
    /// Resolved charset from (0008,0005), or the configured default.
    pub fn charset(&self) -> &Codec {
        self.shared.charset()
    }
    /// Parsed (0008,0201), or `None` if absent.
    pub fn root_timezone(&self) -> Option<DicomTimeZoneOffset> {
        self.shared.root_tz
    }
    /// Configured fallback timezone used when (0008,0201) is absent.
    pub fn default_timezone(&self) -> DicomTimeZoneOffset {
        self.shared.default_tz
    }
    pub fn set_default_timezone(&mut self, tz: DicomTimeZoneOffset) {
        self.shared.default_tz = tz;
    }
    /// Effective timezone: explicit (0008,0201) if present, else the default.
    pub fn timezone(&self) -> DicomTimeZoneOffset {
        self.shared.effective_tz()
    }

    /// Rebuilds the charset/timezone cache from the (0008,0005)/(0008,0201)
    /// attributes. Needed only after editing those through a low-level path
    /// that bypasses the maintaining mutators.
    pub fn sync_context(&mut self) -> Result<()> {
        let charset = match self.root.raw_bytes(&self.shared.master, tags::SpecificCharacterSet.key) {
            Some(bytes) if !bytes.is_empty() => {
                Some(Codec::from_specific_character_set(bytes, self.shared.default_charset.config().clone()))
            }
            _ => None,
        };
        let root_tz = match self.root.raw_bytes(&self.shared.master, tags::TimezoneOffsetFromUTC.key) {
            Some(bytes) => DicomTimeZoneOffset::from_dicom(bytes)?,
            None => None,
        };
        self.shared.charset = charset;
        self.shared.root_tz = root_tz;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Element, Stored};

    #[test]
    fn new_defaults() {
        let ds = DataSet::new();
        assert!(ds.is_empty());
        assert_eq!(ds.role(), DatasetRole::Storage);
        assert!(ds.resolves_private_tags());
        assert_eq!(ds.root_timezone(), None);
        assert_eq!(ds.timezone(), DicomTimeZoneOffset::Local);
    }

    #[test]
    fn role_and_private_toggles() {
        let mut ds = DataSet::new();
        ds.set_role(DatasetRole::QueryRetrieve);
        ds.set_resolve_private_tags(false);
        assert_eq!(ds.role(), DatasetRole::QueryRetrieve);
        assert!(!ds.resolves_private_tags());
    }

    #[test]
    fn sync_context_reads_timezone_attribute() {
        let mut ds = DataSet::new();
        ds.root.map.insert(
            tags::TimezoneOffsetFromUTC.key,
            Element::new(Vr::SH, Stored::Owned(Bytes::from_static(b"+0500"))),
        );
        ds.sync_context().expect("parse +0500");
        assert_eq!(ds.root_timezone(), Some(DicomTimeZoneOffset::Fixed(5 * 3600)));
        assert_eq!(ds.timezone(), DicomTimeZoneOffset::Fixed(5 * 3600));
    }

    #[test]
    fn sync_context_absent_timezone_falls_back_to_default() {
        let mut ds = DataSet::new();
        ds.set_default_timezone(DicomTimeZoneOffset::Fixed(0));
        ds.sync_context().expect("no tz attribute");
        assert_eq!(ds.root_timezone(), None);
        assert_eq!(ds.timezone(), DicomTimeZoneOffset::Fixed(0));
    }

    #[test]
    fn set_get_text_roundtrip() {
        let mut ds = DataSet::new();
        let tag = tags::PatientName; // PatientName (PN)
        ds.set(&tag, "Doe^John").expect("set PN");
        assert_eq!(ds.vr(&tag), Some(Vr::PN));
        assert_eq!(ds.get::<String>(&tag).expect("get PN"), "Doe^John");
        assert_eq!(&*ds.get_str(&tag).expect("str PN"), "Doe^John");
        assert!(ds.contains(&tag));
    }

    #[test]
    fn set_get_numeric_and_date() {
        use dpx_dicom_core::DicomDate;
        let mut ds = DataSet::new();
        let rows = tags::Rows; // Rows (US)
        ds.set(&rows, 512u16).expect("set Rows");
        assert_eq!(ds.vr(&rows), Some(Vr::US));
        assert_eq!(ds.get::<u16>(&rows).expect("get Rows"), 512);

        let study_date = tags::StudyDate; // StudyDate (DA)
        let date = DicomDate { y: Some(1993), m: Some(8), d: Some(22) };
        ds.set(&study_date, date).expect("set DA");
        assert_eq!(ds.get::<DicomDate>(&study_date).expect("get DA"), date);
    }

    #[test]
    fn unsigned_roundtrip_and_cross_read() {
        let mut ds = DataSet::new();
        let rows = tags::Rows; // Rows (US)
        ds.set(&rows, 60000u16).expect("set Rows");
        assert!(matches!(ds.value(&rows).expect("value"), Value::UInt(_)));
        assert_eq!(ds.get::<u16>(&rows).expect("u16"), 60000);
        assert_eq!(ds.get::<u64>(&rows).expect("u64"), 60000);
        assert_eq!(ds.get::<i32>(&rows).expect("i32"), 60000);
    }

    #[test]
    fn absent_attribute_semantics() {
        let ds = DataSet::new();
        let tag = tags::PatientName;
        assert!(ds.get::<String>(&tag).is_err());
        assert_eq!(ds.get_some::<String>(&tag), None);
        assert!(!ds.contains(&tag));
    }

    #[test]
    fn manual_attribute_has_no_headers() {
        let mut ds = DataSet::new();
        let tag = tags::PatientName;
        ds.set(&tag, "X").expect("set");
        assert!(ds.headers(&tag).is_empty());
        assert!(ds.item_headers().is_empty());
    }

    #[test]
    fn sequence_new_item_fill_and_read() {
        let mut ds = DataSet::new();
        let seq_tag = tags::ReferencedSeriesSequence; // ReferencedSeriesSequence (SQ)
        let uid = tags::SeriesInstanceUID; // SeriesInstanceUID (UI)
        {
            let mut seq = ds.sequence_mut(&seq_tag).expect("create SQ");
            let mut item = seq.new_item();
            item.set(&uid, "1.2.3.4").expect("set in item");
            assert_eq!(item.get::<String>(&uid).expect("read in item"), "1.2.3.4");
        }
        let seq = ds.sequence(&seq_tag).expect("read SQ");
        assert_eq!(seq.len(), 1);
        let item = seq.item(0).expect("item 0");
        assert_eq!(item.get::<String>(&uid).expect("read back"), "1.2.3.4");
    }

    #[test]
    fn set_ascii_text_does_not_stamp_charset() {
        let mut ds = DataSet::new();
        ds.set(&tags::PatientName, "Doe^John").expect("set PN");
        assert!(!ds.contains(&tags::SpecificCharacterSet));
    }

    #[test]
    fn set_non_ascii_text_stamps_charset() {
        // Fresh data set's default charset is UTF-8 (ISO_IR 192), which can
        // represent non-ASCII, so a non-ASCII PN must auto-stamp (0008,0005).
        let mut ds = DataSet::new();
        ds.set(&tags::PatientName, "Иванов^Иван").expect("set PN");
        assert_eq!(
            &*ds.get_str(&tags::SpecificCharacterSet).expect("scs present"),
            "ISO_IR 192"
        );
        assert_eq!(&*ds.get_str(&tags::PatientName).expect("pn"), "Иванов^Иван");

        // Round-trips through write + read with the stamped charset.
        let bytes = crate::DcmWriter::new().to_bytes(&ds).expect("write");
        let out = crate::DcmReader::new()
            .header(crate::HeaderType::NoHeader)
            .transfer_syntax(&crate::TransferSyntax::ExplicitVRLittleEndian)
            .mode(crate::ReadMode::Flat)
            .parse_bytes(bytes)
            .expect("read");
        let reread = out.dataset.expect("dataset");
        // Even-length padding adds a trailing space on the wire; trim for compare.
        assert_eq!(reread.get_str(&tags::PatientName).expect("reread pn").trim_end(), "Иванов^Иван");
    }

    #[test]
    fn get_iter_over_multivalued_numeric() {
        let mut ds = DataSet::new();
        // AcquisitionMatrix (US), VM 1-n.
        let tag = tags::AcquisitionMatrix;
        ds.set_value(&tag, Value::UInt(crate::value::OneOrMany::Many(vec![1, 2, 3]))).expect("set US[]");
        let got: Vec<u16> = ds.get_iter::<u16>(&tag).expect("iter").collect();
        assert_eq!(got, vec![1u16, 2, 3]);
    }

    #[test]
    fn get_iter_coerces_string_tokens() {
        let mut ds = DataSet::new();
        // IS (Integer String) stored as a multi-valued string.
        let tag = tags::InstanceNumber;
        ds.set_value(&tag, Value::Str("1\\2\\3".to_owned())).expect("set IS");
        let got: Vec<i32> = ds.get_iter::<i32>(&tag).expect("iter").collect();
        assert_eq!(got, vec![1, 2, 3]);
    }

    #[test]
    fn get_str_iter_borrows_tokens() {
        let mut ds = DataSet::new();
        // ImageType (CS), multi-valued.
        let tag = tags::ImageType;
        ds.set_value(&tag, Value::Str("ORIGINAL\\PRIMARY\\AXIAL".to_owned())).expect("set CS");
        let toks: Vec<String> = ds.get_str_iter(&tag).expect("iter").map(|t| t.to_owned()).collect();
        assert_eq!(toks, vec!["ORIGINAL", "PRIMARY", "AXIAL"]);
    }

    #[test]
    fn sequence_push_native_dataset_adapts() {
        let uid = tags::SeriesInstanceUID;
        let mut child = DataSet::new();
        child.set(&uid, "5.6.7.8").expect("set child");

        let mut parent = DataSet::new();
        let seq_tag = tags::ReferencedSeriesSequence;
        {
            let mut seq = parent.sequence_mut(&seq_tag).expect("create SQ");
            seq.push(child).expect("push native child");
        }
        let seq = parent.sequence(&seq_tag).expect("read SQ");
        assert_eq!(seq.iter().count(), 1);
        assert_eq!(seq.item(0).unwrap().get::<String>(&uid).expect("read"), "5.6.7.8");
    }
}
