//! Static registry of DICOM [Transfer Syntaxes](TransferSyntax).
//!
//! Every standard Transfer Syntax UID from [`uids::ts`](crate::uids::ts) is
//! available here as an associated constant of [`TransferSyntax`] (named after
//! its DICOM keyword) and collected in [`ALL`]. The registry is fixed at compile
//! time and not user-extensible; look an entry up by its UID with
//! [`TransferSyntax::from_uid`].
//!
//! The human-readable name of a Transfer Syntax is not stored here; obtain it
//! from the UID dictionary via [`Uid::name`](crate::Uid::name).

#![allow(non_upper_case_globals)]

use crate::uids;

/// A DICOM Transfer Syntax and the stream properties needed to (de)serialize it.
///
/// Obtain one from the registry with [`from_uid`](Self::from_uid) or use a
/// well-known associated constant. The set of syntaxes is fixed by the standard
/// and cannot be extended at run time. The flags mirror those classified by
/// [`UidType::TransferSyntax`](crate::uid::UidType::TransferSyntax).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferSyntax {
    /// The Transfer Syntax UID, e.g. `"1.2.840.10008.1.2"`.
    pub uid: &'static str,
    /// Numeric values are little-endian (`true`) or big-endian (`false`).
    pub is_little_endian: bool,
    /// Each element carries an explicit Value Representation.
    pub is_explicit_vr: bool,
    /// The main data set is raw-DEFLATE compressed. The reader inflates it before
    /// parsing; the parser core never sees this set.
    pub is_compressed: bool,
    /// The Pixel Data is encapsulated in a compressed codec.
    pub is_encapsulated: bool,
}

impl TransferSyntax {
    /// Looks up a Transfer Syntax in the registry by its UID string. Returns
    /// `None` for an unknown UID.
    pub fn from_uid(uid: &str) -> Option<&'static TransferSyntax> {
        ALL.iter().find(|ts| ts.uid == uid)
    }
}

/// Declares one `TransferSyntax` associated constant per DICOM keyword (reusing
/// the UID string from [`uids::ts`]) and emits the [`ALL`] registry over them.
macro_rules! transfer_syntaxes {
    ($(
        $keyword:ident = ($le:literal, $evr:literal, $comp:literal, $enc:literal)
    ),* $(,)?) => {
        ::place_macro::place! {
            impl TransferSyntax {
                $(
                    #[doc = __str__("`" $keyword "` Transfer Syntax.")]
                    pub const $keyword: TransferSyntax = TransferSyntax {
                        uid: uids::ts::$keyword,
                        is_little_endian: $le,
                        is_explicit_vr: $evr,
                        is_compressed: $comp,
                        is_encapsulated: $enc,
                    };
                )*
            }

            /// The complete, fixed registry of standard Transfer Syntaxes.
            pub static ALL: &[TransferSyntax] = &[ $( TransferSyntax::$keyword, )* ];
        }
    };
}

