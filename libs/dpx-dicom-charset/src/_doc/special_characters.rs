//! # Special characters:
//! [(< back to crate root)](crate)
//!
//! ## Control characters
//!
//! The DICOM Standard allows only some of [control characters]: `LF`, `CR`,
//! `TAB` and `FF` and only for some well defined list of `VR`'s: `ST`, `LT` and
//! `UT`.
//!
//! There are also some other forbidden characters:
//! - `0x7F` \
//!   From [PS3.5 "6.1.2.3 Encoding of Character
//!   Repertoires"](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_6.html#sect_6.1.2.3):
//!   > The character DELETE (bit combination 07/15) shall not be used in DICOM
//!   > character strings.
//!
//! - `0x80` to `0x9F` \
//!   From [PS3.5 "6.1.1 Representation of Encoded Character
//!   Values"](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_6.html#sect_6.1.1):
//!   > Only some Control Characters from the C0 set are used in DICOM (see
//!   > Section 6.1.3), and characters from the C1 set shall not be used.
//!
//! This crate does not restrict usage of any C0 or C1 characters and passes
//! them as is to minimize risk of data loss if dealing with datasets coming
//! from "buggy" or encoding unaware software.
//!
//! ## Special `ISO_IR 6` rules:
//!
//! The Standard states, that when `(0008,0005) Specific Character Set` attribute
//! is not present, then only `G0` table is designated with `ISO-IR 6` (ASCII).
//! When it present
//!
//!  has an empty value or is equal to `ISO_IR 6`, the ""
//!
//! When [Codec] initialized with a single-valued
//! [Term::IsoIr6] or [Term::Iso2022Ir6], it will
//!
//! This crate tries to minimize potential risks of text attributes mangling and
//! allows any control characters. For example in a default configuration, given
//! single-valued Specific Character Set with a value of "ISO_IR 6" text encoder
//! and decoder will directly translate all 8-bit characters into a
//! corresponding unicode code points. Note, that it is not true for
//! multi-valued character set attribute! See
//! [Config::set_g1_for_iso_ir_6](crate::Config::set_g1_for_iso_ir_6).
//!
//! All the compatibility features, that deviates from The Standard requirements
//! may be found in [Config](crate::Config) members documentation.
//!
//! [control characters]:
//!     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_6.html#sect_6.1.3
//!     "PS3.5 \"6.1.3. Control Characters\""
//! [ISO IR 6]: https://itscj.ipsj.or.jp/ir/006.pdf
//! [ISO IR 14]: https://itscj.ipsj.or.jp/ir/014.pdf
//! [default repertoire]:
//!     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_E.html
//!     "PS3.5 \"E DICOM Default Character Repertoire (Normative)\""
