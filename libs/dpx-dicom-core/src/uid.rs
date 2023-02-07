//! Unique Object Identifier [Uid] and associated structures

use crate::*;
use snafu::{ensure, Snafu};
use std::{fmt::Debug, fmt::Display};

mod uid_meta;

pub use uid_meta::META_LIST_DICOM;

/// Default root used to generate [unique](Uid::generate_unique) Uids
pub const DEFAULT_UID_ROOT: &str = "1.2.3";

/// Result type for fallible function of this [module](crate::uid).
type Result<T> = core::result::Result<T, Error>;

/// Enumeration with errors from this [module](crate::uid).
#[derive(Debug, Snafu)]
#[allow(missing_docs)]
pub enum Error {
    #[snafu(display("Uid is empty"))]
    Empty,

    #[snafu(display("Uid is too long ({len} chars), allowed 64"))]
    Overflow { len: usize },

    #[snafu(display("invalid character in Uid {c} at {pos}"))]
    InvalidChar { pos: usize, c: char },

    #[snafu(display("empty component in Uid at {pos}"))]
    EmptyComponent { pos: usize },

    #[snafu(display("Uid component starts with zero at {pos}"))]
    FirstCharIsZero { pos: usize },
}

/// Structure holding an UID (unique identifier)
///
/// A string of characters used to provide global unique identification of a
/// wide variety of items, guaranteeing uniqueness across multiple countries,
/// sites, vendors and equipment. It uses the structure defined by [ISO/IEC
/// 8824] for OSI Object Identifiers.
///
/// It is composed of components separated with "." (DOT) character. Each
/// component must contain only numeric characters. If component contains more
/// than one digit, it should not start with 0. Maximum allowed length: 64
/// chars.
///
/// This structure stores it's text in a [Cow](std::borrow::Cow) to minimize heap allocations.
///
/// You can create this structure from `&str` or `String` using `from` method.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Uid<'a>(Cow<'a, str>);

/// Structure describing properties of a known [Uid]'s
#[derive(Debug, Clone)]
pub struct Meta {
    pub uid: Uid<'static>,
    pub is_retired: bool,
    pub name: Cow<'static, str>,
    pub keyword: Cow<'static, str>,
    pub uid_type: UidType,
}

/// Enumeration of [Uid] categories. Used primarily in [Meta]
#[derive(Debug, Clone)]
pub enum UidType {
    /// Unknown/other category
    Other,
    ApplicationContextName,
    ApplicationHostingModel,
    CodingScheme,
    LdapOid,
    MappingResource,
    MetaSopClass,

    /// Instances of this SOP class has a `Patient` model (patient-study-series-instance).
    ///
    /// These objects may be sent with `C-STORE` to a regular [Storage Service](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_B
    ///     "PS 3.4 \"B. Storage Service Class (Normative)\"").
    SopClassPatientStorage {
        /// Synthesized modality, that uniquely identifies this Storage SOP
        /// class among others.
        ///
        /// This has no particular meaning in the Standard, but used by some
        /// tools to display SOP Class information in a "short" way.
        modality: Cow<'static, str>,

        /// Guessed size of the file with this SOP Class
        guessed_size: usize,

        /// Specialization of the storage SOP class
        kind: StorageKind,
    },

    /// Instances of this SOP Class has a `Non-Patient` model.
    ///
    /// These objects may be sent with `C-STORE` to a speciaL [Non-Patient Object Storage Service](
    ///     https://dicom.nema.org/medical/dicom/current/output/html/part04.html#chapter_GG
    ///     "PS 3.4 \"GG. Non-Patient Object Storage Service Class\"").
    SopClassNonPatientStorage,

    /// This [Uid] represents a Service Class. A special UID found only in
    /// `A-ASSOCIATE-RQ` to negotiate extended information.
    ServiceClass,

    /// Generic SOP class
    SopClass,

    SynchronizationFrameOfReference,

