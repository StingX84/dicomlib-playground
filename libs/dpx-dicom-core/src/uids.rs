//! Dictionary of UIDs defined in DICOM standard.
//!
//! Note: constant names in this module does not follow Rust naming
//! convention `UPPER_CASE_GLOBALS`, instead they are strictly following
//! DICOM standard "keyword" names. This eases interoperability with
//! different libraries as well as with DICOM standard itself.

#![allow(non_upper_case_globals)]

#[rustfmt::skip]

// cSpell:disable

/// [Application Contenxt](https://dicom.nema.org/medical/dicom/current/output/html/part07.html#chapter_A "PS3.7 \"A. Application Context Usage (Normative)\"")
pub mod app_context {
    pub const DICOMApplicationContext : &str = "1.2.840.10008.3.1.1.1";
}

/// [Transfer Syntax](https://dicom.nema.org/medical/dicom/current/output/html/part05.html#chapter_10 "PS3.5 \"10. Transfer Syntax\"")
pub mod ts {
    /// [Implicit VR Little Endian]: Default Transfer Syntax for DICOM
    ///
    /// All applications should support this transfer syntax
    ///
    /// [Implicit VR Little Endian]:
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_A.html#sect_A.1
    ///     "PS3.5 \"A.1 DICOM Implicit VR Little Endian Transfer Syntax\""
    pub const ImplicitVRLittleEndian: &str = "1.2.840.10008.1.2";

    /// [Little Endian Transfer Syntax (Explicit VR)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.2.html#sect_A.2
    ///     "PS3.5 \"A.2 DICOM Little Endian Transfer Syntax (Explicit VR)\"")
    pub const ExplicitVRLittleEndian: &str = "1.2.840.10008.1.2.1";

    /// [Encapsulated Uncompressed Explicit VR Little Endian](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.11.html
    ///     "PS3.5 \"A.4.11 Encapsulated Uncompressed Explicit VR Little Endian\"")
    pub const EncapsulatedUncompressedExplicitVRLittleEndian: &str = "1.2.840.10008.1.2.1.98";

    /// [Deflated Explicit VR Little Endian](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.5.html
    ///     "PS3.5 \"A.5 DICOM Deflated Little Endian Transfer Syntax (Explicit VR)\"")
    pub const DeflatedExplicitVRLittleEndian: &str = "1.2.840.10008.1.2.1.99";

    /// **RETIRED** [Explicit VR Big Endian]
    ///
    /// This Transfer Syntax was retired in 2006, but is supported by the library.
    ///
    /// [Explicit VR Big Endian]:
    ///     https://dicom.nema.org/medical/dicom/2016b/output/chtml/part05/sect_A.3.html
    ///     "(outdated 2016b version) PS3.5 \"A.3 DICOM Big Endian Transfer Syntax (Explicit VR)\""
    pub const ExplicitVRBigEndian: &str = "1.2.840.10008.1.2.2";

    /// [JPEG Baseline (Process 1)] : Default Transfer Syntax for Lossy JPEG 8 Bit Image Compression
    ///
    /// [JPEG Baseline (Process 1)]:
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.html#sect_A.4.1
    ///     "PS3.5 \"A.4.1 JPEG Image Compression\""
    pub const JPEGBaseline8Bit: &str = "1.2.840.10008.1.2.4.50";

    /// [JPEG Extended (Process 2 & 4)]: Default Transfer Syntax for Lossy JPEG 12 Bit Image Compression (Process 4 only)
    ///
    /// [JPEG Extended (Process 2 & 4)]:
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.html#sect_A.4.1
    ///     "PS3.5 \"A.4.1 JPEG Image Compression\""
    pub const JPEGExtended12Bit: &str = "1.2.840.10008.1.2.4.51";

    /// **RETIRED** JPEG Extended (Process 3 & 5)
    pub const RETIRED_JPEGExtended35: &str = "1.2.840.10008.1.2.4.52";

    /// **RETIRED** JPEG Spectral Selection, Non-Hierarchical (Process 6 & 8)
    pub const RETIRED_JPEGSpectralSelectionNonHierarchical68: &str = "1.2.840.10008.1.2.4.53";

    /// **RETIRED** JPEG Spectral Selection, Non-Hierarchical (Process 7 & 9)
    pub const RETIRED_JPEGSpectralSelectionNonHierarchical79: &str = "1.2.840.10008.1.2.4.54";

    /// **RETIRED** JPEG Full Progression, Non-Hierarchical (Process 10 & 12)
    pub const RETIRED_JPEGFullProgressionNonHierarchical1012: &str = "1.2.840.10008.1.2.4.55";

    /// **RETIRED** JPEG Full Progression, Non-Hierarchical (Process 11 & 13)
    pub const RETIRED_JPEGFullProgressionNonHierarchical1113: &str = "1.2.840.10008.1.2.4.56";

    /// [JPEG Lossless, Non-Hierarchical (Process 14)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.html#sect_A.4.1
    ///     "PS3.5 \"A.4.1 JPEG Image Compression\"")
    pub const JPEGLossless: &str = "1.2.840.10008.1.2.4.57";

    /// **RETIRED** JPEG Lossless, Non-Hierarchical (Process 15)
    pub const RETIRED_JPEGLosslessNonHierarchical15: &str = "1.2.840.10008.1.2.4.58";

    /// **RETIRED** JPEG Extended, Hierarchical (Process 16 & 18)
    pub const RETIRED_JPEGExtendedHierarchical1618: &str = "1.2.840.10008.1.2.4.59";

    /// **RETIRED** JPEG Extended, Hierarchical (Process 17 & 19)
    pub const RETIRED_JPEGExtendedHierarchical1719: &str = "1.2.840.10008.1.2.4.60";

    /// **RETIRED** JPEG Spectral Selection, Hierarchical (Process 20 & 22)
    pub const RETIRED_JPEGSpectralSelectionHierarchical2022: &str = "1.2.840.10008.1.2.4.61";

    /// **RETIRED** JPEG Spectral Selection, Hierarchical (Process 21 & 23)
    pub const RETIRED_JPEGSpectralSelectionHierarchical2123: &str = "1.2.840.10008.1.2.4.62";

    /// **RETIRED** JPEG Full Progression, Hierarchical (Process 24 & 26)
    pub const RETIRED_JPEGFullProgressionHierarchical2426: &str = "1.2.840.10008.1.2.4.63";

    /// **RETIRED** JPEG Full Progression, Hierarchical (Process 25 & 27)
    pub const RETIRED_JPEGFullProgressionHierarchical2527: &str = "1.2.840.10008.1.2.4.64";

    /// **RETIRED** JPEG Lossless, Hierarchical (Process 28)
    pub const RETIRED_JPEGLosslessHierarchical28: &str = "1.2.840.10008.1.2.4.65";

    /// **RETIRED** JPEG Lossless, Hierarchical (Process 29)
    pub const RETIRED_JPEGLosslessHierarchical29: &str = "1.2.840.10008.1.2.4.66";

    /// [JPEG Lossless, Non-Hierarchical, First-Order Prediction (Process 14 Selection Value 1)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.html#sect_A.4.1
    ///     "PS3.5 \"A.4.1 JPEG Image Compression\"")
    ///
    /// Default Transfer Syntax for Lossless JPEG Image Compression
    pub const JPEGLosslessSV1: &str = "1.2.840.10008.1.2.4.70";

    /// [JPEG-LS Lossless Image Compression](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.3.html
    ///     "PS3.5 \"A.4.3 JPEG-LS Image Compression\"")
    pub const JPEGLSLossless: &str = "1.2.840.10008.1.2.4.80";

    /// [JPEG-LS Lossy (Near-Lossless) Image Compression](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.3.html
    ///     "PS3.5 \"A.4.3 JPEG-LS Image Compression\"")
    pub const JPEGLSNearLossless: &str = "1.2.840.10008.1.2.4.81";

    /// [JPEG 2000 Image Compression (Lossless Only)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.4.html
    ///     "PS3.5 \"A.4.4 JPEG 2000 Image Compression\"")
    pub const JPEG2000Lossless: &str = "1.2.840.10008.1.2.4.90";

