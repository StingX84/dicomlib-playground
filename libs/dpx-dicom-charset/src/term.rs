use crate::tables::{multi_byte::*, single_byte::*, *};
use std::fmt::Debug;

// cSpell:ignore hanja

/// Enumeration of all the terms supported internally to this library.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Term {
    /// Unknown encoding. Read and write bytes unchanged.
    ///
    /// When decoding byte-stream into String, one-to-one conversion applied.\
    /// Example: ```b"\x00\xEE"``` decoded to ```"\u0000\u00EE"```
    ///
    /// When encoding String into byte-stream, one-to-one conversion applied
    /// except characters greater than `\uFF`, which are replaced with `?`. \
    /// Example: ```"\u0000\u00EE\u0100"``` encoded to ```b"\x00\xEE?"```
    Unknown,

    // --------------------------- DEFAULT ------------------------------
    /// `ISO_IR 6` 7bit Latin1, single-byte, no code extensions
    IsoIr6,

    // ---------------- Single-Byte Without Code Extensions -------------
    /// `ISO_IR 100`, `ISO-8859-1` Latin 1 (ISO 8859-1), single-byte, no code extensions
    IsoIr100,
    /// `ISO_IR 101`, `ISO-8859-2` Latin 2 (ISO 8859-2), single-byte, no code extensions
    IsoIr101,
    /// `ISO_IR 109`, `ISO-8859-3` Latin 3 (ISO 8859-3), single-byte, no code extensions
    IsoIr109,
    /// `ISO_IR 110`, `ISO-8859-4` Latin 4 (ISO 8859-4), single-byte, no code extensions
    IsoIr110,
    /// `ISO_IR 144`, `ISO-8859-5` Cyrillic (ISO 8859-5), single-byte, no code extensions
    IsoIr144,
    /// `ISO_IR 127`, `ISO-8859-6` Arabic (ISO 8859-6), single-byte, no code extensions
    IsoIr127,
    /// `ISO_IR 126`, `ISO-8859-7` Greek (ISO 8859-7), single-byte, no code extensions
    IsoIr126,
    /// `ISO_IR 138`, `ISO-8859-8` Hebrew (ISO 8859-8), single-byte, no code extensions
    IsoIr138,
    /// `ISO_IR 148`, `ISO-8859-9` Latin 5/Turkish (ISO 8859-9), single-byte, no code extensions
    IsoIr148,
    /// `ISO_IR 203`, `ISO-8859-15` Latin 9/Latin 0 (ISO 8859-15), single-byte, no code extensions
    IsoIr203,
    /// `ISO_IR 13`, Japanese (JIS X 0201), single-byte, no code extensions
    IsoIr13,
    /// `ISO_IR 166`, `TIS-620`, `ISO-8859-11` Thai (TIS 620-2533), single-byte, no code extensions
    IsoIr166,

    // ----------------- Single-Byte With Code Extensions ---------------
    /// `ISO 2022 IR 6` 7bit Latin1, single-byte, with code extensions
    Iso2022Ir6,
    /// `ISO 2022 IR 100` Latin 1 (ISO 8859-1), single-byte, with code extensions
    Iso2022Ir100,
    /// `ISO 2022 IR 101` Latin 2 (ISO 8859-2), single-byte, with code extensions
    Iso2022Ir101,
    /// `ISO 2022 IR 109` Latin 3 (ISO 8859-3), single-byte, with code extensions
    Iso2022Ir109,
    /// `ISO 2022 IR 110` Latin 4 (ISO 8859-4), single-byte, with code extensions
    Iso2022Ir110,
    /// `ISO 2022 IR 144` Cyrillic (ISO 8859-5), single-byte, with code extensions
    Iso2022Ir144,
    /// `ISO 2022 IR 127` Arabic (ISO 8859-6), single-byte, with code extensions
    Iso2022Ir127,
    /// `ISO 2022 IR 126` Greek (ISO 8859-7), single-byte, with code extensions
    Iso2022Ir126,
    /// `ISO 2022 IR 138` Hebrew (ISO 8859-8), single-byte, with code extensions
    Iso2022Ir138,
    /// `ISO 2022 IR 148` Latin 5/Turkish (ISO 8859-9), single-byte, with code extensions
    Iso2022Ir148,
    /// `ISO 2022 IR 203` Latin 9/Latin 0 (ISO 8859-15), single-byte, with code extensions
    Iso2022Ir203,
    /// `ISO 2022 IR 13` Japanese (JIS X 0201), single-byte, with code extensions
    Iso2022Ir13,
    /// `ISO 2022 IR 166` Thai (TIS 620-2533), single-byte, with code extensions
    Iso2022Ir166,

    // ----------------- Multi-Byte With Code Extensions ---------------
    /// `ISO 2022 IR 87` Japanese Kanji (JIS X 0208), multi-byte, with code extensions
    Iso2022Ir87,
    /// `ISO 2022 IR 159`, Japanese Supplementary Kanji (JIS X 0208), multi-byte, with code extensions
    Iso2022Ir159,
    /// `ISO 2022 IR 149`, Korean Hangul & Hanja (KS X 1001), multi-byte, with code extensions
    Iso2022Ir149,
    /// `ISO 2022 IR 58` GB 2312-80 China Association for Standardization, multi-byte, with code extensions
    Iso2022Ir58,

    // ----------------- Multi-Byte Without Code Extensions ---------------
    /// `ISO_IR 192`, `UTF-8`, `UTF8` Unicode (ISO 646 in UTF-8), variable byte, no code extensions
    IsoIr192,
    /// `GB18030` Chinese (GB18030), variable byte, no code extensions
    Gb18030,
    /// `GBK`, `GB2312` Chinese (GBK), variable byte, no code extensions
    Gbk,

    // ----------------- Non-standard encodings ---------------
    /// **NON-DICOM**  `cp1250`, `windows-1250` MS Latin 2/ Central European, single-byte, no code extensions
    NonDicomCp1250,
    /// **NON-DICOM** `cp1251`, `windows-1251` MS Cyrillic, single-byte, no code extensions
    NonDicomCp1251,
    /// **NON-DICOM** `cp1252`, `windows-1252` MS Latin 1 / Western European, single-byte, no code extensions
    NonDicomCp1252,
    /// **NON-DICOM** `cp1253`, `windows-1253` MS Greek, single-byte, no code extensions
    NonDicomCp1253,
    /// **NON-DICOM** `cp1254`, `windows-1254` MS Turkish, single-byte, no code extensions
    NonDicomCp1254,
    /// **NON-DICOM** `cp1255`, `windows-1255` MS Hebrew, single-byte, no code extensions
    NonDicomCp1255,
    /// **NON-DICOM** `cp1256`, `windows-1256` MS Arabic, single-byte, no code extensions
    NonDicomCp1256,
    /// **NON-DICOM** `cp1257`, `windows-1257` MS Baltic, single-byte, no code extensions
    NonDicomCp1257,
    /// **NON-DICOM** `cp1258`, `windows-1258` MS Vietnamese, single-byte, no code extensions
    NonDicomCp1258,
    /// **NON-DICOM** `cp866`, `ibm-866` MS-DOS Cyrillic Russian, no code extensions
    NonDicomIbm866,
    /// **NON-DICOM** `KOI8-R`, `KOI8` Russian, no code extensions
    NonDicomKoi8R,
}