    /// This [Uid] represents a Transfer Syntax
    TransferSyntax {
        /// Is this Transfer Syntax uses little-endian (`true`) or big-endian
        /// (`false`) byte ordering for numerics.
        ///
        /// For example:
        /// - [ImplicitVRLittleEndian](crate::uids::ts::ImplicitVRLittleEndian) has this property set to `true`
        /// - [ExplicitVRBigEndian](crate::uids::ts::ExplicitVRBigEndian) has this property set to `false`
        is_little_endian: bool,

        /// Is this Transfer Syntax explicitly defines Value Representation
        /// (`true`) or relies on a dictionary [Vr] lookup (`false`)
        ///
        /// For example:
        /// - [ImplicitVRLittleEndian](crate::uids::ts::ImplicitVRLittleEndian) has this property set to `true`
        /// - [ExplicitVRLittleEndian](crate::uids::ts::ExplicitVRLittleEndian) has this property set to `false`
        is_explicit_vr: bool,

        /// Is the file requires decompression to read it's attributes (`true`)
        ///
        /// For example:
        /// - [DeflatedExplicitVRLittleEndian](crate::uids::ts::DeflatedExplicitVRLittleEndian) has this property set to `true`
        /// - [ImplicitVRLittleEndian](crate::uids::ts::ImplicitVRLittleEndian) has this property set to `false`
        is_compressed: bool,

        /// Is the pixel data encapsulated with some codec (`true`)
        ///
        /// For example:
        /// - [JPEGBaseline8Bit](crate::uids::ts::JPEGBaseline8Bit) has this property set to `true`
        /// - [ImplicitVRLittleEndian](crate::uids::ts::ImplicitVRLittleEndian) has this property set to `false`
        is_encapsulated: bool,
    },

    WellKnownSopInstance,
}

/// Defines a generic kind of data in storage SOP Class
#[derive(Debug, Clone)]
pub enum StorageKind {
    /// Instances of this SOP Class contains an image
    Image,
    /// Instances of this SOP Class contains a multiframe image
    MultiframeImage,
    /// Instances of this SOP Class contains a video or multiframe image
    Cine,
    /// Instances of this SOP Class contains some waveform, for example, ECG.
    Waveform,
    /// Instances of this SOP Class contains some audio data
    Audio,
    /// Instances of this SOP Class contains structured report
    StructuredReport,
    /// Instances of this SOP Class contains a presentation state
    PresentationState,
    /// Instances of this SOP Class encapsulates some object (PDF, STL, OBJ, ...)
    Document,
    /// Some data not belonging to the other categories.
    Other,
}