    /// [JPEG 2000 Image Compression (Lossless or Lossy)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.4.html
    ///     "PS3.5 \"A.4.4 JPEG 2000 Image Compression\"")
    pub const JPEG2000: &str = "1.2.840.10008.1.2.4.91";

    /// [JPEG 2000 Part 2 Multi-component Image Compression (Lossless Only)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.4.html
    ///     "PS3.5 \"A.4.4 JPEG 2000 Image Compression\"")
    pub const JPEG2000MCLossless: &str = "1.2.840.10008.1.2.4.92";

    /// [JPEG 2000 Part 2 Multi-component Image Compression (Lossless or Lossy)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.4.html
    ///     "PS3.5 \"A.4.4 JPEG 2000 Image Compression\"")
    pub const JPEG2000MC: &str = "1.2.840.10008.1.2.4.93";

    /// [JPIP Referenced Transfer Syntax (Explicit VR)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.6.html
    ///     "PS3.5 \"A.6 DICOM JPIP Referenced Transfer Syntax (Explicit VR)\"")
    pub const JPIPReferenced: &str = "1.2.840.10008.1.2.4.94";

    /// [JPIP Referenced Deflate Transfer Syntax (Explicit VR)](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.7.html
    ///     "PS3.5 \"A.7 DICOM JPIP Referenced Deflate Transfer Syntax (Explicit VR)\"")
    pub const JPIPReferencedDeflate: &str = "1.2.840.10008.1.2.4.95";

    /// [MPEG2 Main Profile / Main Level](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.5.html
    ///     "PS3.5 \"A.4.5 MPEG2 Video Compression\"")
    pub const MPEG2MPML: &str = "1.2.840.10008.1.2.4.100";

    /// [Fragmentable MPEG2 Main Profile / Main Level](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.5.html
    ///     "PS3.5 \"A.4.5 MPEG2 Video Compression\"")
    pub const MPEG2MPMLF: &str = "1.2.840.10008.1.2.4.100.1";

    /// [MPEG2 Main Profile / High Level](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.5.html
    ///     "PS3.5 \"A.4.5 MPEG2 Video Compression\"")
    pub const MPEG2MPHL: &str = "1.2.840.10008.1.2.4.101";

    /// [Fragmentable MPEG2 Main Profile / High Level](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.5.html
    ///     "PS3.5 \"A.4.5 MPEG2 Video Compression\"")
    pub const MPEG2MPHLF: &str = "1.2.840.10008.1.2.4.101.1";

    /// [MPEG-4 AVC/H.264 High Profile / Level 4.1](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.6.html
    ///     "PS3.5 \"A.4.6 MPEG-4 AVC/H.264 High Profile / Level 4.1 Video Compression\"")
    pub const MPEG4HP41: &str = "1.2.840.10008.1.2.4.102";

    /// [Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.1](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.6.html
    ///     "PS3.5 \"A.4.6 MPEG-4 AVC/H.264 High Profile / Level 4.1 Video Compression\"")
    pub const MPEG4HP41F: &str = "1.2.840.10008.1.2.4.102.1";

    /// [MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.6.html
    ///     "PS3.5 \"A.4.6 MPEG-4 AVC/H.264 High Profile / Level 4.1 Video Compression\"")
    pub const MPEG4HP41BD: &str = "1.2.840.10008.1.2.4.103";

    /// [Fragmentable MPEG-4 AVC/H.264 BD-compatible High Profile / Level 4.1](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.6.html
    ///     "PS3.5 \"A.4.6 MPEG-4 AVC/H.264 High Profile / Level 4.1 Video Compression\"")
    pub const MPEG4HP41BDF: &str = "1.2.840.10008.1.2.4.103.1";

    /// [MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.7.html
    ///     "PS3.5 \"A.4.7 MPEG-4 AVC/H.264 High Profile / Level 4.2 Video Compression\"")
    pub const MPEG4HP422D: &str = "1.2.840.10008.1.2.4.104";

    /// [Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 2D Video](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.7.html
    ///     "PS3.5 \"A.4.7 MPEG-4 AVC/H.264 High Profile / Level 4.2 Video Compression\"")
    pub const MPEG4HP422DF: &str = "1.2.840.10008.1.2.4.104.1";

    /// [MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.7.html
    ///     "PS3.5 \"A.4.7 MPEG-4 AVC/H.264 High Profile / Level 4.2 Video Compression\"")
    pub const MPEG4HP423D: &str = "1.2.840.10008.1.2.4.105";

    /// [Fragmentable MPEG-4 AVC/H.264 High Profile / Level 4.2 For 3D Video](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.7.html
    ///     "PS3.5 \"A.4.7 MPEG-4 AVC/H.264 High Profile / Level 4.2 Video Compression\"")
    pub const MPEG4HP423DF: &str = "1.2.840.10008.1.2.4.105.1";

    /// [MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.8.html
    ///     "PS3.5 \"A.4.8 MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2 Video Compression\"")
    pub const MPEG4HP42STEREO: &str = "1.2.840.10008.1.2.4.106";

    /// [Fragmentable MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.8.html
    ///     "PS3.5 \"A.4.8 MPEG-4 AVC/H.264 Stereo High Profile / Level 4.2 Video Compression\"")
    pub const MPEG4HP42STEREOF: &str = "1.2.840.10008.1.2.4.106.1";

    /// [HEVC/H.265 Main Profile / Level 5.1](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.9.html
    ///     "PS3.5 \"A.4.9 HEVC/H.265 Main Profile / Level 5.1 Video Compression\"")
    pub const HEVCMP51: &str = "1.2.840.10008.1.2.4.107";

    /// [HEVC/H.265 Main 10 Profile / Level 5.1](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.10.html
    ///     "PS3.5 \"A.4.10 HEVC/H.265 Main 10 Profile / Level 5.1 Video Compression\"")
    pub const HEVCM10P51: &str = "1.2.840.10008.1.2.4.108";

    /// [RLE Lossless](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.4.2.html
    ///     "PS3.5 \"A.4.2 RLE Image Compression\"")
    pub const RLELossless: &str = "1.2.840.10008.1.2.5";

    /// **RETIRED** [RFC 2557 MIME encapsulation](
    ///     https://dicom.nema.org/medical/dicom/2018b/output/chtml/part10/chapter_B.html
    ///     "(outdated 2018b) PS3.10 \"B HL7 Structured Document Files\"")
    pub const RETIRED_RFC2557MIMEEncapsulation: &str = "1.2.840.10008.1.2.6.1";

    /// **RETIRED** [XML Encoding](
    ///     https://dicom.nema.org/medical/dicom/2018b/output/chtml/part10/chapter_B.html
    ///     "(outdated 2018b) PS3.10 \"B HL7 Structured Document Files\"")
    pub const RETIRED_XMLEncoding: &str = "1.2.840.10008.1.2.6.1";

    /// [SMPTE ST 2110-20 Uncompressed Progressive Active Video](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.8.html
    ///     "PS3.5 \"A.8 SMPTE ST 2110-20 Uncompressed Progressive Active Video Transfer Syntax\"")
    pub const SMPTEST211020UncompressedProgressiveActiveVideo: &str = "1.2.840.10008.1.2.7.1";

    /// [SMPTE ST 2110-20 Uncompressed Interlaced Active Video Transfer Syntax](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.9.html
    ///     "PS3.5 \"A.9 SMPTE ST 2110-20 Uncompressed Interlaced Active Video Transfer Syntax\"")
    pub const SMPTEST211020UncompressedInterlacedActiveVideo: &str = "1.2.840.10008.1.2.7.2";

    /// [SMPTE ST 2110-30 PCM Audio Transfer Syntax](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_A.10.html
    ///     "PS3.5 \"A.10 SMPTE ST 2110-30 PCM Audio Transfer Syntax\"")
    pub const SMPTEST211030PCMDigitalAudio: &str = "1.2.840.10008.1.2.7.3";
}

/// [Verification Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_A
///     "PS3.4 \"A Verification Service Class (Normative)\"")
pub mod svc_verification {
    pub const Verification: &str = "1.2.840.10008.1.1";
}

