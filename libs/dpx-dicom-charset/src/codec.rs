use crate::{
    ascii::SliceExt,
    tables::constants::*,
    term::{CodecType, Term, TermKind, TermMatchedWith},
};
use std::borrow::Cow;
use tracing::warn;

#[cfg(feature = "encoding_rs")]
use encoding_rs::Encoding;

// cSpell::ignore noext worklist Привет Բարեւ fffd

#[cfg(feature = "encoding_rs")]
mod external_impl;
mod iso2022_impl;
mod iso2022_simple_impl;
mod non_iso2022_impl;
mod utf8_impl;

/// A DICOM-specific text encoder and decoder
///
/// This Codec implements all the requirements of the DICOM Standard described
/// in [PS3.3] and [PS3.5]. Plus some additional features to be as widely
/// compatible with other software as possible.
///
/// You can find detailed information on handling of `(0008,0005) Specific
/// Character Set` attribute in
/// [from_specific_character_set](fn@Self::from_specific_character_set) function
/// documentation.
///
/// [PS3.3]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.12.html#sect_C.12.1.1.2
///     "PS3.3 \"C.12.1.1.2. Specific Character Set\""
/// [PS3.5]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_6.html#sect_6.1
///     "PS3.5 \"6.1. Support of Character Repertoires\""
#[derive(Debug, Clone)]
pub struct Codec {
    terms: Vec<Term>,
    config: Config,
    chosen_impl: ChosenImpl,
    #[cfg(feature = "encoding_rs")]
    external: Option<&'static Encoding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChosenImpl {
    Utf8,
    Iso2022Simple,
    Iso2022Extended,
    NonIso2022,
    #[cfg(feature = "encoding_rs")]
    External,
}