/// Structure holding a reference to a static list of Uid descriptions (list of [`Meta`]'s)
#[derive(Clone, Copy)]
pub struct StaticMetaList(pub(crate) &'static [Meta]);
inventory::collect!(StaticMetaList);

/// Structure managing a collection of [Uid]s and their [Meta]data.
///
/// This structure is rarely accessed by the application. All the functions
/// provided by this struct are mirrored in higher-level abstractions.
///
/// The structure itself contains a vector of statically allocated tag
/// description ([Meta]) lists and dynamically registered individual uid
/// descriptions.
///
/// Search in the directory is carried out by calling methods
/// [search_by_uid](Self::search_by_uid) or
/// [search_by_keyword](Self::search_by_keyword). To speed up the search process,
/// Dictionary supports a "cache". It is invalidated every time content mutated
/// and may be rebuild with [rebuild_cache](Self::rebuild_cache).
///
/// Dictionary may be extended with static lists using
/// [add_static_list](Self::add_static_list) or with dynamic lists with
/// [add_dynamic_list](Self::add_dynamic_list).
///
/// See [Dictionary::new] for details on built-in static lists.
///
/// Dictionary lookup by higher-level abstractions forwarded through
/// [State](crate::State) struct.
///
/// This class supports "automatic" registration of static descriptions list
/// using crate [`inventory`]. Use [StaticMetaList] struct with [inventory::submit!].
///
/// Example:
/// ```
/// // Declare your attributes and register them in the Dictionary
/// mod app {
///     dpx_dicom_core::declare_uids!{
///         /// A list of my custom app uids
///         pub const MY_UIDS = [
///             /// Some custom SOP Class for my application.
///             CustomUID1: {"1.2.3.4.5", false, "Custom UID1", SopClass},
///             /// Unique object type my application is able to store
///             CustomUID2: {"1.2.3.4.6", false, "Custom UID2", SopClassPatientStorage {
///                 modality: Cow::Borrowed("ZZ"), guessed_size: 256 * 256 * 2, kind: StorageKind::Image }
///             },
///             /// Describe me!
///             CustomUID3: {"1.2.3.4.7", false, "Custom UID3", WellKnownSopInstance},
///         ];
///     }
///     inventory::submit!(MY_UIDS);
/// }
///
/// # use dpx_dicom_core::uid::{Dictionary, Uid};
/// // Example of direct Dictionary invocation without global State:
/// # #[cfg(not(miri))]
/// # fn main() {
/// let dict = Dictionary::new();
/// assert_eq!(
///     dict.search_by_uid(&app::CustomUID1).unwrap().name,
///     "Custom UID1"
/// );
///
/// // Example of recommended Dictionary interaction:
/// // Note: here a global Dictionary will be automatically
/// // constructed on first use.
/// assert_eq!(
///     Uid::from(app::CustomUID1).name().unwrap(),
///     "Custom UID1"
/// );
/// # }
/// # #[cfg(miri)]
/// # fn main() {}
/// ```
///
/// Note: automatic registration is currently unsupported under
/// [Miri](https://github.com/rust-lang/miri).
#[derive(Clone)]
pub struct Dictionary {
    statics: Vec<&'static StaticMetaList>,
    dynamic: Vec<Meta>,
    cache: Option<DictionaryCache>,
}

/// Internal structure for uids cache
#[derive(Debug, Clone)]
struct DictionaryCache {
    by_uid: HashMap<Cow<'static, str>, Meta>,
    by_keyword: HashMap<Cow<'static, str>, Meta>,
}

// ---------------------------------------------------------------------------
// Uid struct implementation
// ---------------------------------------------------------------------------

impl<'a> Uid<'a> {
    /// Create a new Uid struct
    pub const fn new(v: Cow<'a, str>) -> Self {
        Self(v)
    }

    /// Tries to find specified keyword in a [State] and returns
    /// Uid found.
    ///
    /// See also [search_by_keyword](crate::uid::Dictionary::search_by_keyword)
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_core::{Uid, uids};
    /// assert_eq!(
    ///     Uid::from_keyword("ExplicitVRLittleEndian").unwrap().as_str(),
    ///     uids::ts::ExplicitVRLittleEndian
    /// );
    /// ```
    pub fn from_keyword(name: impl AsRef<str>) -> Option<Uid<'static>> {
        crate::State::with_current(|s| {
            s.uid_dictionary()
                .search_by_keyword(name)
                .map(|m| m.uid.clone())
        })
    }

    /// Validates this Uid according to a [DICOM specification]
    ///
    /// - Should not be empty
    /// - Length must not exceed 64 characters
    /// - Consists of components delimited with "." (DOT)
    /// - Each component of a UID is a number and shall consist of one or more
    ///   digits.
    /// - The first digit of each component shall not be zero unless the
    ///   component is a single digit.
    ///
    /// Note. VR `UI` allows NULL padding, but these are encoding details that
    /// application code should not have to deal with.
    ///
    /// [DICOM specification]:
    ///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_9.html#sect_9.1
    ///     "PS 4.5 \"9 Unique Identifiers (UIDs)\""
    #[rustfmt::skip]
    pub fn validate(&self) -> Result<()> {
        let value = self.0.as_ref();

        let to_pos = |s: &str, idx: usize| -> usize {
            let byte_offset = s.as_ptr() as usize - value.as_ptr() as usize + idx;
            value.char_indices().enumerate()
                .find_map(|(co, (i,_))| {
                    if i >= byte_offset { Some(co) } else { None }
                }).unwrap_or(value.len())
        };

        ensure!(!value.is_empty(), EmptySnafu{});
        ensure!(value.len() <= 64, OverflowSnafu{len: self.0.len()});
        for component in value.split('.') {
            ensure!(!component.is_empty(), EmptyComponentSnafu{pos: to_pos(component, 0)});
            ensure!(component.len() == 1 || !component.starts_with('0'), FirstCharIsZeroSnafu{pos: to_pos(component, 0)});
            for (idx, c) in component.chars().enumerate() {
                ensure!(c.is_numeric(), InvalidCharSnafu{pos: to_pos(component, idx), c});
            }
        }
        Ok(())
    }

    /// Returns an internal Uid storage value
    pub const fn value(&self) -> &Cow<'_, str> {
        &self.0
    }

    /// Returns this Uid as a [String]
    pub fn into_owned(self) -> String {
        self.0.into_owned()
    }

    /// Transforms borrowed string into owned. If string
    /// was already owned, this function does not allocate.
    pub fn to_owned(self) -> Uid<'static> {
        Uid::<'static>(Cow::Owned(self.0.into_owned()))
    }

    /// Generates a unique identifier with an optional `prefix`.
    ///
    /// If the `prefix` is not specified, function assumes [DEFAULT_UID_ROOT].
    /// Not this function does not check `prefix` for validity.
    ///
    /// Unique identifier consists of:
    /// - A `prefix`
    /// - crc32 of platform-dependent current machine GUID (see
    ///   [machine_uid::get()])
    /// - current process id
    /// - unique [usize] counter, that atomically grows on each function call.
    ///   At startup, this counter get initialized with some arbitrary value
    ///   from 0 to 999999.
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_core::Uid;
    /// # #[cfg(not(miri))]
    /// println!("{}", Uid::generate_unique(Some("1.2.3.4")));
    /// ```
    pub fn generate_unique(prefix: Option<&str>) -> Uid<'static> {
        use std::sync::atomic;
        use std::time;
        static COUNTER: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
        static MACHINE_UID_CRC: atomic::AtomicU32 = atomic::AtomicU32::new(0);
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let new_machine_uid = machine_uid::get().unwrap_or_else(|_| "N/A".to_owned());
            let new_machine_crc = crc32fast::hash(new_machine_uid.as_bytes());
            // This will extract only microsecond part of the current time
            let new_counter = (time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
                % (999999u128)) as usize;

            MACHINE_UID_CRC.store(new_machine_crc, atomic::Ordering::Relaxed);
            COUNTER.store(new_counter, atomic::Ordering::Relaxed);
        });

        let counter = COUNTER.fetch_add(1, atomic::Ordering::Relaxed);
        let machine_crc = MACHINE_UID_CRC.load(atomic::Ordering::Relaxed);

        let mut rv = format!(
            "{}.{}.{}.{}",
            prefix.unwrap_or(DEFAULT_UID_ROOT),
            machine_crc,
            std::process::id(),
            counter
        );
        rv.truncate(64);
        rv.into()
    }

    /// Searches Uid information in the current [State](crate::State)
    ///
    /// See also [search_by_uid](crate::uid::Dictionary::search_by_uid)
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_core::{Uid, uid::UidType, uids};
    /// assert!(matches!(
    ///     Uid::from(uids::ts::ExplicitVRLittleEndian).meta().unwrap().uid_type,
    ///     UidType::TransferSyntax{..}
    /// ));
    /// ```
    pub fn meta(&self) -> Option<Meta> {
        crate::State::with_current(|s| s.uid_dictionary().search_by_uid(self).cloned())
    }

    /// Searches and returns Uid keyword in the current [State](crate::State)
    ///
    /// See also [search_by_uid](crate::uid::Dictionary::search_by_uid)
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_core::{Uid, uids};
    /// assert_eq!(
    ///     Uid::from(uids::ts::ExplicitVRLittleEndian).keyword().unwrap(),
    ///     String::from("ExplicitVRLittleEndian")
    /// );
    /// ```
    pub fn keyword(&self) -> Option<String> {
        crate::State::with_current(|s| {
            s.uid_dictionary()
                .search_by_uid(self)
                .map(|m| m.keyword.to_string())
        })
    }

    /// Searches and returns Uid name in the current [State](crate::State)
    ///
    /// See also [search_by_uid](crate::uid::Dictionary::search_by_uid)
    ///
    /// Example:
    /// ```
    /// use dpx_dicom_core::{Uid, uids};
    /// assert_eq!(
    ///     Uid::from(uids::ts::ExplicitVRLittleEndian).name().unwrap(),
    ///     String::from("Explicit VR Little Endian")
    /// );
    /// ```
    pub fn name(&self) -> Option<String> {
        crate::State::with_current(|s| {
            s.uid_dictionary()
                .search_by_uid(self)
                .map(|m| m.name.to_string())
        })
    }

    /// Get the raw string Uid
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> From<&'a str> for Uid<'a> {
    fn from(value: &'a str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl<'a> From<&'_ &'a str> for Uid<'a> {
    fn from(value: &'_ &'a str) -> Self {
        Self(Cow::Borrowed(*value))
    }
}

impl From<String> for Uid<'static> {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl From<&'_ String> for Uid<'static> {
    fn from(value: &'_ String) -> Self {
        Self(Cow::Owned(value.clone()))
    }
}