/// [Storage Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_B
///     "PS3.4 \"B. Storage Service Class (Normative)\"")
pub mod svc_storage {
    /// [Storage Service Class](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#sect_B.3.1.3
    ///     "PS3.4 \"B.3.1.3. Service Class UID (A-ASSOCIATE-RQ)\"")
    pub const Storage: &str = "1.2.840.10008.4.2";

    // Standard DICOM storage SOP Classes
    pub const RETIRED_StoredPrintStorage: &str = "1.2.840.10008.5.1.1.27";
    pub const RETIRED_HardcopyGrayscaleImageStorage: &str = "1.2.840.10008.5.1.1.29";
    pub const RETIRED_HardcopyColorImageStorage: &str = "1.2.840.10008.5.1.1.30";
    pub const ComputedRadiographyImageStorage: &str = "1.2.840.10008.5.1.4.1.1.1";
    pub const DigitalXRayImageStorageForPresentation: &str = "1.2.840.10008.5.1.4.1.1.1.1";
    pub const DigitalXRayImageStorageForProcessing: &str = "1.2.840.10008.5.1.4.1.1.1.1.1";
    pub const DigitalMammographyXRayImageStorageForPresentation: &str = "1.2.840.10008.5.1.4.1.1.1.2";
    pub const DigitalMammographyXRayImageStorageForProcessing: &str = "1.2.840.10008.5.1.4.1.1.1.2.1";
    pub const DigitalIntraOralXRayImageStorageForPresentation: &str = "1.2.840.10008.5.1.4.1.1.1.3";
    pub const DigitalIntraOralXRayImageStorageForProcessing: &str = "1.2.840.10008.5.1.4.1.1.1.3.1";
    pub const CTImageStorage: &str = "1.2.840.10008.5.1.4.1.1.2";
    pub const EnhancedCTImageStorage: &str = "1.2.840.10008.5.1.4.1.1.2.1";
    pub const LegacyConvertedEnhancedCTImageStorage: &str = "1.2.840.10008.5.1.4.1.1.2.2";
    pub const RETIRED_UltrasoundMultiFrameImageStorageRetired: &str = "1.2.840.10008.5.1.4.1.1.3";
    pub const UltrasoundMultiFrameImageStorage: &str = "1.2.840.10008.5.1.4.1.1.3.1";
    pub const MRImageStorage: &str = "1.2.840.10008.5.1.4.1.1.4";
    pub const EnhancedMRImageStorage: &str = "1.2.840.10008.5.1.4.1.1.4.1";
    pub const MRSpectroscopyStorage: &str = "1.2.840.10008.5.1.4.1.1.4.2";
    pub const EnhancedMRColorImageStorage: &str = "1.2.840.10008.5.1.4.1.1.4.3";
    pub const LegacyConvertedEnhancedMRImageStorage: &str = "1.2.840.10008.5.1.4.1.1.4.4";
    pub const RETIRED_NuclearMedicineImageStorageRetired: &str = "1.2.840.10008.5.1.4.1.1.5";
    pub const RETIRED_UltrasoundImageStorageRetired: &str = "1.2.840.10008.5.1.4.1.1.6";
    pub const UltrasoundImageStorage: &str = "1.2.840.10008.5.1.4.1.1.6.1";
    pub const EnhancedUSVolumeStorage: &str = "1.2.840.10008.5.1.4.1.1.6.2";
    pub const SecondaryCaptureImageStorage: &str = "1.2.840.10008.5.1.4.1.1.7";
    pub const MultiFrameSingleBitSecondaryCaptureImageStorage: &str = "1.2.840.10008.5.1.4.1.1.7.1";
    pub const MultiFrameGrayscaleByteSecondaryCaptureImageStorage: &str = "1.2.840.10008.5.1.4.1.1.7.2";
    pub const MultiFrameGrayscaleWordSecondaryCaptureImageStorage: &str = "1.2.840.10008.5.1.4.1.1.7.3";
    pub const MultiFrameTrueColorSecondaryCaptureImageStorage: &str = "1.2.840.10008.5.1.4.1.1.7.4";
    pub const RETIRED_StandaloneOverlayStorage: &str = "1.2.840.10008.5.1.4.1.1.8";
    pub const RETIRED_StandaloneCurveStorage: &str = "1.2.840.10008.5.1.4.1.1.9";
    pub const RETIRED_WaveformStorageTrial: &str = "1.2.840.10008.5.1.4.1.1.9.1";
    pub const TwelveLeadECGWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.1.1";
    pub const GeneralECGWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.1.2";
    pub const AmbulatoryECGWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.1.3";
    pub const HemodynamicWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.2.1";
    pub const CardiacElectrophysiologyWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.3.1";
    pub const BasicVoiceAudioWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.4.1";
    pub const GeneralAudioWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.4.2";
    pub const ArterialPulseWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.5.1";
    pub const RespiratoryWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.6.1";
    pub const MultichannelRespiratoryWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.6.2";
    pub const RoutineScalpElectroencephalogramWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.7.1";
    pub const ElectromyogramWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.7.2";
    pub const ElectrooculogramWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.7.3";
    pub const SleepElectroencephalogramWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.7.4";
    pub const BodyPositionWaveformStorage: &str = "1.2.840.10008.5.1.4.1.1.9.8.1";
    pub const RETIRED_StandaloneModalityLUTStorage: &str = "1.2.840.10008.5.1.4.1.1.10";
    pub const RETIRED_StandaloneVOILUTStorage: &str = "1.2.840.10008.5.1.4.1.1.11";
    pub const GrayscaleSoftcopyPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.1";
    pub const ColorSoftcopyPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.2";
    pub const PseudoColorSoftcopyPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.3";
    pub const BlendingSoftcopyPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.4";
    pub const XAXRFGrayscaleSoftcopyPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.5";
    pub const GrayscalePlanarMPRVolumetricPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.6";
    pub const CompositingPlanarMPRVolumetricPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.7";
    pub const AdvancedBlendingPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.8";
    pub const VolumeRenderingVolumetricPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.9";
    pub const SegmentedVolumeRenderingVolumetricPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.10";
    pub const MultipleVolumeRenderingVolumetricPresentationStateStorage: &str = "1.2.840.10008.5.1.4.1.1.11.11";
    pub const XRayAngiographicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.12.1";
    pub const EnhancedXAImageStorage: &str = "1.2.840.10008.5.1.4.1.1.12.1.1";
    pub const XRayRadiofluoroscopicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.12.2";
    pub const EnhancedXRFImageStorage: &str = "1.2.840.10008.5.1.4.1.1.12.2.1";
    pub const RETIRED_XRayAngiographicBiPlaneImageStorage: &str = "1.2.840.10008.5.1.4.1.1.12.3";
    pub const XRay3DAngiographicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.13.1.1";
    pub const XRay3DCraniofacialImageStorage: &str = "1.2.840.10008.5.1.4.1.1.13.1.2";
    pub const BreastTomosynthesisImageStorage: &str = "1.2.840.10008.5.1.4.1.1.13.1.3";
    pub const BreastProjectionXRayImageStorageForPresentation: &str = "1.2.840.10008.5.1.4.1.1.13.1.4";
    pub const BreastProjectionXRayImageStorageForProcessing: &str = "1.2.840.10008.5.1.4.1.1.13.1.5";
    pub const IntravascularOpticalCoherenceTomographyImageStorageForPresentation: &str = "1.2.840.10008.5.1.4.1.1.14.1";
    pub const IntravascularOpticalCoherenceTomographyImageStorageForProcessing: &str = "1.2.840.10008.5.1.4.1.1.14.2";
    pub const NuclearMedicineImageStorage: &str = "1.2.840.10008.5.1.4.1.1.20";
    pub const ParametricMapStorage: &str = "1.2.840.10008.5.1.4.1.1.30";
    pub const RawDataStorage: &str = "1.2.840.10008.5.1.4.1.1.66";
    pub const SpatialRegistrationStorage: &str = "1.2.840.10008.5.1.4.1.1.66.1";
    pub const SpatialFiducialsStorage: &str = "1.2.840.10008.5.1.4.1.1.66.2";
    pub const DeformableSpatialRegistrationStorage: &str = "1.2.840.10008.5.1.4.1.1.66.3";
    pub const SegmentationStorage: &str = "1.2.840.10008.5.1.4.1.1.66.4";
    pub const SurfaceSegmentationStorage: &str = "1.2.840.10008.5.1.4.1.1.66.5";
    pub const TractographyResultsStorage: &str = "1.2.840.10008.5.1.4.1.1.66.6";
    pub const RealWorldValueMappingStorage: &str = "1.2.840.10008.5.1.4.1.1.67";
    pub const SurfaceScanMeshStorage: &str = "1.2.840.10008.5.1.4.1.1.68.1";
    pub const SurfaceScanPointCloudStorage: &str = "1.2.840.10008.5.1.4.1.1.68.2";
    pub const RETIRED_VLImageStorageTrial: &str = "1.2.840.10008.5.1.4.1.1.77.1";
    pub const RETIRED_VLMultiFrameImageStorageTrial: &str = "1.2.840.10008.5.1.4.1.1.77.2";
    pub const VLEndoscopicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.1";
    pub const VideoEndoscopicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.1.1";
    pub const VLMicroscopicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.2";
    pub const VideoMicroscopicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.2.1";
    pub const VLSlideCoordinatesMicroscopicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.3";
    pub const VLPhotographicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.4";
    pub const VideoPhotographicImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.4.1";
    pub const OphthalmicPhotography8BitImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.1";
    pub const OphthalmicPhotography16BitImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.2";
    pub const StereometricRelationshipStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.3";
    pub const OphthalmicTomographyImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.4";
    pub const WideFieldOphthalmicPhotographyStereographicProjectionImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.5";
    pub const WideFieldOphthalmicPhotography3DCoordinatesImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.6";
    pub const OphthalmicOpticalCoherenceTomographyEnFaceImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.7";
    pub const OphthalmicOpticalCoherenceTomographyBscanVolumeAnalysisStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.5.8";
    pub const VLWholeSlideMicroscopyImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.6";
    pub const DermoscopicPhotographyImageStorage: &str = "1.2.840.10008.5.1.4.1.1.77.1.7";
    pub const LensometryMeasurementsStorage: &str = "1.2.840.10008.5.1.4.1.1.78.1";
    pub const AutorefractionMeasurementsStorage: &str = "1.2.840.10008.5.1.4.1.1.78.2";
    pub const KeratometryMeasurementsStorage: &str = "1.2.840.10008.5.1.4.1.1.78.3";
    pub const SubjectiveRefractionMeasurementsStorage: &str = "1.2.840.10008.5.1.4.1.1.78.4";
    pub const VisualAcuityMeasurementsStorage: &str = "1.2.840.10008.5.1.4.1.1.78.5";
    pub const SpectaclePrescriptionReportStorage: &str = "1.2.840.10008.5.1.4.1.1.78.6";
    pub const OphthalmicAxialMeasurementsStorage: &str = "1.2.840.10008.5.1.4.1.1.78.7";
    pub const IntraocularLensCalculationsStorage: &str = "1.2.840.10008.5.1.4.1.1.78.8";
    pub const MacularGridThicknessAndVolumeReportStorage: &str = "1.2.840.10008.5.1.4.1.1.79.1";
    pub const OphthalmicVisualFieldStaticPerimetryMeasurementsStorage: &str = "1.2.840.10008.5.1.4.1.1.80.1";
    pub const OphthalmicThicknessMapStorage: &str = "1.2.840.10008.5.1.4.1.1.81.1";
    pub const CornealTopographyMapStorage: &str = "1.2.840.10008.5.1.4.1.1.82.1";
    pub const RETIRED_TextSRStorageTrial: &str = "1.2.840.10008.5.1.4.1.1.88.1";
    pub const RETIRED_AudioSRStorageTrial: &str = "1.2.840.10008.5.1.4.1.1.88.2";
    pub const RETIRED_DetailSRStorageTrial: &str = "1.2.840.10008.5.1.4.1.1.88.3";
    pub const RETIRED_ComprehensiveSRStorageTrial: &str = "1.2.840.10008.5.1.4.1.1.88.4";
    pub const BasicTextSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.11";
    pub const EnhancedSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.22";
    pub const ComprehensiveSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.33";
    pub const Comprehensive3DSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.34";
    pub const ExtensibleSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.35";
    pub const ProcedureLogStorage: &str = "1.2.840.10008.5.1.4.1.1.88.40";
    pub const MammographyCADSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.50";
    pub const KeyObjectSelectionDocumentStorage: &str = "1.2.840.10008.5.1.4.1.1.88.59";
    pub const ChestCADSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.65";
    pub const XRayRadiationDoseSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.67";
    pub const RadiopharmaceuticalRadiationDoseSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.68";
    pub const ColonCADSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.69";
    pub const ImplantationPlanSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.70";
    pub const AcquisitionContextSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.71";
    pub const SimplifiedAdultEchoSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.72";
    pub const PatientRadiationDoseSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.73";
    pub const PlannedImagingAgentAdministrationSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.74";
    pub const PerformedImagingAgentAdministrationSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.75";
    pub const EnhancedXRayRadiationDoseSRStorage: &str = "1.2.840.10008.5.1.4.1.1.88.76";
    pub const ContentAssessmentResultsStorage: &str = "1.2.840.10008.5.1.4.1.1.90.1";
    pub const MicroscopyBulkSimpleAnnotationsStorage: &str = "1.2.840.10008.5.1.4.1.1.91.1";
    pub const EncapsulatedPDFStorage: &str = "1.2.840.10008.5.1.4.1.1.104.1";
    pub const EncapsulatedCDAStorage: &str = "1.2.840.10008.5.1.4.1.1.104.2";
    pub const EncapsulatedSTLStorage: &str = "1.2.840.10008.5.1.4.1.1.104.3";
    pub const EncapsulatedOBJStorage: &str = "1.2.840.10008.5.1.4.1.1.104.4";
    pub const EncapsulatedMTLStorage: &str = "1.2.840.10008.5.1.4.1.1.104.5";
    pub const PositronEmissionTomographyImageStorage: &str = "1.2.840.10008.5.1.4.1.1.128";
    pub const LegacyConvertedEnhancedPETImageStorage: &str = "1.2.840.10008.5.1.4.1.1.128.1";
    pub const RETIRED_StandalonePETCurveStorage: &str = "1.2.840.10008.5.1.4.1.1.129";
    pub const EnhancedPETImageStorage: &str = "1.2.840.10008.5.1.4.1.1.130";
    pub const BasicStructuredDisplayStorage: &str = "1.2.840.10008.5.1.4.1.1.131";
    pub const CTPerformedProcedureProtocolStorage: &str = "1.2.840.10008.5.1.4.1.1.200.2";
    pub const XAPerformedProcedureProtocolStorage: &str = "1.2.840.10008.5.1.4.1.1.200.8";
    pub const RTImageStorage: &str = "1.2.840.10008.5.1.4.1.1.481.1";
    pub const RTDoseStorage: &str = "1.2.840.10008.5.1.4.1.1.481.2";
    pub const RTStructureSetStorage: &str = "1.2.840.10008.5.1.4.1.1.481.3";
    pub const RTBeamsTreatmentRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.4";
    pub const RTPlanStorage: &str = "1.2.840.10008.5.1.4.1.1.481.5";
    pub const RTBrachyTreatmentRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.6";
    pub const RTTreatmentSummaryRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.7";
    pub const RTIonPlanStorage: &str = "1.2.840.10008.5.1.4.1.1.481.8";
    pub const RTIonBeamsTreatmentRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.9";
    pub const RTPhysicianIntentStorage: &str = "1.2.840.10008.5.1.4.1.1.481.10";
    pub const RTSegmentAnnotationStorage: &str = "1.2.840.10008.5.1.4.1.1.481.11";
    pub const RTRadiationSetStorage: &str = "1.2.840.10008.5.1.4.1.1.481.12";
    pub const CArmPhotonElectronRadiationStorage: &str = "1.2.840.10008.5.1.4.1.1.481.13";
    pub const TomotherapeuticRadiationStorage: &str = "1.2.840.10008.5.1.4.1.1.481.14";
    pub const RoboticArmRadiationStorage: &str = "1.2.840.10008.5.1.4.1.1.481.15";
    pub const RTRadiationRecordSetStorage: &str = "1.2.840.10008.5.1.4.1.1.481.16";
    pub const RTRadiationSalvageRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.17";
    pub const TomotherapeuticRadiationRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.18";
    pub const CArmPhotonElectronRadiationRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.19";
    pub const RoboticRadiationRecordStorage: &str = "1.2.840.10008.5.1.4.1.1.481.20";
    pub const RTRadiationSetDeliveryInstructionStorage: &str = "1.2.840.10008.5.1.4.1.1.481.21";
    pub const RTTreatmentPreparationStorage: &str = "1.2.840.10008.5.1.4.1.1.481.22";
    pub const EnhancedRTImageStorage: &str = "1.2.840.10008.5.1.4.1.1.481.23";
    pub const EnhancedContinuousRTImageStorage: &str = "1.2.840.10008.5.1.4.1.1.481.24";
    pub const RTPatientPositionAcquisitionInstructionStorage: &str = "1.2.840.10008.5.1.4.1.1.481.25";
    pub const RETIRED_RTBeamsDeliveryInstructionStorageTrial: &str = "1.2.840.10008.5.1.4.34.1";
    pub const RETIRED_RTConventionalMachineVerificationTrial: &str = "1.2.840.10008.5.1.4.34.2";
    pub const RETIRED_RTIonMachineVerificationTrial: &str = "1.2.840.10008.5.1.4.34.3";
    pub const RTBeamsDeliveryInstructionStorage: &str = "1.2.840.10008.5.1.4.34.7";
    pub const RTBrachyApplicationSetupDeliveryInstructionStorage: &str = "1.2.840.10008.5.1.4.34.10";