/// A structure representing a `VR`'s properties for encoding and decoding
/// operations
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Enables special handling of a `\` BACKSLASH character when text is
    /// being encoded or decoded.
    ///
    /// Caller enables this option for all textual attributes except `LT`, `ST`,
    /// `UR` and `UT`.
    ///
    /// Note: some special cases may require the application to disable this
    /// option when it otherwise should be enabled. For example, when medical
    /// organization uses `\` BACKSLASH in their patient record numbering, which
    /// ends up in a PatientID attribute. Such attributes will definitely
    /// confuse a PACS server.
    pub is_multi_valued: bool,

    /// Enables special handling of '^' CARET and '=' EQUALS characters text is
    /// being encoded or decoded.
    ///
    /// Caller enables this option only for `PN` attributes.
    pub is_pn: bool,
}

#[derive(Clone, Copy)]
pub struct ReplacementFn(pub for<'a> fn(&'a [u8]) -> Cow<'static, str>);

/// Configuration for [Codec] instantiation
///
/// Note, that a configuration created with [new](Self::new) or [Default] trait
/// is tuned for maximum interoperability rather than strictly following the
/// standard. However, this implies, that even if application can successfully
/// open-modify-write externally created datasets, other applications not using
/// this library will fail to process these datasets.
///
/// To construct a [Config], that enforces the [Codec] to be much more
/// restrictive, use [new_restrictive](Self::new_restrictive) struct method.
#[derive(Debug, Clone)]
pub struct Config {
    /// Enables the library to accept non-standard terms for standard
    /// encodings.
    ///
    /// For example, `ISO-8859-1` will be treated as `ISO_IR 100`.
    ///
    /// See also this [table](crate::_doc::builtin_terms).
    pub allow_encoding_aliases: bool,

    /// Allows the library to treat
    /// [SingleByteWithoutCodeExtensions](crate::TermKind::SingleByteWithoutCodeExtensions)
    /// terms in multi-valued `Specific Character Set` as if they were
    /// [SingleByteWithCodeExtensions](crate::TermKind::SingleByteWithCodeExtensions).
    ///
    /// This will allow rather invalid terms like `ISO_IR 6\ISO_IR 100` to be
    /// processed as `ISO 2022 IR 6\ISO 2022 IR 100`.
    pub allow_iso2022_non_extensible_term_in_multi_valued_charset: bool,

    /// Enables the library to accept non-standard encodings and their aliases.
    ///
    /// For example, `cp1251` will become acceptable encoding.
    ///
    /// For the reference of the non-standard encodings supported by the
    /// library, check the [crate] documentation.
    pub allow_non_standard_encodings: bool,

    /// Disables [tracing::warn!] message when some problems detected within
    /// [Codec::from_specific_character_set] function.
    pub disable_tracing: bool,

    /// Enables the library to ignore duplicate and empty values in the
    /// multi-valued specific character set IF this will not transform
    /// a character set into single-valued.
    pub ignore_multi_value_duplicates: bool,

    /// Function used to produce a replacement character in [Codec::decode]
    /// function when invalid input byte encountered.
    ///
    /// On input, this function receives 1 or more bytes, that are treated
    /// as invalid input. For example, this might be an invalid ESC sequence.
    ///
    /// Default implementation, ignores input and always returns a single
    /// character with code `\u{FFFD}`.
    pub replacement_character_fn: ReplacementFn,

    /// Default term for `G1` region if specific character set is a
    /// single-valued `ISO_IR 6` or `ISO 2022 IR 6`.
    ///
    /// This option allows to override `G1` (upper half of code page) to a
    /// specific code page. By default, this options is `None` which makes codec
    /// translated non-ASCII bytes 1 to 1 to unicode code points. This allows to
    /// recover from character set misinterpretation when processing datasets
    /// originating from some "buggy" software.
    ///
    /// You can set this option to any ISO-2022 compatible `ISO IR` [Term]
    /// (whose [Term::kind()] is
    /// [SingleByteWithoutCodeExtensions](crate::TermKind::SingleByteWithoutCodeExtensions),
    /// [SingleByteWithCodeExtensions](crate::TermKind::SingleByteWithCodeExtensions)
    /// or
    /// [MultiByteWithCodeExtensions](crate::TermKind::MultiByteWithCodeExtensions)).
    ///
    /// Note, that [Term::Iso2022Ir87] and [Term::Iso2022Ir159] does not
    /// designate `G1` region. Better do not use them and leave `None` here.
    ///
    /// Afterthoughts: `ISO_IR 6` term defined in The Standard leaves `G0` not
    /// designated, which will lead to a data loss when processing text
    /// attributes created by some "encoding-unaware" application. This option
    /// makes the library deviate from The Standard, but it is better to retain
    /// a data, rather than lose it entirely. For example, this approach allows
    /// PACS server to post process and fix such "buggy" datasets later, when
    /// the problem has been noticed by the personal.
    pub set_g1_for_iso_ir_6: Option<Term>,

    /// Instructs a library to use more recent version of the code pages, then
    /// stated in the Standard.
    ///
    /// This allows an application to slightly deviate from a DICOM standard in
    /// such a way to be more compatible with modern computer systems. Because,
    /// most of the DICOM libraries are simply using OS-provided, ICU, iconv or
    /// [whatwg] based libraries to perform charset conversions, with a simple
    /// DICOM-to-ISO mappings, for example, maps `ISO_IR 126` to `ISO 8859-8`,
    /// which will use modern character set [`IR 227`] instead of outdated, but
    /// DICOM specified [`IR 126`].
    ///
    /// # Modern code page variants:
    /// - for [IsoIr126](crate::term::Term::IsoIr126) and
    ///   [Iso2022Ir126](crate::term::Term::Iso2022Ir126):
    ///   - Standard page: [`IR 126`] (Greek, 1986)
    ///   - Modern page: [`IR 227`] (Greek, 2003)
    /// - for [IsoIr138](crate::term::Term::IsoIr138) and
    ///   [Iso2022Ir138](crate::term::Term::Iso2022Ir138):
    ///   - Standard page: [`IR 138`] (Hebrew, 1987)
    ///   - Modern page: [`IR 234`] (Hebrew, 2004)
    ///
    /// All other dicom terms has no "modern" alternatives.
    ///
    /// [whatwg]: https://encoding.spec.whatwg.org
    /// [`IR 126`]: https://itscj.ipsj.or.jp/ir/126.pdf
    /// [`IR 227`]: https://itscj.ipsj.or.jp/ir/227.pdf
    /// [`IR 138`]: https://itscj.ipsj.or.jp/ir/138.pdf
    /// [`IR 234`]: https://itscj.ipsj.or.jp/ir/234.pdf
    pub use_modern_code_page: bool,
}

impl std::fmt::Debug for ReplacementFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<function pointer>")
    }
}

impl Context {
    pub const fn new(has_delimiter: bool, is_pn: bool) -> Self {
        Self {
            is_multi_valued: has_delimiter,
            is_pn,
        }
    }
}

impl Config {
    fn default_replacement_fn(_: &[u8]) -> Cow<'static, str> {
        Cow::Borrowed("\u{FFFD}")
    }

    /// Constructs a configuration object for optimal compatibility features
    /// enabled
    ///
    /// Options enabled:
    /// - [allow_encoding_aliases](Self::allow_encoding_aliases)
    /// - [allow_iso2022_non_extensible_term_in_multi_valued_charset](Self::allow_iso2022_non_extensible_term_in_multi_valued_charset)
    /// - [allow_non_standard_encodings](Self::allow_non_standard_encodings)
    /// - [ignore_multi_value_duplicates](Self::ignore_multi_value_duplicates)
    /// - [use_modern_code_page](Self::use_modern_code_page)
    pub const fn new() -> Self {
        Self {
            allow_encoding_aliases: true,
            allow_iso2022_non_extensible_term_in_multi_valued_charset: true,
            allow_non_standard_encodings: true,
            disable_tracing: false,
            ignore_multi_value_duplicates: true,
            replacement_character_fn: ReplacementFn(Self::default_replacement_fn),
            set_g1_for_iso_ir_6: None,
            use_modern_code_page: true,
        }
    }

    /// Constructs a configuration object to more strictly follow The Standard.
    pub const fn new_restrictive() -> Self {
        Self {
            allow_encoding_aliases: false,
            allow_iso2022_non_extensible_term_in_multi_valued_charset: false,
            allow_non_standard_encodings: false,
            disable_tracing: false,
            ignore_multi_value_duplicates: false,
            replacement_character_fn: ReplacementFn(Self::default_replacement_fn),
            set_g1_for_iso_ir_6: None,
            use_modern_code_page: false,
        }
    }
}

impl Default for Config {
    /// Creates compatibility oriented configuration.
    ///
    /// Same as [new()](Config::new) method
    fn default() -> Self {
        Self::new()
    }
}

/// Parser helper structure for multi-valued sub-parser
enum ParseMultiFirst {
    /// Parsing should continue with next value
    Accept(Term, u16),
    /// Abort the process with error
    Fail(u16),
}

/// Parser helper structure for multi-valued sub-parser
enum ParseMultiOthers {
    /// Parsing should continue with next value
    Accept(Term, u16),
    /// Current value should be ignored
    Ignore(u16),
    /// Abort the process with error
    Fail(u16),
}

// Final failure reasons of the parser
/// `#dpxkb_ds_0001` - Empty character set
const FAIL_DPXKB_DS_0001_EMPTY_CHAR_SET: u16 = 1 << 0;
/// `#dpxkb_ds_0002` - Unknown encoding in character set
const FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING: u16 = 1 << 1;
/// `#dpxkb_ds_0003` - Non standard encoding in character set
const FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING: u16 = 1 << 2;
/// `#dpxkb_ds_0005` - Encoding in the multi-valued character set string does not support ISO-2022 extensions
const FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022: u16 = 1 << 3;
/// `#dpxkb_ds_0006` - First encoding is Multi-Byte in the multi-valued character set
const FAIL_DPXKB_DS_0006_FIRST_IS_MULTI_BYTE: u16 = 1 << 4;
/// `#dpxkb_ds_0010` - Empty value in multi-valued specific character set
const FAIL_DPXKB_DS_0010_EMPTY_VALUE_IN_MULTI_VALUED: u16 = 1 << 5;
/// `#dpxkb_ds_0011` - Duplicate value in multi-valued specific character set
const FAIL_DPXKB_DS_0011_DUPLICATE_VALUE_IN_MULTI_VALUED: u16 = 1 << 6;

// Warning flags set during parsing
/// `#dpxkb_ds_0004` - Non standard encoding accepted in character set
const WARN_DPXKB_DS_0004_ACCEPTED_NON_STANDARD_ENCODING: u16 = 1 << 8;
/// `#dpxkb_ds_0007` - Non standard encoding aliased name accepted in character set
const WARN_DPXKB_DS_0007_ACCEPTED_ALIAS: u16 = 1 << 9;
/// `#dpxkb_ds_0008` - Ignored empty value in multi-valued specific character set
const WARN_DPXKB_DS_0008_IGNORED_EMPTY_VALUED: u16 = 1 << 10;
/// `#dpxkb_ds_0009` - Ignored duplicate value in multi-valued specific character set
const WARN_DPXKB_DS_0009_IGNORED_DUPLICATE_VALUED: u16 = 1 << 11;
/// `#dpxkb_ds_0012` - Promoted SingleByteWithoutExtensions to SingleByteWithExtensions in multi valued character set
const WARN_DPXKB_DS_0012_SINGLE_BYTE_WITHOUT_EXTENSIONS_PROMOTED: u16 = 1 << 12;