const LAST_VALID_DICOM: u8 = Term::Gbk as u8;
const MAX_TERM: usize = Term::NonDicomKoi8R as usize;

#[derive(Clone)]
pub(crate) enum CodecType {
    /// Dicom standard single-byte without extensions\
    /// Tuple of:
    /// - 0 - [Term] describing the same encoding with extensions enabled
    ///   ([Iso2022](Self::Iso2022)).
    Iso2022NoExtensions(Term),

    /// ISO-2022 compatible single or multi-byte with extensions\
    /// Tuple of:
    /// - 1 - `G0` table.
    /// - 2 - `G1` table.
    Iso2022WithExtensions(&'static Table, &'static Table),

    /// Standard
    /// [MultiByteWithoutCodeExtensions](crate::TermKind::MultiByteWithoutCodeExtensions)
    /// or non-standard encoding.
    NonIso2022(PfnForward, PfnBackward),

    /// Special case for UTF-8. Uses highly optimized verification function from rust core.
    Utf8,
}

/// A structure, that contains information on [Term]
///
/// You can obtain this calling method [Term::meta()]
pub struct TermMeta {
    /// [Term] this meta corresponds to
    pub term: Term,
    /// Keyword(s) the [Term] can be referred to from (0008,0005) Specific
    /// Character Set attribute
    ///
    /// For standard encodings, this member contains a Standard defined Term in
    /// a first element and optional aliases in other elements.
    ///
    /// For non-standard encodings, all elements are aliases.
    pub keywords: &'static [&'static str],
    /// Human readable description
    ///
    /// For standard encodings, this field contains standard-defined `Character Set Description`
    ///
    /// For non-standard encodings, this field contains some meaningful text.
    pub description: &'static str,
    /// The codec implementation type and it's data
    pub(crate) mode: CodecType,
    /// Kind of this encoding according to the Standard.
    pub kind: TermKind,
    /// Flag that is set for [Term]'s compatible with 7-bit ASCII.
    pub is_ascii_compatible: bool,
}