    // DICOS Storage
    pub const DICOSCTImageStorage: &str = "1.2.840.10008.5.1.4.1.1.501.1";
    pub const DICOSDigitalXRayImageStorageForPresentation: &str = "1.2.840.10008.5.1.4.1.1.501.2.1";
    pub const DICOSDigitalXRayImageStorageForProcessing: &str = "1.2.840.10008.5.1.4.1.1.501.2.2";
    pub const DICOSThreatDetectionReportStorage: &str = "1.2.840.10008.5.1.4.1.1.501.3";
    pub const DICOS2DAITStorage: &str = "1.2.840.10008.5.1.4.1.1.501.4";
    pub const DICOS3DAITStorage: &str = "1.2.840.10008.5.1.4.1.1.501.5";
    pub const DICOSQuadrupoleResonanceStorage: &str = "1.2.840.10008.5.1.4.1.1.501.6";

    // DICONDE Storage
    pub const DICONDEEddyCurrentImageStorage: &str = "1.2.840.10008.5.1.4.1.1.601.1";
    pub const DICONDEEddyCurrentMultiFrameImageStorage: &str = "1.2.840.10008.5.1.4.1.1.601.2";
}

/// [Query/Retrieve Service Class]
///
/// Also includes [Composite Instance Root Retrieve Service Class] and
/// [Composite Instance Retrieve Without Bulk Data Service Class]
///
/// [Query/Retrieve Service Class]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_C
///     "PS3.4 \"C. Query/Retrieve Service Class (Normative)\""
/// [Composite Instance Root Retrieve Service Class]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_Y
///     "PS3.4 \"Y. Composite Instance Root Retrieve Service Class (Normative)\""
/// [Composite Instance Retrieve Without Bulk Data Service Class]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_Z
///     "PS3.4 \"Z. Composite Instance Retrieve Without Bulk Data Service Class (Normative)\""
pub mod svc_qr {
    pub const PatientRootQueryRetrieveInformationModelFind: &str = "1.2.840.10008.5.1.4.1.2.1.1";
    pub const PatientRootQueryRetrieveInformationModelMove: &str = "1.2.840.10008.5.1.4.1.2.1.2";
    pub const PatientRootQueryRetrieveInformationModelGet: &str = "1.2.840.10008.5.1.4.1.2.1.3";
    pub const StudyRootQueryRetrieveInformationModelFind: &str = "1.2.840.10008.5.1.4.1.2.2.1";
    pub const StudyRootQueryRetrieveInformationModelMove: &str = "1.2.840.10008.5.1.4.1.2.2.2";
    pub const StudyRootQueryRetrieveInformationModelGet: &str = "1.2.840.10008.5.1.4.1.2.2.3";
    pub const RETIRED_PatientStudyOnlyQueryRetrieveInformationModelFind: &str = "1.2.840.10008.5.1.4.1.2.3.1";
    pub const RETIRED_PatientStudyOnlyQueryRetrieveInformationModelMove: &str = "1.2.840.10008.5.1.4.1.2.3.2";
    pub const RETIRED_PatientStudyOnlyQueryRetrieveInformationModelGet: &str = "1.2.840.10008.5.1.4.1.2.3.3";
    pub const CompositeInstanceRootRetrieveMove: &str = "1.2.840.10008.5.1.4.1.2.4.2";
    pub const CompositeInstanceRootRetrieveGet: &str = "1.2.840.10008.5.1.4.1.2.4.3";
    pub const CompositeInstanceRetrieveWithoutBulkDataGet: &str = "1.2.840.10008.5.1.4.1.2.5.3";
}