const DPXKB_MAP: &[(u16, &str, &str)] = &[
    (
        FAIL_DPXKB_DS_0001_EMPTY_CHAR_SET,
        "#dpxkb_ds_0001",
        "Empty character set",
    ),
    (
        FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING,
        "#dpxkb_ds_0002",
        "Unknown encoding in character set",
    ),
    (
        FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING,
        "#dpxkb_ds_0003",
        "Non standard encoding in character set",
    ),
    (
        FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022,
        "#dpxkb_ds_0005",
        "Non ISO-2022 encoding in multi-valued character set",
    ),
    (
        FAIL_DPXKB_DS_0006_FIRST_IS_MULTI_BYTE,
        "#dpxkb_ds_0006",
        "First encoding is Multi-Byte in multi-valued character set",
    ),
    (
        FAIL_DPXKB_DS_0010_EMPTY_VALUE_IN_MULTI_VALUED,
        "#dpxkb_ds_0010",
        "Empty value in multi-valued character set",
    ),
    (
        FAIL_DPXKB_DS_0011_DUPLICATE_VALUE_IN_MULTI_VALUED,
        "#dpxkb_ds_0011",
        "Duplicate value in multi-valued character set",
    ),
    (
        WARN_DPXKB_DS_0004_ACCEPTED_NON_STANDARD_ENCODING,
        "#dpxkb_ds_0004",
        "non standard term",
    ),
    (
        WARN_DPXKB_DS_0007_ACCEPTED_ALIAS,
        "#dpxkb_ds_0007",
        "term alias",
    ),
    (
        WARN_DPXKB_DS_0008_IGNORED_EMPTY_VALUED,
        "#dpxkb_ds_0008",
        "empty value",
    ),
    (
        WARN_DPXKB_DS_0009_IGNORED_DUPLICATE_VALUED,
        "#dpxkb_ds_0009",
        "duplicate value",
    ),
    (
        WARN_DPXKB_DS_0012_SINGLE_BYTE_WITHOUT_EXTENSIONS_PROMOTED,
        "#dpxkb_ds_0012",
        "'ISO_IR' as 'ISO 2022 IR'",
    ),
];

impl Codec {
    /// Creates a codec with a modern-world default [Term::IsoIr192] (UTF-8) term.
    pub fn new() -> Self {
        Self {
            terms: vec![Term::IsoIr192],
            config: Config::new(),
            chosen_impl: ChosenImpl::Utf8,
            #[cfg(feature = "encoding_rs")]
            external: None,
        }
    }

    /// Constructs a codec from the the content of `(0008,0005) Specific
    /// Character Set` attribute.
    ///
    /// # Params:
    /// - `specific_character_set` - Single or multi-valued string with DICOM
    ///   terms. See DICOM [PS3.3 "C.12.1.1.2 Specific Character Set"](
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.12.html#sect_C.12.1.1.2)
    /// - `config` - Configuration options for the encoder and decoder.
    ///
    /// # Returns:
    /// A [Codec] object capable of encoding and decoding strings. When method
    /// fails to interpret the input `specific_character_set`, it sets resulting
    /// term to [Term::Unknown], which allows the codec to be as transparent as
    /// possible to prevent irreversible data loss.
    ///
    /// # Failures:
    /// - Input string is empty. See `#dpxkb_ds_0001`;
    /// - Input string contains an unknown encoding. See `#dpxkb_ds_0002`;
    /// - Input string contains a non-standard encoding and
    ///   [Config::allow_non_standard_encodings] was not enabled. See
    ///   `#dpxkb_ds_0003`;
    /// - Input is multi valued and one of the values does not support ISO-2022
    ///   extensions. If offending term was one of
    ///   [SingleByteWithoutCodeExtensions](TermKind::SingleByteWithoutCodeExtensions)
    ///   such as [Term::IsoIr100], a configuration option
    ///   [Config::allow_iso2022_non_extensible_term_in_multi_valued_charset]
    ///   may be used to accept such terms. Non-standard and Non ISO-2022
    ///   encodings will never be accepted in this context. See
    ///   `#dpxkb_ds_0005`;
    /// - Input is multi valued and the first value is one of
    ///   [MultiByteWithCodeExtensions](TermKind::MultiByteWithCodeExtensions)
    ///   terms. See `#dpxkb_ds_0006`;
    /// - Input is multi valued and empty value seen at position other than the
    ///   first. Such a values can be ignored if
    ///   [Config::ignore_multi_value_duplicates] option is enabled. See
    ///   `#dpxkb_ds_0010`;
    /// - Input is multi valued and some values are parsed as identical
    ///   [Term]'s. This could arise when the values are literally identical,
    ///   de-aliasing (if [allowed](Config::allow_encoding_aliases)) or
    ///   [SingleByteWithoutCodeExtensions](TermKind::SingleByteWithoutCodeExtensions)
    ///   promotion to
    ///   [SingleByteWithCodeExtensions](TermKind::SingleByteWithCodeExtensions)
    ///   results in the identical [Term]'s. See `#dpxkb_ds_0011`.
    ///
    /// # Warnings:
    /// - Accepted non-standard encoding in a single valued input. This
    ///   was allowed by [Config::allow_non_standard_encodings]. See `#dpxkb_ds_0004`;
    /// - Accepted an alias to the standard encoding. This was allowed
    ///   by [Config::allow_encoding_aliases]. See `#dpxkb_ds_0007`;
    /// - Empty value in the multi valued input was ignored. This was allowed
    ///   by [Config::ignore_multi_value_duplicates]. See `#dpxkb_ds_0008`;
    /// - Duplicate value in the multi valued input was ignored. This was
    ///   allowed by [Config::ignore_multi_value_duplicates]. See `#dpxkb_ds_0009`;
    /// - Term [SingleByteWithoutCodeExtensions](TermKind::SingleByteWithoutCodeExtensions)
    ///   was promoted to [SingleByteWithCodeExtensions](TermKind::SingleByteWithCodeExtensions)
    ///   in multi valued input. This was allowed by [Config::allow_iso2022_non_extensible_term_in_multi_valued_charset].
    ///   See `#dpxkb_ds_0012`.
    ///
    /// # Tracing:
    /// This function may emit `warning` trace message with a "dpxkb" property
    /// set to one of `#dpxkb_ds_0001` to `#dpxkb_ds_0012`.
    /// It is emitted, when "Failure" and/or "Warning" condition (see above)
    /// was met. To disable any logging, set [Config::disable_tracing] to `true`.
    ///
    /// To "reconstruct" back the `Specific Character Set` attribute value from
    /// this object, use
    /// [`specific_character_set`](fn@Self::specific_character_set) function.
    pub fn from_specific_character_set(specific_character_set: &[u8], config: Config) -> Self {
        let specific_character_set = specific_character_set.trim_spaces();

        if specific_character_set.is_empty() {
            return Self::parse_failed(
                specific_character_set,
                config,
                FAIL_DPXKB_DS_0001_EMPTY_CHAR_SET,
            );
        }

        if !specific_character_set.contains(&CODE_VALUES_SEPARATOR) {
            return Self::parse_single_valued(specific_character_set, config);
        }

        let mut values_iterator = specific_character_set.split(|&c| c == CODE_VALUES_SEPARATOR);
        let mut term_list: Vec<Term> = Vec::new();

        // UNWRAP SAFETY: trim will always return at least one item even if input were empty!
        let first_value = values_iterator.next().unwrap().trim_spaces_end();
        let mut warnings: u16 = 0;

        match Self::parse_multi_valued_first_value(first_value, &config) {
            ParseMultiFirst::Accept(term, w) => {
                term_list.push(term);
                warnings |= w;
            }
            ParseMultiFirst::Fail(w) => {
                return Self::parse_failed(specific_character_set, config, w);
            }
        }

        for term_string in values_iterator {
            let term_string = term_string.trim_spaces();

            match Self::parse_multi_valued_next_value(term_string, &config, &term_list) {
                ParseMultiOthers::Accept(term, w) => {
                    term_list.push(term);
                    warnings |= w;
                }
                ParseMultiOthers::Ignore(w) => warnings |= w,
                ParseMultiOthers::Fail(w) => {
                    return Self::parse_failed(specific_character_set, config, w);
                }
            }
        }

        if term_list.len() < 2 {
            if warnings & WARN_DPXKB_DS_0008_IGNORED_EMPTY_VALUED != 0 {
                warnings |= FAIL_DPXKB_DS_0010_EMPTY_VALUE_IN_MULTI_VALUED;
            } else if warnings & WARN_DPXKB_DS_0009_IGNORED_DUPLICATE_VALUED != 0 {
                warnings |= FAIL_DPXKB_DS_0011_DUPLICATE_VALUE_IN_MULTI_VALUED;
            } else {
                unreachable!()
            }
            return Self::parse_failed(specific_character_set, config, warnings);
        }

        Self::parsed_terms(specific_character_set, config, warnings, term_list)
    }