/// Enumeration containing the [Term] `kind`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TermKind {
    /// Standard term listed in [PS3.3 \"Table C.12-2. Defined Terms for
    /// Single-Byte Character Sets Without Code Extensions\"](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.12.html#&table_C.12-2)
    SingleByteWithoutCodeExtensions,
    /// Standard term listed in [PS3.3 \"C.12-3. Defined Terms for Single-Byte
    ///     Character Sets with Code Extensions\"](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.12.html#&table_C.12-3)
    SingleByteWithCodeExtensions,
    /// Standard term listed in [PS3.3 \"C.12-4. Defined Terms for Multi-Byte
    ///     Character Sets with Code Extensions\"](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.12.html#&table_C.12-4)
    MultiByteWithCodeExtensions,
    /// Standard term listed in [PS3.3 \"C.12-5. Defined Terms for Multi-Byte
    ///     Character Sets Without Code Extensions\"](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.12.html#&table_C.12-5)
    MultiByteWithoutCodeExtensions,
    /// Non standard terms which sometimes used by "buggy" or encoding unaware software
    NonStandard,
}

#[rustfmt::skip]
static ALL_TERMS: [TermMeta; MAX_TERM + 1] = [
    // Unknown (identity)
    TermMeta { // Default repertoire
        term: Term::Unknown, keywords: &[""],
        description: "Unknown encoding",
        mode: CodecType::NonIso2022(forward_identity, backward_identity),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    // STANDARD (DICOM PS3.3-2022e Table C.12-2. Defined Terms for Single-Byte Character Sets Without Code Extensions)
    TermMeta { // Default repertoire
        term: Term::IsoIr6, keywords: &["ISO_IR 6"],
        description: "Default repertoire",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir6, ),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta { // latin 1
        term: Term::IsoIr100, keywords: &["ISO_IR 100", "ISO-8859-1"],
        description: "Latin alphabet No. 1",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir100),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 2
        term: Term::IsoIr101, keywords: &["ISO_IR 101", "ISO-8859-2"],
        description: "Latin alphabet No. 2",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir101),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 3
        term: Term::IsoIr109, keywords: &["ISO_IR 109", "ISO-8859-3"],
        description: "Latin alphabet No. 3",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir109),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 4
        term: Term::IsoIr110,keywords: &["ISO_IR 110", "ISO-8859-4"],
        description: "Latin alphabet No. 4",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir110),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // cyrillic
        term: Term::IsoIr144, keywords: &["ISO_IR 144", "ISO-8859-5"],
        description: "Cyrillic",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir144),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // arabic
        term: Term::IsoIr127, keywords: &["ISO_IR 127", "ISO-8859-6"],
        description: "Arabic",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir127),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // greek (1986)
        term: Term::IsoIr126, keywords: &["ISO_IR 126", "ISO-8859-7"],
        description: "Greek",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir126),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // hebrew
        term: Term::IsoIr138, keywords: &["ISO_IR 138", "ISO-8859-8"],
        description: "Hebrew",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir138),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 5 (turkish)
        term: Term::IsoIr148, keywords: &["ISO_IR 148", "ISO-8859-9"],
        description: "Latin alphabet No. 5",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir148),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // Latin 9
        term: Term::IsoIr203, keywords: &["ISO_IR 203", "ISO-8859-15"],
        description: "Latin alphabet No. 9",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir203),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // japanese JIS X 0201 (Katakana)
        term: Term::IsoIr13, keywords: &["ISO_IR 13"],
        description: "Japanese Katakana",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir13),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: false,
    },
    TermMeta{ // thai TIS 620-2533
        term: Term::IsoIr166, keywords: &["ISO_IR 166", "TIS-620", "ISO-8859-11"],
        description: "Thai",
        mode: CodecType::Iso2022NoExtensions(Term::Iso2022Ir166),
        kind: TermKind::SingleByteWithoutCodeExtensions, is_ascii_compatible: true,
    },

    // STANDARD (DICOM PS 3.3-2022e Table C.12-3. Defined Terms for Single-Byte Character Sets with Code Extensions)
    TermMeta{  // default
        term: Term::Iso2022Ir6, keywords: &["ISO 2022 IR 6"],
        description: "Default repertoire",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ALWAYS_INVALID),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 1
        term: Term::Iso2022Ir100, keywords: &["ISO 2022 IR 100"],
        description: "Latin alphabet No. 1",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_100),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 2
        term: Term::Iso2022Ir101, keywords: &["ISO 2022 IR 101"],
        description: "Latin alphabet No. 2",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_101),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 3
        term: Term::Iso2022Ir109, keywords: &["ISO 2022 IR 109"],
        description: "Latin alphabet No. 3",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_109),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{  // latin 4
        term: Term::Iso2022Ir110, keywords: &["ISO 2022 IR 110"],
        description: "Latin alphabet No. 4",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_110),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{  // cyrillic
        term: Term::Iso2022Ir144, keywords: &["ISO 2022 IR 144"],
        description: "Cyrillic",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_144),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // arabic
        term: Term::Iso2022Ir127, keywords: &["ISO 2022 IR 127"],
        description: "Arabic",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_127),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // greek
        term: Term::Iso2022Ir126, keywords: &["ISO 2022 IR 126"],
        description: "Greek",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_126),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // hebrew
        term: Term::Iso2022Ir138, keywords: &["ISO 2022 IR 138"],
        description: "Hebrew",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_138),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // latin 5 (turkish)
        term: Term::Iso2022Ir148, keywords: &["ISO 2022 IR 148"],
        description: "Latin alphabet No. 5",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_148),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // Latin 9
        term: Term::Iso2022Ir203, keywords: &["ISO 2022 IR 203"],
        description: "Latin alphabet No. 9",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_203),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{  // japanese JIS X 0201 (katakana)
        term: Term::Iso2022Ir13, keywords: &["ISO 2022 IR 13"],
        description: "Japanese Katakana",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_14, &TABLE_G1_ISO_IR_13),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: false,
    },
    TermMeta{ // thai TIS 620-2533
        term: Term::Iso2022Ir166, keywords: &["ISO 2022 IR 166"],
        description: "Thai",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_6, &TABLE_G1_ISO_IR_166),
        kind: TermKind::SingleByteWithCodeExtensions, is_ascii_compatible: true,
    },

    // STANDARD (DICOM PS 3.3-2022e Table C.12-4. Defined Terms for Multi-Byte Character Sets with Code Extensions)
    TermMeta{ // Japanese JIS X 0208: Kanji
        term: Term::Iso2022Ir87, keywords: &["ISO 2022 IR 87"],
        description: "Japanese Kanji",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_87, &TABLE_G1_ALWAYS_INVALID),
        kind: TermKind::MultiByteWithCodeExtensions, is_ascii_compatible: false,
    },
    TermMeta{  // Japanese JIS X 0212: Supplementary Kanji set
        term: Term::Iso2022Ir159, keywords: &["ISO 2022 IR 159"],
        description: "Japanese Sup. Kanji",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ISO_IR_159, &TABLE_G1_ALWAYS_INVALID),
        kind: TermKind::MultiByteWithCodeExtensions, is_ascii_compatible: false,
    },
    TermMeta{ // Korean KS X 1001: Hangul and Hanja
        term: Term::Iso2022Ir149, keywords: &["ISO 2022 IR 149"],
        description: "Korean",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ALWAYS_INVALID, &TABLE_G1_ISO_IR_149),
        kind: TermKind::MultiByteWithCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // GB 2312-80 China Association for Standardization
        term: Term::Iso2022Ir58, keywords: &["ISO 2022 IR 58"],
        description: "Simplified Chinese",
        mode: CodecType::Iso2022WithExtensions(&TABLE_G0_ALWAYS_INVALID, &TABLE_G1_ISO_IR_58),
        kind: TermKind::MultiByteWithCodeExtensions, is_ascii_compatible: true,
    },

    // STANDARD (DICOM PS 3.3-2022e Table C.12-5. Defined Terms for Multi-Byte Character Sets Without Code Extensions)
    TermMeta{ // Unicode in UTF-8
        term: Term::IsoIr192, keywords: &["ISO_IR 192", "UTF-8", "UTF8"],
        description: "Unicode in UTF-8",
        mode: CodecType::Utf8,
        kind: TermKind::MultiByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // Chinese GB18030
        term: Term::Gb18030, keywords: &["GB18030"],
        description: "GB18030",
        mode: CodecType::NonIso2022(forward_gb18030, backward_gb18030),
        kind: TermKind::MultiByteWithoutCodeExtensions, is_ascii_compatible: true,
    },
    TermMeta{ // Chinese GBK
        term: Term::Gbk, keywords: &["GBK", "GB2312"],
        description: "GBK",
        mode: CodecType::NonIso2022(forward_gbk, backward_gbk),
        kind: TermKind::MultiByteWithoutCodeExtensions, is_ascii_compatible: true,
    },

    // NON-STANDARD SINGLE-BYTE ENCODINGS WITHOUT CODE EXTENSIONS
    TermMeta{ // MS windows-1250
        term: Term::NonDicomCp1250, keywords: &["cp1250", "windows-1250"],
        description: "Non-standard MS Central European",
        mode: CodecType::NonIso2022(forward_cp_1250, backward_cp_1250),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1251
        term: Term::NonDicomCp1251, keywords: &["cp1251", "windows-1251"],
        description: "Non-standard MS Cyrillic",
        mode: CodecType::NonIso2022(forward_cp_1251, backward_cp_1251),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1252
        term: Term::NonDicomCp1252, keywords: &["cp1252", "windows-1252"],
        description: "Non-standard MS Western European",
        mode: CodecType::NonIso2022(forward_cp_1252, backward_cp_1252),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1253
        term: Term::NonDicomCp1253, keywords: &["cp1253", "windows-1253"],
        description: "Non-standard MS Greek",
        mode: CodecType::NonIso2022(forward_cp_1253, backward_cp_1253),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1254
        term: Term::NonDicomCp1254, keywords: &["cp1254", "windows-1254"],
        description: "Non-standard MS Turkish",
        mode: CodecType::NonIso2022(forward_cp_1254, backward_cp_1254),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1255
        term: Term::NonDicomCp1255, keywords: &["cp1255", "windows-1255"],
        description: "Non-standard MS Hebrew",
        mode: CodecType::NonIso2022(forward_cp_1255, backward_cp_1255),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1256
        term: Term::NonDicomCp1256, keywords: &["cp1256", "windows-1256"],
        description: "Non-standard MS Arabic",
        mode: CodecType::NonIso2022(forward_cp_1256, backward_cp_1256),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1257
        term: Term::NonDicomCp1257, keywords: &["cp1257", "windows-1257"],
        description: "Non-standard MS Baltic",
        mode: CodecType::NonIso2022(forward_cp_1257, backward_cp_1257),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS windows-1258
        term: Term::NonDicomCp1258, keywords: &["cp1258", "windows-1258"],
        description: "Non-standard MS Vietnamese",
        mode: CodecType::NonIso2022(forward_cp_1258, backward_cp_1258),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // MS-DOS CP-866
        term: Term::NonDicomIbm866, keywords: &["cp866", "ibm-866"],
        description: "Non-standard MS-DOS Cyrillic",
        mode: CodecType::NonIso2022(forward_cp_866, backward_cp_866),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
    TermMeta{ // KOI8-R
        term: Term::NonDicomKoi8R, keywords: &["KOI8-R", "KOI8"],
        description: "Non-standard Russian",
        mode: CodecType::NonIso2022(forward_koi8_r, backward_koi8_r),
        kind: TermKind::NonStandard, is_ascii_compatible: true,
    },
];