/// [Study Management Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_F
///     "PS3.4 \"F. Procedure Step SOP Classes (Normative)\"")
///
/// Currently called as "Procedure Step SOP Classes", because almost all of the
/// service has been retired and only "Procedure Step" remains also known as "MPPS".
pub mod svc_study {
    pub const RETIRED_DetachedStudyManagement: &str = "1.2.840.10008.3.1.2.3.1";
    pub const RETIRED_StudyComponentManagement: &str = "1.2.840.10008.3.1.2.3.2";
    pub const RETIRED_DetachedResultsManagement: &str = "1.2.840.10008.3.1.2.5.1";
    pub const RETIRED_DetachedResultsManagementMeta: &str = "1.2.840.10008.3.1.2.5.4";
    pub const RETIRED_DetachedStudyManagementMeta: &str = "1.2.840.10008.3.1.2.5.5";
    pub const RETIRED_GeneralPurposeScheduledProcedureStep: &str = "1.2.840.10008.5.1.4.32.2";
    pub const RETIRED_GeneralPurposePerformedProcedureStep: &str = "1.2.840.10008.5.1.4.32.3";

    // MPPS
    pub const ModalityPerformedProcedureStep: &str = "1.2.840.10008.3.1.2.3.3";
    pub const ModalityPerformedProcedureStepRetrieve: &str = "1.2.840.10008.3.1.2.3.4";
    pub const ModalityPerformedProcedureStepNotification: &str = "1.2.840.10008.3.1.2.3.5";
}

/// [Print Management Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_H
///     "PS3.4 \"H. Print Management Service Class (Normative)\"")
pub mod svc_print {
    pub const BasicFilmSession: &str = "1.2.840.10008.5.1.1.1";
    pub const BasicFilmBox: &str = "1.2.840.10008.5.1.1.2";
    pub const BasicGrayscaleImageBox: &str = "1.2.840.10008.5.1.1.4";
    pub const BasicColorImageBox: &str = "1.2.840.10008.5.1.1.4.1";
    pub const ReferencedImageBox: &str = "1.2.840.10008.5.1.1.4.2";
    pub const BasicGrayscalePrintManagementMeta: &str = "1.2.840.10008.5.1.1.9";
    pub const RETIRED_ReferencedGrayscalePrintManagementMeta: &str = "1.2.840.10008.5.1.1.9.1";
    pub const PrintJob: &str = "1.2.840.10008.5.1.1.14";
    pub const BasicAnnotationBox: &str = "1.2.840.10008.5.1.1.15";
    pub const Printer: &str = "1.2.840.10008.5.1.1.16";
    pub const PrinterConfigurationRetrieval: &str = "1.2.840.10008.5.1.1.16.376";
    pub const PrinterInstance: &str = "1.2.840.10008.5.1.1.17";
    pub const PrinterConfigurationRetrievalInstance: &str = "1.2.840.10008.5.1.1.17.376";
    pub const BasicColorPrintManagementMeta: &str = "1.2.840.10008.5.1.1.18";
    pub const RETIRED_ReferencedColorPrintManagementMeta: &str = "1.2.840.10008.5.1.1.18.1";
    pub const VOILUTBox: &str = "1.2.840.10008.5.1.1.22";
    pub const PresentationLUT: &str = "1.2.840.10008.5.1.1.23";
    pub const RETIRED_ImageOverlayBox: &str = "1.2.840.10008.5.1.1.24";
    pub const RETIRED_BasicPrintImageOverlayBox: &str = "1.2.840.10008.5.1.1.24.1";
    pub const RETIRED_PullPrintRequest: &str = "1.2.840.10008.5.1.1.31";
    pub const RETIRED_PullStoredPrintManagementMeta: &str = "1.2.840.10008.5.1.1.32";
}