    /// Internal helper function, that emits [tracing::warn!] when some problems
    /// found in the specific character set parser.
    fn emit_parse_warnings(self, source: &[u8], warnings: u16) -> Self {
        if warnings != 0 && !self.config.disable_tracing {
            let mut warning_strings = Vec::<&'static str>::new();
            let mut dpxkb = Vec::<&'static str>::new();
            let is_failed = (warnings & 0xFF) != 0;
            let mut warnings = if is_failed { warnings & 0xFF } else { warnings };

            for m in DPXKB_MAP {
                if warnings & m.0 == m.0 {
                    warning_strings.push(m.2);
                    dpxkb.push(m.1);
                    warnings ^= m.0;
                }
            }
            debug_assert_eq!(warnings, 0, "All the flags should be processed!");

            let actual_char_set = self.specific_character_set();
            let message = if is_failed {
                if !source.is_empty() {
                    format!(
                        "{} \"{}\"",
                        warning_strings.join(", "),
                        String::from_utf8_lossy(source)
                    )
                } else {
                    warning_strings.join(", ")
                }
            } else if actual_char_set.as_bytes() != source {
                format!(
                    "character set \"{}\" accepted as \"{}\" ({})",
                    String::from_utf8_lossy(source),
                    actual_char_set,
                    warning_strings.join(", ")
                )
            } else {
                format!(
                    "Accepted {} in character set \"{}\"",
                    warning_strings.join(", "),
                    String::from_utf8_lossy(source),
                )
            };
            warn!(?dpxkb, message);
        }

        self
    }

    fn parse_failed(term_string: &[u8], config: Config, warnings: u16) -> Self {
        Self::parsed_terms(term_string, config, warnings, vec![Term::Unknown])
    }

    /// Parser helper function, that creates a struct instance
    fn parsed_terms(term_string: &[u8], config: Config, warnings: u16, terms: Vec<Term>) -> Self {
        let chosen_impl = Self::choose_impl(&terms, &config);
        Self {
            terms,
            config,
            chosen_impl,
            #[cfg(feature = "encoding_rs")]
            external: None,
        }
        .emit_parse_warnings(term_string, warnings)
    }

    /// Part of the specific character set parser: parses the single-valued
    /// input text Result of this function is directly returned from
    /// [from_specific_character_set]
    fn parse_single_valued(term_string: &[u8], config: Config) -> Self {
        if let Some((term, matched_with)) = Term::search_by_keyword(term_string) {
            let mut warnings: u16 = 0;
            if !term.is_standard_dicom() {
                if !config.allow_non_standard_encodings {
                    return Self::parse_failed(
                        term_string,
                        config,
                        FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING,
                    );
                }
                warnings |= WARN_DPXKB_DS_0004_ACCEPTED_NON_STANDARD_ENCODING;
            } else if matched_with != TermMatchedWith::Primary {
                if !config.allow_encoding_aliases {
                    return Self::parse_failed(
                        term_string,
                        config,
                        FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING,
                    );
                }
                warnings |= WARN_DPXKB_DS_0007_ACCEPTED_ALIAS;
            }

            return Self::parsed_terms(term_string, config, warnings, vec![term]);
        }

        // We may accept "encoding-rs" label only within a single-valued string
        #[cfg(feature = "encoding_rs")]
        if let Some(e) = Encoding::for_label(term_string) {
            if !config.allow_non_standard_encodings {
                return Self::parse_failed(
                    term_string,
                    config,
                    FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING,
                );
            }
            return Self {
                terms: Vec::new(),
                config,
                chosen_impl: ChosenImpl::External,
                external: Some(e),
            }
            .emit_parse_warnings(
                term_string,
                WARN_DPXKB_DS_0004_ACCEPTED_NON_STANDARD_ENCODING,
            );
        }

        Self::parse_failed(term_string, config, FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING)
    }

