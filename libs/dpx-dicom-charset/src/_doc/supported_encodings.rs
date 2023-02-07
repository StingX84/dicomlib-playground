//! # Supported encodings:
//! [(< back to crate root)](crate)
//!
//! This crate supports all "levels of implementation" according to a DICOM
//! Standard [chart].
//!
//! Also, there are number of non-standard encodings implemented in the library.
//! This non-standard set has been chosen after many years of PACS server
//! development and support. It is dictated by the "encoding-unaware"
//! windows-based software coming from The Default Country (US 🤐), or even EU,
//! that runs in a localized environments.
//!
//! See the list of [built-in Encodings](crate::_doc::builtin_terms)
//!
//! Additionally, if `encoding-rs` feature is enabled, this crate will depend on
//! `encoding-rs` crate to cope with other non-standard, but widespread
//! encodings.
//!
//! ISO-2022 support is limited to the requirements from The DICOM Standard.
//! Only ESC sequences and character sets listed in the
//! [documentation](crate::_doc::iso_ir_char_sets) are supported.
//!
//! Here is a summary of DICOM-specific ISO-2022 support for those familiar with
//! its terminology:
//! - `G0` working set is always designated to `GL`;
//! - `G1` to `GR`;
//! - `G2` and `G3` are not supported;
//! - `C0` and `C1` are fixed;
//! - Shifts, Locked shifts and other special functions are not supported;
//! - G0 and G1 resets to it's original designations after:
//!   - any control character,
//!   - value separator `\` (0x5C, BACKSLASH) for `VR`'s supporting multiple
//!     values,
//!   - `^` (0x5E, CARET) and `=` (0x3D, EQUAL SIGN) for `VR` of `PN`.
//!
//! [chart]:
//!     https://dicom.nema.org/medical/dicom/current/output/html/part05.html#sect_6.1.2.5.4
//!     "PS 3.5 \"6.1.2.5.4. Levels of Implementation and Initial Designation\""
//! [control characters]:
//!     https://dicom.nema.org/medical/dicom/current/output/html/part05.html#sect_6.1.3
//!     "PS 3.5 \"6.1.3 Control Characters\""