/// [Media Storage Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_I
///     "PS3.4 \"I. Media Storage Service Class (Normative)\"")
pub mod svc_media {
    /// [Media Storage Directory Storage](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part10.html#sect_8.6
    ///     "PS3.10 \"8.6 Reserved DICOMDIR File ID\"")
    pub const MediaStorageDirectoryStorage: &str = "1.2.840.10008.1.3.10";
}

/// [Storage Commitment Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_J
///     "PS3.4 \"J. Storage Commitment Service Class (Normative)\"")
pub mod svc_commitment {
    pub const StorageCommitmentPushModel: &str = "1.2.840.10008.1.20.1";
    pub const StorageCommitmentPushModelInstance: &str = "1.2.840.10008.1.20.1.1";
    pub const RETIRED_StorageCommitmentPullModel: &str = "1.2.840.10008.1.20.2";
    pub const RETIRED_StorageCommitmentPullModelInstance: &str = "1.2.840.10008.1.20.2.1";
}

/// [Basic Worklist Management Service](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_K
///     "PS3.4 \"K. Basic Worklist Management Service (Normative)\"")
pub mod svc_worklist {
    pub const ModalityWorklistInformationModelFind: &str = "1.2.840.10008.5.1.4.31";
    pub const RETIRED_GeneralPurposeWorklistManagementMeta: &str = "1.2.840.10008.5.1.4.32";
    pub const RETIRED_GeneralPurposeWorklistInformationModelFind: &str = "1.2.840.10008.5.1.4.32.1";
}

/// [Application Event Logging Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_P
///     "PS3.4 \"P. Application Event Logging Service Class (Normative)\"")
pub mod svc_event_logging {
    pub const ProceduralEventLogging: &str = "1.2.840.10008.1.40";
    pub const ProceduralEventLoggingInstance: &str = "1.2.840.10008.1.40.1";
    pub const SubstanceAdministrationLogging: &str = "1.2.840.10008.1.42";
    pub const SubstanceAdministrationLoggingInstance: &str = "1.2.840.10008.1.42.1";
}

/// [Relevant Patient Information Query Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_Q
///     "PS3.4 \"Q. Relevant Patient Information Query Service Class (Normative)\"")
pub mod svc_relevant_patient_info {
    pub const GeneralRelevantPatientInformationQuery: &str = "1.2.840.10008.5.1.4.37.1";
    pub const BreastImagingRelevantPatientInformationQuery: &str = "1.2.840.10008.5.1.4.37.2";
    pub const CardiacRelevantPatientInformationQuery: &str = "1.2.840.10008.5.1.4.37.3";
}

/// [Instance Availability Notification Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_R
///     "PS3.4 \"R. Instance Availability Notification Service Class (Normative)\"")
pub mod svc_ian {
    pub const InstanceAvailabilityNotification: &str = "1.2.840.10008.5.1.4.33";
}

/// [Media Creation Management Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_S
///     "PS3.4 \"S. Media Creation Management Service Class (Normative)\"")
pub mod svc_media_creation {
    pub const MediaCreationManagement: &str = "1.2.840.10008.5.1.1.33";
}

/// [Hanging Protocol Storage] and [Query/Retrieve Service] Classes
///
/// [Hanging Protocol Storage]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_T
///     "PS3.4 \"T. Hanging Protocol Storage Service Class\""
/// [Query/Retrieve Service]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_U
///     "PS3.4 \"U. Hanging Protocol Query/Retrieve Service Class\""
pub mod svc_hanging {
    pub const HangingProtocolStorage: &str = "1.2.840.10008.5.1.4.38.1";
    pub const HangingProtocolInformationModelFind: &str = "1.2.840.10008.5.1.4.38.2";
    pub const HangingProtocolInformationModelMove: &str = "1.2.840.10008.5.1.4.38.3";
    pub const HangingProtocolInformationModelGet: &str = "1.2.840.10008.5.1.4.38.4";
}

/// [Substance Administration Query Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_V
///     "PS3.4 \"V. Substance Administration Query Service Class (Normative)\"")
pub mod svc_substance {
    pub const ProductCharacteristicsQuery: &str = "1.2.840.10008.5.1.4.41";
    pub const SubstanceApprovalQuery: &str = "1.2.840.10008.5.1.4.42";
}

/// [Color Palette Storage] and [Query/Retrieve Service] Classes
///
/// Also contains [well-known color palette instances](
///     https://dicom.nema.org/medical/dicom/current/output/html/part06.html#chapter_B
///     "PS3.6 \"B. Well-Known Color Palettes (Normative)\"")
///
/// [Color Palette Storage]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_W
///     "PS3.4 \"W. Color Palette Storage Service Class\""
/// [Query/Retrieve Service]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_X
///     "PS3.4 \"X. Color Palette Query/Retrieve Service Class\""
pub mod svc_palette {
    pub const ColorPaletteStorage: &str = "1.2.840.10008.5.1.4.39.1";
    pub const ColorPaletteQueryRetrieveInformationModelFind: &str = "1.2.840.10008.5.1.4.39.2";
    pub const ColorPaletteQueryRetrieveInformationModelMove: &str = "1.2.840.10008.5.1.4.39.3";
    pub const ColorPaletteQueryRetrieveInformationModelGet: &str = "1.2.840.10008.5.1.4.39.4";

    /// Well known color palettes
    pub const HotIronPalette: &str = "1.2.840.10008.1.5.1";
    pub const PETPalette: &str = "1.2.840.10008.1.5.2";
    pub const HotMetalBluePalette: &str = "1.2.840.10008.1.5.3";
    pub const PET20StepPalette: &str = "1.2.840.10008.1.5.4";
    pub const SpringPalette: &str = "1.2.840.10008.1.5.5";
    pub const SummerPalette: &str = "1.2.840.10008.1.5.6";
    pub const FallPalette: &str = "1.2.840.10008.1.5.7";
    pub const WinterPalette: &str = "1.2.840.10008.1.5.8";
}

/// [Implant Template Storage and Query/Retrieve Service Classes](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_BB
///     "PS3.4 \"BB. Implant Template Query/Retrieve Service Classes\"")
pub mod svc_implant {
    pub const GenericImplantTemplateStorage: &str = "1.2.840.10008.5.1.4.43.1";
    pub const GenericImplantTemplateInformationModelFind: &str = "1.2.840.10008.5.1.4.43.2";
    pub const GenericImplantTemplateInformationModelMove: &str = "1.2.840.10008.5.1.4.43.3";
    pub const GenericImplantTemplateInformationModelGet: &str = "1.2.840.10008.5.1.4.43.4";

    pub const ImplantAssemblyTemplateStorage: &str = "1.2.840.10008.5.1.4.44.1";
    pub const ImplantAssemblyTemplateInformationModelFind: &str = "1.2.840.10008.5.1.4.44.2";
    pub const ImplantAssemblyTemplateInformationModelMove: &str = "1.2.840.10008.5.1.4.44.3";
    pub const ImplantAssemblyTemplateInformationModelGet: &str = "1.2.840.10008.5.1.4.44.4";

    pub const ImplantTemplateGroupStorage: &str = "1.2.840.10008.5.1.4.45.1";
    pub const ImplantTemplateGroupInformationModelFind: &str = "1.2.840.10008.5.1.4.45.2";
    pub const ImplantTemplateGroupInformationModelMove: &str = "1.2.840.10008.5.1.4.45.3";
    pub const ImplantTemplateGroupInformationModelGet: &str = "1.2.840.10008.5.1.4.45.4";
}