    /// Part of the specific character set parser: parses the first value of the
    /// multi-valued input text.
    ///
    /// This function can instruct top-level parser to:
    /// - abort with error
    /// - finish parsing with a specified term discarding other values
    /// - or to continue with other values.
    fn parse_multi_valued_first_value(term_string: &[u8], config: &Config) -> ParseMultiFirst {
        if term_string.is_empty() {
            // From standard:
            // > If the Attribute Specific Character Set (0008,0005) has more
            // > than one value and value 1 is empty, it is assumed that value 1
            // > is ISO 2022 IR 6.
            return ParseMultiFirst::Accept(Term::Iso2022Ir6, 0);
        }

        let Some((term, matched_with)) = Term::search_by_keyword(term_string)
            else {
                #[cfg(feature = "encoding_rs")]
                if Encoding::for_label(term_string).is_some() {
                    return ParseMultiFirst::Fail(FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
                }
                return ParseMultiFirst::Fail(FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);
            };

        if !term.is_standard_dicom() || term.kind() == TermKind::MultiByteWithoutCodeExtensions {
            return ParseMultiFirst::Fail(FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
        }

        let mut warnings: u16 = 0;
        if matched_with != TermMatchedWith::Primary {
            if !config.allow_encoding_aliases {
                return ParseMultiFirst::Fail(FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);
            }
            warnings = WARN_DPXKB_DS_0007_ACCEPTED_ALIAS;
        }

        if let (
            TermKind::SingleByteWithoutCodeExtensions,
            &CodecType::Iso2022NoExtensions(extended_variant),
        ) = (term.kind(), &term.meta().mode)
        {
            if !config.allow_iso2022_non_extensible_term_in_multi_valued_charset {
                return ParseMultiFirst::Fail(FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
            }
            if warnings & WARN_DPXKB_DS_0007_ACCEPTED_ALIAS == 0 {
                warnings |= WARN_DPXKB_DS_0012_SINGLE_BYTE_WITHOUT_EXTENSIONS_PROMOTED;
            }
            return ParseMultiFirst::Accept(extended_variant, warnings);
        }

        ParseMultiFirst::Accept(term, warnings)
    }

    /// Part of the specific character set parser: parses value other than the
    /// first in the multi-valued input text.
    ///
    /// This function can instruct top-level parser to:
    /// - abort with error
    /// - ignore current value
    /// - or to continue with other values.
    fn parse_multi_valued_next_value(
        term_string: &[u8],
        config: &Config,
        term_list: &[Term],
    ) -> ParseMultiOthers {
        if term_string.is_empty() {
            if !config.ignore_multi_value_duplicates {
                return ParseMultiOthers::Fail(FAIL_DPXKB_DS_0010_EMPTY_VALUE_IN_MULTI_VALUED);
            }
            return ParseMultiOthers::Ignore(WARN_DPXKB_DS_0008_IGNORED_EMPTY_VALUED);
        }

        let Some((mut term, matched_with)) = Term::search_by_keyword(term_string)
            else {
                #[cfg(feature = "encoding_rs")]
                if Encoding::for_label(term_string).is_some() {
                    return ParseMultiOthers::Fail(FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
                }
                return ParseMultiOthers::Fail(FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);
            };

        if !term.is_standard_dicom() || term.kind() == TermKind::MultiByteWithoutCodeExtensions {
            return ParseMultiOthers::Fail(FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
        }

        let mut warnings: u16 = 0;
        if matched_with != TermMatchedWith::Primary {
            if !config.allow_encoding_aliases {
                return ParseMultiOthers::Fail(FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);
            }
            warnings |= WARN_DPXKB_DS_0007_ACCEPTED_ALIAS;
        }

        if let (
            TermKind::SingleByteWithoutCodeExtensions,
            &CodecType::Iso2022NoExtensions(extended_variant),
        ) = (term.kind(), &term.meta().mode)
        {
            if !config.allow_iso2022_non_extensible_term_in_multi_valued_charset {
                return ParseMultiOthers::Fail(FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
            }
            if warnings & WARN_DPXKB_DS_0007_ACCEPTED_ALIAS == 0 {
                warnings |= WARN_DPXKB_DS_0012_SINGLE_BYTE_WITHOUT_EXTENSIONS_PROMOTED;
            }
            term = extended_variant;
        }

        if term_list.contains(&term) {
            if !config.ignore_multi_value_duplicates {
                return ParseMultiOthers::Fail(FAIL_DPXKB_DS_0011_DUPLICATE_VALUE_IN_MULTI_VALUED);
            }
            warnings |= WARN_DPXKB_DS_0009_IGNORED_DUPLICATE_VALUED;
            return ParseMultiOthers::Ignore(warnings);
        }

        ParseMultiOthers::Accept(term, warnings)
    }

    /// Constructs a Codec from a specified `term_list`
    ///
    /// Note: `ISO-2022 enabled` terms are terms with a
    /// [kind](crate::term::Term::kind) of
    /// [SingleByteWithCodeExtensions](TermKind::SingleByteWithCodeExtensions),
    /// [SingleByteWithoutCodeExtensions](TermKind::SingleByteWithoutCodeExtensions)
    /// or [MultiByteWithCodeExtensions](TermKind::MultiByteWithCodeExtensions).
    ///
    /// Supplied term list will be "sanitized" if it is multi-valued in a
    /// following way:
    /// - If the first term is not `ISO-2022 enabled`, then rest of the terms
    ///   are ignored.
    /// - If some term other than the first is not `ISO-2022 enabled`, then it
    ///   is ignored.
    /// - Any term of kind
    ///   [SingleByteWithoutCodeExtensions](TermKind::SingleByteWithoutCodeExtensions)
    ///   will be converted to a corresponding
    ///   [SingleByteWithCodeExtensions](TermKind::SingleByteWithCodeExtensions).
    /// - Any duplicate term will be ignored
    ///
    /// Empty `term_list` will be treated as [Term::Unknown].
    pub fn from_term_list(term_list: &[Term], config: Config) -> Self {
        fn is_iso2022_enabled(term: Term) -> bool {
            matches!(
                term.kind(),
                TermKind::SingleByteWithCodeExtensions
                    | TermKind::SingleByteWithoutCodeExtensions
                    | TermKind::MultiByteWithCodeExtensions
            )
        }

        // TODO: Make behavior consistent with "from_specific_character_set"

        if term_list.is_empty() {
            Self {
                terms: vec![Term::Unknown],
                config,
                ..Default::default()
            }
        } else if term_list.len() == 1 || !is_iso2022_enabled(term_list[0]) {
            Self {
                terms: vec![term_list[0]],
                config,
                ..Default::default()
            }
        } else {
            let mut terms = term_list
                .iter()
                .filter(|term| is_iso2022_enabled(**term))
                .copied()
                .collect::<Vec<Term>>();

            if terms.len() > 1 {
                // Convert "no-extensions" to "with-extensions".
                for term in terms.iter_mut() {
                    if let (
                        TermKind::SingleByteWithoutCodeExtensions,
                        &CodecType::Iso2022NoExtensions(ext_term),
                    ) = (term.kind(), &term.meta().mode)
                    {
                        *term = ext_term;
                    }
                }
                // Eliminate duplicates
                for index in (1..terms.len()).rev() {
                    if terms[0..index].contains(&terms[index]) {
                        terms.remove(index);
                    }
                }
            }

            Self {
                terms,
                config,
                ..Default::default()
            }
        }
    }

    /// Creates a struct from a specified encoding of [encoding_rs] crate.
    #[cfg(feature = "encoding_rs")]
    pub fn from_external(encoding: &'static encoding_rs::Encoding, config: Config) -> Self {
        Self {
            terms: Vec::new(),
            config,
            chosen_impl: ChosenImpl::External,
            external: Some(encoding),
        }
    }

    /// Returns `Some`([encoding_rs::Encoding]) if custom encoding from
    /// [encoding_rs] crate used or `None` if struct uses own
    /// [terms](Self::terms).
    #[cfg(feature = "encoding_rs")]
    pub fn external(&self) -> Option<&'static encoding_rs::Encoding> {
        self.external
    }

    /// Returns an effective list of [Term]'s this Codec uses to encode and
    /// decode [String]s.
    ///
    /// If struct was constructed with an encoding from
    /// [`encodings_rs`](https://crates.io/crates/encoding_rs), then the
    /// returned vector will be empty. Use
    /// [specific_character_set](fn@Self::specific_character_set) get an
    /// actual text-version of the encoding.
    pub fn terms(&self) -> &Vec<Term> {
        &self.terms
    }

    /// Returns an effective encoding DICOM term.
    ///
    /// This string may not be a valid standard term in case, when
    /// struct was constructed from non-standard specific character set.
    pub fn specific_character_set(&self) -> String {
        #[cfg(feature = "encoding_rs")]
        if let Some(encoding) = self.external {
            return encoding.name().to_string();
        }

        self.terms
            .iter()
            .map(|t| t.keywords()[0])
            .fold(String::new(), |mut r, t| {
                if !r.is_empty() {
                    r.push(CHAR_VALUES_SEPARATOR);
                }
                r.push_str(t);
                r
            })
    }

    /// Returns a configuration this class was created with
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Replaces a configuration in this struct
    pub fn set_config(&mut self, new_value: Config) {
        self.config = new_value;
        if !self.terms.is_empty() {
            self.chosen_impl = Self::choose_impl(&self.terms, &self.config);
        }
    }

    /// Internal function, that selects a best suitable codec within
    /// specified `terms` and `config`. Does not work with `encoding_rs`
    /// based codec.
    fn choose_impl(terms: &Vec<Term>, _: &Config) -> ChosenImpl {
        let term = *terms.first().expect("Bug! Term list should not be empty!");

        match term.meta().mode {
            CodecType::Iso2022NoExtensions(..) | CodecType::Iso2022WithExtensions(..) => {
                if terms.len() == 1 {
                    ChosenImpl::Iso2022Simple
                } else {
                    ChosenImpl::Iso2022Extended
                }
            }
            CodecType::NonIso2022(..) => ChosenImpl::NonIso2022,
            CodecType::Utf8 => ChosenImpl::Utf8,
        }
    }

    /// Decodes byte-string from `bytes` into UTF-8 string.
    ///
    /// # Params:
    /// - `bytes` - The input byte-string.
    /// - `context` - The context of the operation. It is entirely dependent on
    ///   the Value Representation of the element being decoded
    ///
    /// # Returns
    /// If possible, this function "borrows" input `bytes` in the output string.
    /// If not, allocates a new [String].
    pub fn decode<'a>(&self, bytes: &'a [u8], context: &Context) -> Cow<'a, str> {
        match self.chosen_impl {
            ChosenImpl::Utf8 => utf8_impl::decode(bytes, &self),
            ChosenImpl::Iso2022Simple => iso2022_simple_impl::decode(bytes, self),
            ChosenImpl::Iso2022Extended => iso2022_impl::decode(bytes, self, context),
            ChosenImpl::NonIso2022 => non_iso2022_impl::decode(bytes, self),
            #[cfg(feature = "encoding_rs")]
            ChosenImpl::External => external_impl::decode(bytes, self, context),
        }
    }

    /// Encodes UTF-8 to byte-string
    ///
    /// # Params:
    /// - `string` - The input UTF-8 string.
    /// - `context` - The context of the operation. It is entirely dependent on
    ///   the Value Representation of the element being decoded
    ///
    /// # Returns
    /// If possible, this function "borrows" input bytes from the input string
    /// in the output vector.
    /// If not, allocates a new buffer.
    pub fn encode<'a>(&self, string: &'a str, context: &Context) -> Cow<'a, [u8]> {
        match self.chosen_impl {
            ChosenImpl::Utf8 => utf8_impl::encode(string),
            ChosenImpl::Iso2022Simple => iso2022_simple_impl::encode(string, self),
            ChosenImpl::Iso2022Extended => iso2022_impl::encode(string, self, context),
            ChosenImpl::NonIso2022 => non_iso2022_impl::encode(string, self),
            #[cfg(feature = "encoding_rs")]
            ChosenImpl::External => external_impl::encode(string, self, context),
        }
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self {
            terms: vec![Term::IsoIr192],
            config: Config::new(),
            chosen_impl: ChosenImpl::Utf8,
            #[cfg(feature = "encoding_rs")]
            external: None,
        }
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use tracing_test::traced_test;

    #[traced_test]
    fn assert_warning(char_set: &str, config: Config, exp_terms: &[Term], exp_dpxkb: u16) -> Codec {
        let codec = Codec::from_specific_character_set(char_set.as_bytes(), config);
        assert_eq!(codec.terms().as_slice(), exp_terms);
        let exp_dpxkb_string = DPXKB_MAP.iter().find_map(|v| if v.0 == exp_dpxkb { Some(v.1) } else { None }).expect("Unknown dpxkb const");
        assert!(logs_contain(exp_dpxkb_string));
        codec
    }
    fn assert_error(char_set: &str, config: Config, exp_dpxkb: u16) {
        assert_warning(char_set, config, vec![Term::Unknown].as_slice(), exp_dpxkb);
    }

    #[traced_test]
    fn assert_ok(char_set: &str, config: Config, exp_terms: &[Term]) -> Codec {
        let codec = Codec::from_specific_character_set(char_set.as_bytes(), config);
        assert_eq!(codec.terms().as_slice(), exp_terms);
        assert!(
            !logs_contain(""),
            "Expected an empty log after function execution"
        );
        codec
    }

    #[test]
    fn is_empty_specific_character_set_parsed_correctly() {
        // Empty should be failed
        assert_error("", Config::new(), FAIL_DPXKB_DS_0001_EMPTY_CHAR_SET);

        // Spaces are currently silently discarded.
        assert_error("   ", Config::new_restrictive(), FAIL_DPXKB_DS_0001_EMPTY_CHAR_SET);
    }

    #[test]
    fn is_single_valued_specific_character_set_parsed_correctly() {
        // Standard DICOM terms should be parsed normally without any warnings
        assert_ok("ISO_IR 6", Config::new(), &[Term::IsoIr6]);
        assert_ok("ISO_IR 100", Config::new(), &[Term::IsoIr100]);
        assert_ok("ISO_IR 144", Config::new(), &[Term::IsoIr144]);
        assert_ok("ISO_IR 203", Config::new(), &[Term::IsoIr203]);
        assert_ok("ISO 2022 IR 6", Config::new(), &[Term::Iso2022Ir6]);
        assert_ok("ISO 2022 IR 100", Config::new(), &[Term::Iso2022Ir100]);
        assert_ok("ISO 2022 IR 144", Config::new(), &[Term::Iso2022Ir144]);
        assert_ok("ISO 2022 IR 203", Config::new(), &[Term::Iso2022Ir203]);
        assert_ok("ISO 2022 IR 87", Config::new(), &[Term::Iso2022Ir87]);
        assert_ok("ISO_IR 192", Config::new(), &[Term::IsoIr192]);
        assert_ok("GB18030", Config::new(), &[Term::Gb18030]);
        assert_ok("GBK", Config::new(), &[Term::Gbk]);

        // Spaces are SILENTLY ignored
        assert_ok("  ISO_IR 6  ", Config::new(), &[Term::IsoIr6]);

        // Non-standard encodings may be accepted only if configuration allows so
        assert_warning("cp1250", Config::new(), &[Term::NonDicomCp1250], WARN_DPXKB_DS_0004_ACCEPTED_NON_STANDARD_ENCODING);
        assert_error("cp1250", Config::new_restrictive(), FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);
        // Non-standard encodings are also may have aliases
        assert_warning("windows1250", Config::new(), &[Term::NonDicomCp1250], WARN_DPXKB_DS_0004_ACCEPTED_NON_STANDARD_ENCODING);

        // Aliased encoding names, including standard DICOM terms with different
        // casing should also depend on configuration.
        assert_warning("iso_ir 100", Config::new(), &[Term::IsoIr100], WARN_DPXKB_DS_0007_ACCEPTED_ALIAS);
        assert_warning("iso-ir100", Config::new(), &[Term::IsoIr100], WARN_DPXKB_DS_0007_ACCEPTED_ALIAS);
        assert_warning("iso 8859 1", Config::new(), &[Term::IsoIr100], WARN_DPXKB_DS_0007_ACCEPTED_ALIAS);
        assert_error("iso_ir 100", Config::new_restrictive(), FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);

        // Unknown encodings causes errors in both compatible and restrictive configurations
        assert_error("some unknown", Config::new(), FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);
        assert_error("some unknown", Config::new_restrictive(), FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);
    }

    #[cfg(feature = "encoding_rs")]
    #[test]
    fn is_single_valued_specific_character_set_accepts_encoding_rs() {
        // Like other non-standard encodings, this should be allowed by the
        // default configuration
        let c = assert_warning("x-unicode20utf8", Config::new(), &[], WARN_DPXKB_DS_0004_ACCEPTED_NON_STANDARD_ENCODING);
        assert_eq!(c.external, Some(encoding_rs::UTF_8));
        // Restrictive configuration should forbid this
        assert_error("x-unicode20utf8", Config::new_restrictive(), FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);
    }

    #[test]
    fn is_multi_valued_specific_character_set_first_value_parsed_correctly() {
        // --- FIRST VALUE: EMPTY
        // If first value is empty, it should be interpreted as IR 6 according
        // to The Standard
        assert_ok("\\ISO 2022 IR 100", Config::new(), &[Term::Iso2022Ir6, Term::Iso2022Ir100]);

        // --- FIRST VALUE: UNKNOWN
        // Unknown encodings always fails the parser
        assert_error("some unknown\\ISO 2022 IR 100", Config::new(), FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);
        assert_error("some unknown\\ISO 2022 IR 100", Config::new_restrictive(), FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);

        // Multi-valued encoding should not allow encodings-rs
        #[cfg(feature = "encoding_rs")]
        assert_error("x-unicode20utf8\\ISO 2022 IR 100", Config::new(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);

        // --- FIRST VALUE: NON-STANDARD
        // Non-standard encodings in the first value should fail the parser
        assert_error("cp1251\\ISO 2022 IR 100", Config::new(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);

        // --- FIRST VALUE: MULTI-BYTE WITHOUT CODE EXTENSIONS
        // Never allowed to use multi-byte without code extensions encodings
        // in the multi valued attribute.
        assert_error("ISO_IR 192\\ISO 2022 IR 100", Config::new(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);

        // --- FIRST VALUE: ALIAS

        // Aliases should be supported by a default configuration. Here are two
        // transformations performed:
        // 1. find aliased name
        // 2. replace non-extensible with extensible single-byte encoding
        assert_warning("ISO-8859-1\\ISO 2022 IR 144", Config::new(), &[Term::Iso2022Ir100, Term::Iso2022Ir144], WARN_DPXKB_DS_0007_ACCEPTED_ALIAS);
        // But forbidden with restrictive one
        assert_error("ISO-8859-1\\ISO 2022 IR 100", Config::new_restrictive(), FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);
        // Specifically, 'allow_encoding_aliases' option should deny it
        assert_error(
            "ISO-8859-1\\ISO 2022 IR 100",
            Config { allow_encoding_aliases: false, .. Default::default() },
            FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);

        // --- FIRST VALUE: SINGLE-BYTE WITHOUT CODE EXTENSIONS
        // Default configuration allows to "substitute" this terms with a
        // corresponding single-byte WITH extensions
        assert_warning("ISO_IR 6\\ISO 2022 IR 144", Config::new(), &[Term::Iso2022Ir6, Term::Iso2022Ir144], WARN_DPXKB_DS_0012_SINGLE_BYTE_WITHOUT_EXTENSIONS_PROMOTED);
        // Restrictive configuration wan't allow this
        assert_error("ISO_IR 6\\ISO 2022 IR 144", Config::new_restrictive(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
        // Specifically, there is a flag, that denies this
        assert_error(
            "ISO_IR 6\\ISO 2022 IR 144",
            Config { allow_iso2022_non_extensible_term_in_multi_valued_charset: false, .. Default::default() },
            FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);

        // Finally, test for a standard-compliant multi-valued character set
        assert_ok(
            "ISO 2022 IR 6\\ISO 2022 IR 100\\ISO 2022 IR 144", Config::new(),
            &[Term::Iso2022Ir6, Term::Iso2022Ir100, Term::Iso2022Ir144]);
        // This standard-compliant encoding should also be normally parsed with restrictive configuration
        assert_ok(
            "ISO 2022 IR 6\\ISO 2022 IR 100\\ISO 2022 IR 144", Config::new_restrictive(),
            &[Term::Iso2022Ir6, Term::Iso2022Ir100, Term::Iso2022Ir144]);
    }

    #[test]
    fn is_multi_valued_specific_character_set_other_values_parsed_correctly() {
        // --- OTHER VALUE: EMPTY
        // Empty values not allowed in the standard, but default configuration relaxes
        // that rule and allows the parser to ignore such values.
        assert_warning(
            "ISO 2022 IR 6\\\\ISO 2022 IR 100", Config::new(),
            &[Term::Iso2022Ir6, Term::Iso2022Ir100],
            WARN_DPXKB_DS_0008_IGNORED_EMPTY_VALUED
        );
        // This is true unless term string remains multi valued after ignoring
        assert_error("ISO 2022 IR 6\\", Config::new(), FAIL_DPXKB_DS_0010_EMPTY_VALUE_IN_MULTI_VALUED);
        // Also, restrictive configuration forbids this behavior
        assert_error(
            "ISO 2022 IR 6\\\\ISO 2022 IR 100", Config::new_restrictive(),
            FAIL_DPXKB_DS_0010_EMPTY_VALUE_IN_MULTI_VALUED
        );
        // Option responsible for that is 'ignore_multi_value_duplicates'
        assert_error(
            "ISO 2022 IR 6\\",
            Config { ignore_multi_value_duplicates: false, .. Default::default() },
            FAIL_DPXKB_DS_0010_EMPTY_VALUE_IN_MULTI_VALUED);

        // --- OTHER VALUE: UNKNOWN
        // Unknown encodings always fails the parser
        assert_error("ISO 2022 IR 6\\some unknown", Config::new(), FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);
        assert_error("ISO 2022 IR 6\\some unknown\\ISO 2022 IR 100", Config::new(), FAIL_DPXKB_DS_0002_UNKNOWN_ENCODING);
        // Multi-valued encoding should not allow encodings-rs
        #[cfg(feature = "encoding_rs")]
        assert_error("ISO 2022 IR 6\\x-unicode20utf8", Config::new(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);

        // --- OTHER VALUE: NON-STANDARD
        // Non-standard encodings should fail the parser
        assert_error("ISO 2022 IR 6\\cp1251", Config::new(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);

        // --- OTHER VALUE: MULTI-BYTE WITHOUT CODE EXTENSIONS

        // Never allowed to use multi-byte without code extensions encodings
        // in the multi valued attribute.
        assert_error("ISO 2022 IR 100\\ISO_IR 192", Config::new(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);

        // --- OTHER VALUE: ALIAS

        // Aliases should be supported by a default configuration. Here are two
        // transformations performed:
        // 1. find aliased name
        // 2. replace non-extensible with extensible single-byte encoding
        assert_warning("ISO 2022 IR 144\\ISO-8859-1", Config::new(), &[Term::Iso2022Ir144, Term::Iso2022Ir100], WARN_DPXKB_DS_0007_ACCEPTED_ALIAS);
        // But forbidden with restrictive one
        assert_error("ISO 2022 IR 144\\ISO-8859-1", Config::new_restrictive(), FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);
        // Specifically, 'allow_encoding_aliases' option should deny it
        assert_error(
            "ISO 2022 IR 144\\ISO-8859-1",
            Config { allow_encoding_aliases: false, .. Default::default() },
            FAIL_DPXKB_DS_0003_NON_STANDARD_ENCODING);

        // --- FIRST VALUE: SINGLE-BYTE WITHOUT CODE EXTENSIONS
        // Default configuration allows to "substitute" this terms with a
        // corresponding single-byte WITH extensions
        assert_warning("ISO 2022 IR 6\\ISO_IR 144", Config::new(), &[Term::Iso2022Ir6, Term::Iso2022Ir144], WARN_DPXKB_DS_0012_SINGLE_BYTE_WITHOUT_EXTENSIONS_PROMOTED);
        // Restrictive configuration will not allow this
        assert_error("ISO 2022 IR 6\\ISO_IR 144", Config::new_restrictive(), FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);
        // Specifically, there is a flag, that denies this
        assert_error(
            "ISO 2022 IR 6\\ISO_IR 144",
            Config { allow_iso2022_non_extensible_term_in_multi_valued_charset: false, .. Default::default() },
            FAIL_DPXKB_DS_0005_MULTI_VALUED_NON_ISO_2022);


        // --- OTHER VALUE: DUPLICATE
        // With the default configuration duplicates are ignored unless charset remains multi valued
        assert_warning("ISO 2022 IR 100\\ISO 2022 IR 100\\ISO 2022 IR 144", Config::new(), &[Term::Iso2022Ir100, Term::Iso2022Ir144], WARN_DPXKB_DS_0009_IGNORED_DUPLICATE_VALUED);
        // This should also works, if duplicate is not consecutive
        assert_warning("ISO 2022 IR 100\\ISO 2022 IR 144\\ISO 2022 IR 100", Config::new(), &[Term::Iso2022Ir100, Term::Iso2022Ir144], WARN_DPXKB_DS_0009_IGNORED_DUPLICATE_VALUED);
        // And also work if duplicate is an alias
        assert_warning("ISO 2022 IR 100\\ISO 2022 IR 144\\ISO-8859-1", Config::new(), &[Term::Iso2022Ir100, Term::Iso2022Ir144], WARN_DPXKB_DS_0009_IGNORED_DUPLICATE_VALUED);
        // This should not work if charset becomes single valued
        assert_error("ISO 2022 IR 100\\ISO 2022 IR 100", Config::new(), FAIL_DPXKB_DS_0011_DUPLICATE_VALUE_IN_MULTI_VALUED);
        // In a restrictive configuration this should fail
        assert_error("ISO 2022 IR 100\\ISO 2022 IR 100\\ISO 2022 IR 144", Config::new_restrictive(), FAIL_DPXKB_DS_0011_DUPLICATE_VALUE_IN_MULTI_VALUED);
        assert_error(
            "ISO 2022 IR 100\\ISO 2022 IR 100\\ISO 2022 IR 144",
            Config { ignore_multi_value_duplicates: false, .. Default::default() },
            FAIL_DPXKB_DS_0011_DUPLICATE_VALUE_IN_MULTI_VALUED);
    }

    #[test]
    fn can_create_from_term_list() {
        assert_eq!(
            Codec::from_term_list(&[], Config::default()).terms,
            vec![Term::Unknown]
        );
        assert_eq!(
            Codec::from_term_list(&[Term::IsoIr100], Config::default()).terms,
            vec![Term::IsoIr100]
        );
        assert_eq!(
            Codec::from_term_list(&[Term::NonDicomIbm866], Config::default()).terms,
            vec![Term::NonDicomIbm866]
        );
        // When first is non iso2022 enabled, others should be ignored
        assert_eq!(
            Codec::from_term_list(&[Term::Gbk, Term::Iso2022Ir100], Config::default()).terms,
            vec![Term::Gbk]
        );
        // When some term other than first is not iso2022 enabled, it should be ignored
        assert_eq!(
            Codec::from_term_list(
                &[Term::Iso2022Ir6, Term::IsoIr192, Term::Iso2022Ir100],
                Config::default()
            )
            .terms,
            vec![Term::Iso2022Ir6, Term::Iso2022Ir100]
        );
        // SingleByteWithoutExtensions should be converted to SingleByteWithExtensions when multi-valued
        assert_eq!(
            Codec::from_term_list(&[Term::IsoIr6, Term::IsoIr100], Config::default()).terms,
            vec![Term::Iso2022Ir6, Term::Iso2022Ir100]
        );
        // Duplicates should be eliminated
        assert_eq!(
            Codec::from_term_list(
                &[
                    Term::IsoIr6,
                    Term::Iso2022Ir6,
                    Term::Iso2022Ir100,
                    Term::IsoIr100
                ],
                Config::default()
            )
            .terms,
            vec![Term::Iso2022Ir6, Term::Iso2022Ir100]
        );
    }
}
