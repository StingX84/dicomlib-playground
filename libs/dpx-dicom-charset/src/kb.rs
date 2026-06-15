//! Knowledge-base entry constants for the `dpx-dicom-charset` crate.
//!
//! Each constant corresponds to a documented entry in `docs/knowledge-base.md`.
//! Pass them to [`dicom_err!`](dpx_dicom_core::dicom_err) via the `kb:` argument:
//!
//! ```
//! use dpx_dicom_charset::kb;
//! use dpx_dicom_core::dicom_err;
//! let _err = dicom_err!(InvalidData, kb: kb::DS_0001, "specific character set is empty");
//! ```

use dpx_dicom_core::KbEntry;

pub const DS_0001: KbEntry = KbEntry { id: "dpxkb_ds_0001", title: "Empty character set" };
pub const DS_0002: KbEntry = KbEntry { id: "dpxkb_ds_0002", title: "Unknown encoding in character set" };
pub const DS_0003: KbEntry = KbEntry { id: "dpxkb_ds_0003", title: "Non-standard encoding in character set" };
pub const DS_0004: KbEntry = KbEntry { id: "dpxkb_ds_0004", title: "Non-standard encoding accepted" };
pub const DS_0005: KbEntry = KbEntry { id: "dpxkb_ds_0005", title: "Non-ISO-2022 encoding in multi-valued character set" };
pub const DS_0006: KbEntry = KbEntry { id: "dpxkb_ds_0006", title: "First value is multi-byte in multi-valued character set" };
pub const DS_0007: KbEntry = KbEntry { id: "dpxkb_ds_0007", title: "Aliased encoding name accepted" };
pub const DS_0008: KbEntry = KbEntry { id: "dpxkb_ds_0008", title: "Empty value ignored in multi-valued character set" };
pub const DS_0009: KbEntry = KbEntry { id: "dpxkb_ds_0009", title: "Duplicate value ignored in multi-valued character set" };
pub const DS_0010: KbEntry = KbEntry { id: "dpxkb_ds_0010", title: "Empty value in multi-valued character set" };
pub const DS_0011: KbEntry = KbEntry { id: "dpxkb_ds_0011", title: "Duplicate value in multi-valued character set" };
pub const DS_0012: KbEntry = KbEntry { id: "dpxkb_ds_0012", title: "SingleByteWithoutExtensions promoted in multi-valued character set" };