transfer_syntaxes! {
    // keyword                                          le     evr    comp   enc
    ImplicitVRLittleEndian                          = (true , false, false, false),
    ExplicitVRLittleEndian                          = (true , true , false, false),
    EncapsulatedUncompressedExplicitVRLittleEndian  = (true , true , false, true ),
    DeflatedExplicitVRLittleEndian                  = (true , true , true , false),
    ExplicitVRBigEndian                             = (false, true , false, false),
    JPEGBaseline8Bit                                = (true , true , false, true ),
    JPEGExtended12Bit                               = (true , true , false, true ),
    RETIRED_JPEGExtended35                          = (true , true , false, true ),
    RETIRED_JPEGSpectralSelectionNonHierarchical68  = (true , true , false, true ),
    RETIRED_JPEGSpectralSelectionNonHierarchical79  = (true , true , false, true ),
    RETIRED_JPEGFullProgressionNonHierarchical1012  = (true , true , false, true ),
    RETIRED_JPEGFullProgressionNonHierarchical1113  = (true , true , false, true ),
    JPEGLossless                                    = (true , true , false, true ),
    RETIRED_JPEGLosslessNonHierarchical15           = (true , true , false, true ),
    RETIRED_JPEGExtendedHierarchical1618            = (true , true , false, true ),
    RETIRED_JPEGExtendedHierarchical1719            = (true , true , false, true ),
    RETIRED_JPEGSpectralSelectionHierarchical2022   = (true , true , false, true ),
    RETIRED_JPEGSpectralSelectionHierarchical2123   = (true , true , false, true ),
    RETIRED_JPEGFullProgressionHierarchical2426     = (true , true , false, true ),
    RETIRED_JPEGFullProgressionHierarchical2527     = (true , true , false, true ),
    RETIRED_JPEGLosslessHierarchical28              = (true , true , false, true ),
    RETIRED_JPEGLosslessHierarchical29              = (true , true , false, true ),
    JPEGLosslessSV1                                 = (true , true , false, true ),
    JPEGLSLossless                                  = (true , true , false, true ),
    JPEGLSNearLossless                              = (true , true , false, true ),
    JPEG2000Lossless                                = (true , true , false, true ),
    JPEG2000                                        = (true , true , false, true ),
    JPEG2000MCLossless                              = (true , true , false, true ),
    JPEG2000MC                                      = (true , true , false, true ),
    JPIPReferenced                                  = (true , true , false, true ),
    JPIPReferencedDeflate                           = (true , true , true , true ),
    MPEG2MPML                                       = (true , true , false, true ),
    MPEG2MPMLF                                      = (true , true , false, true ),
    MPEG2MPHL                                       = (true , true , false, true ),
    MPEG2MPHLF                                      = (true , true , false, true ),
    MPEG4HP41                                       = (true , true , false, true ),
    MPEG4HP41F                                      = (true , true , false, true ),
    MPEG4HP41BD                                     = (true , true , false, true ),
    MPEG4HP41BDF                                    = (true , true , false, true ),
    MPEG4HP422D                                     = (true , true , false, true ),
    MPEG4HP422DF                                    = (true , true , false, true ),
    MPEG4HP423D                                     = (true , true , false, true ),
    MPEG4HP423DF                                    = (true , true , false, true ),
    MPEG4HP42STEREO                                 = (true , true , false, true ),
    MPEG4HP42STEREOF                                = (true , true , false, true ),
    HEVCMP51                                        = (true , true , false, true ),
    HEVCM10P51                                      = (true , true , false, true ),
    RLELossless                                     = (true , true , false, true ),
    RETIRED_RFC2557MIMEEncapsulation                = (true , true , false, true ),
    RETIRED_XMLEncoding                             = (true , true , false, true ),
    SMPTEST211020UncompressedProgressiveActiveVideo = (true , true , false, true ),
    SMPTEST211020UncompressedInterlacedActiveVideo  = (true , true , false, true ),
    SMPTEST211030PCMDigitalAudio                    = (true , true , false, true ),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uid::{META_LIST_DICOM, UidType};

    #[test]
    fn from_uid_resolves_known_and_rejects_unknown() {
        assert_eq!(TransferSyntax::from_uid("1.2.840.10008.1.2"), Some(&TransferSyntax::ImplicitVRLittleEndian));
        let rle = TransferSyntax::from_uid(uids::ts::RLELossless).expect("RLE in registry");
        assert!(rle.is_encapsulated && rle.is_explicit_vr && rle.is_little_endian);
        assert!(TransferSyntax::from_uid("9.9.9.9.9").is_none());
    }

    /// Every UID the dictionary classifies as a Transfer Syntax must have a
    /// registry entry, and vice versa: the two hand-maintained lists stay paired.
    #[test]
    fn registry_and_dictionary_cover_the_same_uids() {
        for m in META_LIST_DICOM.value() {
            if matches!(m.uid_type, UidType::TransferSyntax) {
                assert!(TransferSyntax::from_uid(m.uid.as_str()).is_some(), "registry missing TS {}", m.uid);
            }
        }
        for ts in ALL {
            assert!(META_LIST_DICOM.value().iter().any(|m| m.uid.as_str() == ts.uid), "dictionary missing TS {}", ts.uid);
        }
    }
}