/// [Unified Procedure Step Service and SOP Classes](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_CC
///     "PS3.4 \"CC. Unified Procedure Step Service and SOP Classes (Normative)\"")
pub mod svc_ups {
    pub const RETIRED_UnifiedWorklistAndProcedureStepTrial: &str = "1.2.840.10008.5.1.4.34.4";
    pub const RETIRED_UnifiedProcedureStepPushTrial: &str = "1.2.840.10008.5.1.4.34.4.1";
    pub const RETIRED_UnifiedProcedureStepWatchTrial: &str = "1.2.840.10008.5.1.4.34.4.2";
    pub const RETIRED_UnifiedProcedureStepPullTrial: &str = "1.2.840.10008.5.1.4.34.4.3";
    pub const RETIRED_UnifiedProcedureStepEventTrial: &str = "1.2.840.10008.5.1.4.34.4.4";

    pub const UnifiedWorklistAndProcedureStep: &str = "1.2.840.10008.5.1.4.34.6";
    pub const UnifiedProcedureStepPush: &str = "1.2.840.10008.5.1.4.34.6.1";
    pub const UnifiedProcedureStepWatch: &str = "1.2.840.10008.5.1.4.34.6.2";
    pub const UnifiedProcedureStepPull: &str = "1.2.840.10008.5.1.4.34.6.3";
    pub const UnifiedProcedureStepEvent: &str = "1.2.840.10008.5.1.4.34.6.4";
    pub const UnifiedProcedureStepQuery: &str = "1.2.840.10008.5.1.4.34.6.5";
    pub const UPSGlobalSubscriptionInstance: &str = "1.2.840.10008.5.1.4.34.5";
    pub const UPSFilteredGlobalSubscriptionInstance: &str = "1.2.840.10008.5.1.4.34.5.1";
}

/// [Unified Procedure Step Service and SOP Classes](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_DD
///     "PS3.4 \"DD. RT Machine Verification Service Classes (Normative)\"")
pub mod svc_rt_machine_verficiation {
    pub const RTConventionalMachineVerification: &str = "1.2.840.10008.5.1.4.34.8";
    pub const RTIonMachineVerification: &str = "1.2.840.10008.5.1.4.34.9";
}

/// [Display System Management Service Class](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_EE
///     "PS3.4 \"EE. Display System Management Service Class (Normative)\"")
pub mod svc_display {
    pub const DisplaySystem: &str = "1.2.840.10008.5.1.1.40";
    pub const DisplaySystemInstance: &str = "1.2.840.10008.5.1.1.40.1";
}

/// [Defined Procedure Protocol Query/Retrieve Service Classes](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_HH
///     "PS3.4 \"HH. Defined Procedure Protocol Query/Retrieve Service Classes\"")
pub mod svc_defined_procedure_protocol {
    pub const CTDefinedProcedureProtocolStorage: &str = "1.2.840.10008.5.1.4.1.1.200.1";
    pub const XADefinedProcedureProtocolStorage: &str = "1.2.840.10008.5.1.4.1.1.200.7";
    pub const DefinedProcedureProtocolInformationModelFind: &str = "1.2.840.10008.5.1.4.20.1";
    pub const DefinedProcedureProtocolInformationModelMove: &str = "1.2.840.10008.5.1.4.20.2";
    pub const DefinedProcedureProtocolInformationModelGet: &str = "1.2.840.10008.5.1.4.20.3";
}

/// [Protocol Approval Query/Retrieve Service Classes](
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_II
///     "PS3.4 \"II. Protocol Approval Query/Retrieve Service Classes\"")
pub mod svc_protocol_approval {
    pub const ProtocolApprovalStorage: &str = "1.2.840.10008.5.1.4.1.1.200.3";
    pub const ProtocolApprovalInformationModelFind: &str = "1.2.840.10008.5.1.4.1.1.200.4";
    pub const ProtocolApprovalInformationModelMove: &str = "1.2.840.10008.5.1.4.1.1.200.5";
    pub const ProtocolApprovalInformationModelGet: &str = "1.2.840.10008.5.1.4.1.1.200.6";
}

/// [Storage Management] and [Inventory Query/Retrieve Service] Classes
///
/// [Storage Management]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_KK
///     "PS3.4 \"KK. Storage Management Service Class\""
/// [Inventory Query/Retrieve Service]:
///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_JJ
///     "PS3.4 \"JJ. Inventory Query/Retrieve Service Class\""
pub mod svc_inventory {
    pub const InventoryStorage: &str = "1.2.840.10008.5.1.4.1.1.201.1";
    pub const InventoryFind: &str = "1.2.840.10008.5.1.4.1.1.201.2";
    pub const InventoryMove: &str = "1.2.840.10008.5.1.4.1.1.201.3";
    pub const InventoryGet: &str = "1.2.840.10008.5.1.4.1.1.201.4";
    pub const InventoryCreation: &str = "1.2.840.10008.5.1.4.1.1.201.5";
    pub const RepositoryQuery: &str = "1.2.840.10008.5.1.4.1.1.201.6";
    pub const StorageManagementInstance: &str = "1.2.840.10008.5.1.4.1.1.201.1.1";
}

/// DICOM services, that are retired too many years ago
pub mod svc_retired {

    /// **RETIRED** [Study Content Notification Service Class](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_D
    ///     "PS3.4 \"D. Study Content Notification Service Class (Normative)\"")
    pub mod svc_study_content_notification {
        /// **RETIRED** Basic Study Content Notification SOP Class (Retired)
        pub const RETIRED_BasicStudyContentNotification: &str = "1.2.840.10008.1.9";
    }

    /// **RETIRED** [Patient Management Service Class](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_E
    ///     "PS3.4 \"E. Patient Management Service Class (Normative)\"")
    pub mod svc_patient {
        pub const RETIRED_DetachedPatientManagement: &str = "1.2.840.10008.3.1.2.1.1";
        pub const RETIRED_DetachedPatientManagementMeta: &str = "1.2.840.10008.3.1.2.1.4";
        pub const RETIRED_DetachedVisitManagement: &str = "1.2.840.10008.3.1.2.2.1";
    }

    /// **RETIRED** [Results Management Service Class](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_G
    ///     "PS3.4 \"G. Results Management Service Class (Normative)\"")
    pub mod svc_results {
        pub const RETIRED_DetachedInterpretationManagement: &str = "1.2.840.10008.3.1.2.6.1";
    }

    /// **RETIRED** [Queue Management Service Class](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_L
    ///     "PS3.4 \"L. Queue Management Service Class (Normative)\"")
    pub mod svc_queue_management {
        pub const RETIRED_PrintQueueInstance: &str = "1.2.840.10008.5.1.1.25";
        pub const RETIRED_PrintQueueManagement: &str = "1.2.840.10008.5.1.1.26";
    }
}