impl<'a> From<Uid<'a>> for String {
    fn from(value: Uid<'a>) -> Self {
        value.0.into_owned()
    }
}

impl<'a, 'b> From<&'b Uid<'a>> for &'b str
where
    'a: 'b,
{
    fn from(value: &'b Uid<'a>) -> Self {
        value.as_ref()
    }
}

impl<'a> AsRef<str> for Uid<'a> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<'a> Display for Uid<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_ref())
    }
}

impl<'a> Debug for Uid<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Uid({})", self.0)
    }
}

// ---------------------------------------------------------------------------
// Dictionary struct implementation
// ---------------------------------------------------------------------------
impl Dictionary {
    /// Constructs the struct gathering all the [`StaticMetaList`] objects
    /// registered with [`inventory::submit!`].
    ///
    /// This function always registers built-in [Meta] list [META_LIST_DICOM].
    ///
    /// Note: See struct-level [documentation](Self) for examples.
    ///
    /// Note: The created dictionary has no "cache". To speed up searches,
    /// you may call [rebuild_cache](Self::rebuild_cache) after creation or
    /// any mutating function call.
    pub fn new() -> Self {
        Self {
            statics: inventory::iter::<StaticMetaList>.into_iter().collect(),
            dynamic: Vec::new(),
            cache: None,
        }
    }