/// A type of match and the matched keyword returned from [Term::search_by_keyword]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermMatchedWith {
    /// The searched string was exactly matched to the first keyword of a [Term]
    Primary,
    /// The searched string was case-insensitively matched to the first keyword of a [Term]
    PrimaryICase,
    /// The searched string was case-insensitively matched to some other than the first keywords of a [Term]
    Alias,
    /// The searched string was fuzzily matched to some keyword of a [Term]
    Fuzzy,
}

impl Term {
    /// Returns a slice of [TermMeta] about all known encoding terms
    ///
    /// Example:
    /// ```no_run
    /// use dpx_dicom_charset::Term;
    /// println!("Supported encodings:");
    /// for meta in Term::all().iter() {
    ///     println!("{}", meta.keywords[0]);
    /// }
    /// ```
    pub fn all() -> &'static [TermMeta] {
        &ALL_TERMS
    }

    /// Returns [TermMeta] structure for this code
    pub fn meta(self) -> &'static TermMeta {
        &Self::all()[self as usize]
    }

    /// Returns a DICOM term for this code
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_charset::Term;
    /// assert_eq!(Term::IsoIr144.keywords()[0], "ISO_IR 144");
    /// ```
    pub fn keywords(self) -> &'static [&'static str] {
        Self::meta(self).keywords
    }

    /// Returns a description for this code
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_charset::Term;
    /// assert_eq!(Term::IsoIr144.description(), "Cyrillic");
    /// ```
    pub fn description(self) -> &'static str {
        Self::meta(self).description
    }

    /// Returns `true` if this code represents a term defined in [PS3.3] standard.
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_charset::Term;
    /// assert_eq!(Term::IsoIr144.is_standard_dicom(), true);
    /// assert_eq!(Term::NonDicomIbm866.is_standard_dicom(), false);
    /// ```
    ///
    /// [PS3.3]:
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.12.1.1.2
    ///     "PS3.3 \"C.12.1.1.2 Specific Character Set\""
    pub const fn is_standard_dicom(self) -> bool {
        self as u8 <= LAST_VALID_DICOM
    }

    pub fn kind(self) -> TermKind {
        Self::meta(self).kind
    }

    /// Returns `Some` [Term] for the specified numeric code or `None` if
    /// provided number is not a valid code.
    pub fn from_u8(code: u8) -> Option<Term> {
        if code <= MAX_TERM as u8 {
            return Some(Self::all()[code as usize].term);
        }
        None
    }

    /// Returns `Some` [Term] for the specified keyword or `None` if not
    /// found.
    ///
    /// Search is case insensitive. See [search_by_keyword](Self::from_keyword) for more
    /// search options.
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_charset::Term;
    /// assert_eq!(Term::from_keyword(b"ISO_IR 126").unwrap(), Term::IsoIr126);
    /// assert_eq!(Term::from_keyword(b"gb18030").unwrap(), Term::Gb18030);
    /// assert!(Term::from_keyword(b"some unknown").is_none());
    /// ```
    pub fn from_keyword(keyword: &[u8]) -> Option<Term> {
        Self::all().iter().find_map(|e| {
            e.keywords
                .iter()
                .find(|t| t.as_bytes().eq_ignore_ascii_case(keyword))
                .map(|_| e.term)
        })
    }

    /// Searches for [Term] keyword and returns `Some` tuple of [Term] and the
    /// matched keyword or `None` if match was not found.
    ///
    /// This method performs a search using two different approaches:
    /// - The case-insensitive comparison same as [from_keyword](Self::from_keyword)
    /// - The "fuzzy" comparison, which is also case-insensitive, but ignores
    ///   white spaces, '_' and '-' symbols in both searched string and Term
    ///   keyword being searched.
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_charset::{Term, TermMatchedWith};
    /// // Match exactly DICOM term
    /// assert_eq!(Term::search_by_keyword(b"ISO_IR 126").unwrap(), (Term::IsoIr126, TermMatchedWith::Primary));
    /// // Match to a DICOM term, but case characters case may differ
    /// assert_eq!(Term::search_by_keyword(b"gb18030").unwrap(), (Term::Gb18030, TermMatchedWith::PrimaryICase));
    /// // Match to an alias of DICOM term.
    /// assert_eq!(Term::search_by_keyword(b"ISO-8859-1").unwrap(), (Term::IsoIr100, TermMatchedWith::Alias));
    /// // In case of aliases, case does not matters
    /// assert_eq!(Term::search_by_keyword(b"iso-8859-1").unwrap(), (Term::IsoIr100, TermMatchedWith::Alias));
    /// // We can even find this encoding with some weird spaces
    /// assert_eq!(Term::search_by_keyword(b"iso  8859_1").unwrap(), (Term::IsoIr100, TermMatchedWith::Fuzzy));
    /// // .. or without spaces at all
    /// assert_eq!(Term::search_by_keyword(b"Iso88591").unwrap(), (Term::IsoIr100, TermMatchedWith::Fuzzy));
    /// // Unknown encodings still could not be found
    /// assert!(Term::search_by_keyword(b"some unknown").is_none());
    /// ```
    pub fn search_by_keyword(keyword: &[u8]) -> Option<(Term, TermMatchedWith)> {
        // Search only "first" terms
        let primary_match = Self::all().iter().find_map(|e| {
            if e.keywords[0].as_bytes() == keyword {
                return Some((e.term, TermMatchedWith::Primary));
            }
            if e.keywords[0].as_bytes().eq_ignore_ascii_case(keyword) {
                return Some((e.term, TermMatchedWith::PrimaryICase));
            }
            None
        });
        if primary_match.is_some() {
            return primary_match;
        }

        let alias_match = Self::all().iter().find_map(|e| {
            e.keywords
                .iter()
                .skip(1)
                .find(|&&t| t.as_bytes().eq_ignore_ascii_case(keyword))
                .map(|_| (e.term, TermMatchedWith::Alias))
        });
        if alias_match.is_some() {
            return alias_match;
        }

        fn fuzzy_compare(l: &[u8], r: &[u8]) -> bool {
            let l_filtered = l
                .iter()
                .filter(|&&c| c != b' ' && c != b'_' && c != b'-')
                .map(|&c| c.to_ascii_lowercase());
            let r_filtered = r
                .iter()
                .filter(|&&c| c != b' ' && c != b'_' && c != b'-')
                .map(|&c| c.to_ascii_lowercase());

            l_filtered.eq(r_filtered)
        }

        let fuzzy_match = Self::all().iter().find_map(|e| {
            e.keywords
                .iter()
                .find(|&&t| fuzzy_compare(t.as_bytes(), keyword))
                .map(|_| (e.term, TermMatchedWith::Fuzzy))
        });

        fuzzy_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_all_terms_listed_in_order_of_definition() {
        let all_terms = Term::all();
        for code in 0..=MAX_TERM {
            assert_eq!(all_terms[code].term, Term::from_u8(code as u8).unwrap());
        }
    }
}