/// [Configuration Management LDAP UIDs](
///     https://dicom.nema.org/medical/dicom/current/output/html/part15.html#chapter_H
///     "PS3.15 \"H. Application Configuration Management Profiles\"")
pub mod ldap {
    pub const dicomDeviceName: &str = "1.2.840.10008.15.0.3.1";
    pub const dicomDescription: &str = "1.2.840.10008.15.0.3.2";
    pub const dicomManufacturer: &str = "1.2.840.10008.15.0.3.3";
    pub const dicomManufacturerModelName: &str = "1.2.840.10008.15.0.3.4";
    pub const dicomSoftwareVersion: &str = "1.2.840.10008.15.0.3.5";
    pub const dicomVendorData: &str = "1.2.840.10008.15.0.3.6";
    pub const dicomAETitle: &str = "1.2.840.10008.15.0.3.7";
    pub const dicomNetworkConnectionReference: &str = "1.2.840.10008.15.0.3.8";
    pub const dicomApplicationCluster: &str = "1.2.840.10008.15.0.3.9";
    pub const dicomAssociationInitiator: &str = "1.2.840.10008.15.0.3.10";
    pub const dicomAssociationAcceptor: &str = "1.2.840.10008.15.0.3.11";
    pub const dicomHostname: &str = "1.2.840.10008.15.0.3.12";
    pub const dicomPort: &str = "1.2.840.10008.15.0.3.13";
    pub const dicomSOPClass: &str = "1.2.840.10008.15.0.3.14";
    pub const dicomTransferRole: &str = "1.2.840.10008.15.0.3.15";
    pub const dicomTransferSyntax: &str = "1.2.840.10008.15.0.3.16";
    pub const dicomPrimaryDeviceType: &str = "1.2.840.10008.15.0.3.17";
    pub const dicomRelatedDeviceReference: &str = "1.2.840.10008.15.0.3.18";
    pub const dicomPreferredCalledAETitle: &str = "1.2.840.10008.15.0.3.19";
    pub const dicomTLSCyphersuite: &str = "1.2.840.10008.15.0.3.20";
    pub const dicomAuthorizedNodeCertificateReference: &str = "1.2.840.10008.15.0.3.21";
    pub const dicomThisNodeCertificateReference: &str = "1.2.840.10008.15.0.3.22";
    pub const dicomInstalled: &str = "1.2.840.10008.15.0.3.23";
    pub const dicomStationName: &str = "1.2.840.10008.15.0.3.24";
    pub const dicomDeviceSerialNumber: &str = "1.2.840.10008.15.0.3.25";
    pub const dicomInstitutionName: &str = "1.2.840.10008.15.0.3.26";
    pub const dicomInstitutionAddress: &str = "1.2.840.10008.15.0.3.27";
    pub const dicomInstitutionDepartmentName: &str = "1.2.840.10008.15.0.3.28";
    pub const dicomIssuerOfPatientID: &str = "1.2.840.10008.15.0.3.29";
    pub const dicomPreferredCallingAETitle: &str = "1.2.840.10008.15.0.3.30";
    pub const dicomSupportedCharacterSet: &str = "1.2.840.10008.15.0.3.31";
    pub const dicomConfigurationRoot: &str = "1.2.840.10008.15.0.4.1";
    pub const dicomDevicesRoot: &str = "1.2.840.10008.15.0.4.2";
    pub const dicomUniqueAETitlesRegistryRoot: &str = "1.2.840.10008.15.0.4.3";
    pub const dicomDevice: &str = "1.2.840.10008.15.0.4.4";
    pub const dicomNetworkAE: &str = "1.2.840.10008.15.0.4.5";
    pub const dicomNetworkConnection: &str = "1.2.840.10008.15.0.4.6";
    pub const dicomUniqueAETitle: &str = "1.2.840.10008.15.0.4.7";
    pub const dicomTransferCapability: &str = "1.2.840.10008.15.0.4.8";
}

/// [Application Hosting](
///     https://dicom.nema.org/medical/dicom/current/output/html/part19.html
///     "PS3.19 \"Application Hosting\"")
pub mod app_hosting {
    pub const NativeDICOMModel: &str = "1.2.840.10008.7.1.1";
    pub const AbstractMultiDimensionalImageModel: &str = "1.2.840.10008.7.1.2";
}

/// [Real Time Commmunication](
///     https://dicom.nema.org/medical/dicom/current/output/html/part22.html
///     "PS3.22 \"Real-Time Communication\"")
pub mod real_time {
    pub const VideoEndoscopicImageRealTimeCommunication: &str = "1.2.840.10008.10.1";
    pub const VideoPhotographicImageRealTimeCommunication: &str = "1.2.840.10008.10.2";
    pub const AudioWaveformRealTimeCommunication: &str = "1.2.840.10008.10.3";
    pub const RenditionSelectionDocumentRealTimeCommunication: &str = "1.2.840.10008.10.4";
}

/// Coding schemes
pub mod coding_scheme {
    pub const DCMUID: &str = "1.2.840.10008.2.6.1";
    pub const DCM: &str = "1.2.840.10008.2.16.4";
    pub const MA: &str = "1.2.840.10008.2.16.5";
    pub const UBERON: &str = "1.2.840.10008.2.16.6";
    pub const ITIS_TSN: &str = "1.2.840.10008.2.16.7";
    pub const MGI: &str = "1.2.840.10008.2.16.8";
    pub const PUBCHEM_CID: &str = "1.2.840.10008.2.16.9";
    pub const DC: &str = "1.2.840.10008.2.16.10";
    pub const NYUMCCG: &str = "1.2.840.10008.2.16.11";
    pub const MAYONRISBSASRG: &str = "1.2.840.10008.2.16.12";
    pub const IBSI: &str = "1.2.840.10008.2.16.13";
    pub const RO: &str = "1.2.840.10008.2.16.14";
    pub const RADELEMENT: &str = "1.2.840.10008.2.16.15";
    pub const I11: &str = "1.2.840.10008.2.16.16";
    pub const UNS: &str = "1.2.840.10008.2.16.17";
    pub const RRID: &str = "1.2.840.10008.2.16.18";
}

/// [Well-known Frame of References](
///     https://dicom.nema.org/medical/dicom/current/output/html/part06.html#table_A-2
///     "PS3.6 \"Table A-2\"")
pub mod frame_of_reference {
    pub const TalairachBrainAtlas: &str = "1.2.840.10008.1.4.1.1";
    pub const SPM2T1: &str = "1.2.840.10008.1.4.1.2";
    pub const SPM2T2: &str = "1.2.840.10008.1.4.1.3";
    pub const SPM2PD: &str = "1.2.840.10008.1.4.1.4";
    pub const SPM2EPI: &str = "1.2.840.10008.1.4.1.5";
    pub const SPM2FILT1: &str = "1.2.840.10008.1.4.1.6";
    pub const SPM2PET: &str = "1.2.840.10008.1.4.1.7";
    pub const SPM2TRANSM: &str = "1.2.840.10008.1.4.1.8";
    pub const SPM2SPECT: &str = "1.2.840.10008.1.4.1.9";
    pub const SPM2GRAY: &str = "1.2.840.10008.1.4.1.10";
    pub const SPM2WHITE: &str = "1.2.840.10008.1.4.1.11";
    pub const SPM2CSF: &str = "1.2.840.10008.1.4.1.12";
    pub const SPM2BRAINMASK: &str = "1.2.840.10008.1.4.1.13";
    pub const SPM2AVG305T1: &str = "1.2.840.10008.1.4.1.14";
    pub const SPM2AVG152T1: &str = "1.2.840.10008.1.4.1.15";
    pub const SPM2AVG152T2: &str = "1.2.840.10008.1.4.1.16";
    pub const SPM2AVG152PD: &str = "1.2.840.10008.1.4.1.17";
    pub const SPM2SINGLESUBJT1: &str = "1.2.840.10008.1.4.1.18";
    pub const ICBM452T1: &str = "1.2.840.10008.1.4.2.1";
    pub const ICBMSingleSubjectMRI: &str = "1.2.840.10008.1.4.2.2";
    pub const IEC61217FixedCoordinateSystem: &str = "1.2.840.10008.1.4.3.1";
    pub const StandardRoboticArmCoordinateSystem: &str = "1.2.840.10008.1.4.3.2";
    pub const IEC61217TableTopCoordinateSystem: &str = "1.2.840.10008.1.4.3.3";
    pub const SRI24: &str = "1.2.840.10008.1.4.4.1";
    pub const Colin27: &str = "1.2.840.10008.1.4.5.1";
    pub const LPBA40AIR: &str = "1.2.840.10008.1.4.6.1";
    pub const LPBA40FLIRT: &str = "1.2.840.10008.1.4.6.2";
    pub const LPBA40SPM5: &str = "1.2.840.10008.1.4.6.3";
}

/// [Content Mapping Resource](
///     https://dicom.nema.org/medical/dicom/current/output/html/part16.html
///     "PS3.16 \"Content Mapping Resource\"")
pub mod mapping_resource {
    pub const DICOMContentMappingResource: &str = "1.2.840.10008.8.1.1";
}

/// [Synchronization Frame Of Reference](
///     https://dicom.nema.org/medical/dicom/current/output/html/part03.html#sect_C.7.4.2.1.1
///     "PS3.3 \"C.7.4.2.1.1 Synchronization Frame of Reference UID\"")
pub mod synch {
    /// UTC Synchronization Frame of Reference
    pub const UTC: &str = "1.2.840.10008.15.1.1";
}