    /// Constructs the empty struct without any statically registered lists.
    pub fn new_empty() -> Self {
        Self {
            statics: Vec::new(),
            dynamic: Vec::new(),
            cache: None,
        }
    }

    /// Adds custom constant list of ['Meta'] objects.
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    pub fn add_static_list(&mut self, dict: &'static StaticMetaList) {
        // Note: We do not check equality of dictionary content, only check if
        // they are at the same memory address.
        if !self
            .statics
            .iter()
            .any(|e| ::core::ptr::eq((*e) as *const _, dict as *const _))
        {
            self.cache = None;
            self.statics.push(dict);
        }
    }

    /// Adds custom list of ['Meta'] objects.
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    pub fn add_dynamic_list<T: Iterator<Item = Meta>>(&mut self, iter: T) {
        // Invalidate cache early to satisfy SAFETY invariants including safety
        // on "panic" unwind
        self.cache = None;
        self.dynamic.reserve(iter.size_hint().1.unwrap_or(0));
        for v in iter {
            self.dynamic.push(v);
        }
    }

    /// Clears any tags that were previously added
    /// with [add_dynamic_list](Self::add_dynamic_list)
    pub fn clear_dynamic_list(&mut self) {
        self.cache = None;
        self.dynamic.clear();
    }

    /// Rebuilds a cache
    pub fn rebuild_cache(&mut self) {
        let guessed_total_count =
            self.statics.iter().fold(0, |acc, dict| acc + dict.0.len()) + self.dynamic.len();

        let mut cache = DictionaryCache {
            by_uid: HashMap::with_capacity(guessed_total_count),
            by_keyword: HashMap::with_capacity(guessed_total_count),
        };

        for dict in self.statics.iter() {
            for m in dict.0.iter() {
                cache.by_uid.insert(m.uid.0.clone(), m.clone());
                cache.by_keyword.insert(m.keyword.clone(), m.clone());
            }
        }

        for m in self.dynamic.iter() {
            cache.by_uid.insert(m.uid.0.clone(), m.clone());
            cache.by_keyword.insert(m.keyword.clone(), m.clone());
        }

        self.cache = Some(cache);
    }

    /// Searches for the [Meta] information with a specified Uid string.
    pub fn search_by_uid(&self, uid: impl AsRef<str>) -> Option<&Meta> {
        if let Some(cache) = &self.cache {
            return cache.by_uid.get(uid.as_ref());
        }

        if let Some(m) = self.dynamic.iter().rev().find(|m| m.uid.0 == uid.as_ref()) {
            return Some(m);
        }

        for s in self.statics.iter().rev() {
            if let Some(m) = s.0.iter().find(|m| m.uid.0 == uid.as_ref()) {
                return Some(m);
            }
        }
        None
    }

    /// Searches for the [Meta] information with a specified keyword string.
    pub fn search_by_keyword(&self, keyword: impl AsRef<str>) -> Option<&Meta> {
        if let Some(cache) = &self.cache {
            return cache.by_keyword.get(keyword.as_ref());
        }

        if let Some(m) = self
            .dynamic
            .iter()
            .rev()
            .find(|m| m.keyword == keyword.as_ref())
        {
            return Some(m);
        }

        for s in self.statics.iter().rev() {
            if let Some(m) = s.0.iter().find(|m| m.keyword == keyword.as_ref()) {
                return Some(m);
            }
        }
        None
    }
}

impl Debug for Dictionary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "uid::Dictionary({} list, {} statics, {} dynamics, {} cached)",
            self.statics.len(),
            self.statics.iter().fold(0, |v, c| v + c.0.len()),
            self.dynamic.len(),
            match &self.cache {
                Some(c) => c.by_uid.len(),
                None => 0,
            },
        )
    }
}

impl Default for Dictionary {
    fn default() -> Self {
        Dictionary::new()
    }
}

// ---------------------------------------------------------------------------
// StaticMetaList struct implementation
// ---------------------------------------------------------------------------
impl StaticMetaList {
    /// Creates a new instance with a specified constant list
    pub const fn new(ml: &'static [Meta]) -> Self {
        Self(ml)
    }
    /// Returns a contained constant list of [`Meta`] objects
    pub const fn value(&self) -> &'static [Meta] {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_uids {
        use crate::declare_uids;
        use inventory::submit;
        declare_uids! {
            pub const TEST_UID_LIST = [
                CustomUID1: {"1.2.3.4.5", false, "Custom UID1", SopClass},
                CustomUID2: {"1.2.3.4.6", false, "Custom UID2", SopClassPatientStorage{ modality: Cow::Borrowed("ZZ"), guessed_size: 256 * 256 * 2, kind: StorageKind::Image }},
                CustomUID3: {"1.2.3.4.7", false, "Custom UID3", WellKnownSopInstance},
            ];
        }
        submit!(TEST_UID_LIST);
    }

    fn search_uids_in_dict(dict: &Dictionary) {
        fn assert_search(d: &Dictionary, searched: &Uid<'static>, expected: &Uid<'static>) {
            let found = &d
                .search_by_uid(searched)
                .unwrap_or_else(|| panic!("Uid \"{searched}\" was not found"))
                .uid;
            assert_eq!(found, expected);
        }

        for m in test_uids::TEST_UID_LIST.value() {
            assert_search(dict, &m.uid, &m.uid);
        }
    }

    #[test]
    fn is_dict_searchable() {
        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&test_uids::TEST_UID_LIST);

        // Search in non-cached dictionary
        search_uids_in_dict(&dict);
    }

    #[test]
    fn is_dict_cache_searchable() {
        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&test_uids::TEST_UID_LIST);

        dict.rebuild_cache();
        search_uids_in_dict(&dict);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn is_dict_auto_collects_statics() {
        let dict = Dictionary::new();

        search_uids_in_dict(&dict);

        assert_eq!(
            dict.search_by_uid(uids::svc_storage::BasicTextSRStorage)
                .unwrap()
                .keyword
                .as_ref(),
            "BasicTextSRStorage"
        );
        assert_eq!(
            dict.search_by_uid(&Uid::from(uids::svc_storage::BasicTextSRStorage))
                .unwrap()
                .keyword
                .as_ref(),
            "BasicTextSRStorage"
        );
    }

    macro_rules! assert_err {
        ($e:expr, $exp_err:path, $exp_pos:literal) => {
            match $e {
                Ok(_) => panic!("{} expected to fail", stringify!($e)),
                Err($exp_err { pos: pos, .. }) => {
                    assert_eq!(
                        pos,
                        $exp_pos,
                        "{} expected to fail on pos {}, but got {}",
                        stringify!($e),
                        $exp_pos,
                        pos
                    )
                }
                Err(x) => panic!(
                    "{} expected to fail with {}, but failed with {:?}",
                    stringify!($e),
                    stringify!($exp_err),
                    x
                ),
            }
        };
    }

    #[test]
    fn is_validated_correctly() {
        assert!(Uid::from("0").validate().is_ok());
        assert!(Uid::from("1").validate().is_ok());
        assert!(Uid::from("12334567890").validate().is_ok());
        assert!(Uid::from("0.1").validate().is_ok());
        assert!(Uid::from("0.123").validate().is_ok());
        assert!(Uid::from("123.123.456.7.8.9").validate().is_ok());
        assert!(Uid::from(std::str::from_utf8(&[b'1'; 64]).unwrap())
            .validate()
            .is_ok());
        assert!(matches!(
            Uid::from("").validate().unwrap_err(),
            Error::Empty
        ));
        assert!(matches!(
            Uid::from(std::str::from_utf8(&[b'1'; 65]).unwrap())
                .validate()
                .unwrap_err(),
            Error::Overflow { .. }
        ));
        assert_err!(Uid::from("01").validate(), Error::FirstCharIsZero, 0);
        assert_err!(Uid::from("1.01").validate(), Error::FirstCharIsZero, 2);
        assert_err!(Uid::from("1.1z2").validate(), Error::InvalidChar, 3);
        assert_err!(Uid::from(".1").validate(), Error::EmptyComponent, 0);
        assert_err!(Uid::from("1.").validate(), Error::EmptyComponent, 2);
    }

    #[cfg(not(miri))]
    #[test]
    fn is_generated_unique() {
        let uniq1 = Uid::generate_unique(None);
        let uniq2 = Uid::generate_unique(Some(DEFAULT_UID_ROOT));
        let uniq3 = Uid::generate_unique(Some("666"));
        assert_ne!(uniq1, uniq2);
        assert_ne!(uniq1, uniq3);
        assert!(uniq1.validate().is_ok());
        assert!(uniq2.validate().is_ok());
        assert!(uniq3.validate().is_ok());
        assert!(uniq1.value().starts_with(DEFAULT_UID_ROOT));
        assert!(uniq2.value().starts_with(DEFAULT_UID_ROOT));
        assert!(uniq3.value().starts_with("666."));
    }

    #[test]
    fn is_dict_globally_acessible() {
        assert!(matches!(
            Uid::from(crate::uids::ts::ExplicitVRLittleEndian)
                .meta()
                .unwrap()
                .uid_type,
            UidType::TransferSyntax { .. }
        ));
    }
}
