use crate::*;
use snafu::{ensure, Snafu};
use std::{fmt::Debug, fmt::Display};

pub const DEFAULT_UID_ROOT: &str = "1.2.3";

/// Structure holding an OID (unique identifier)
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
/// This structure stores it's text in a [Cow] to minimize heap allocations.
///
/// You can create this structure from `&str` or `String` using `from` method.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uid<'a>(Cow<'a, str>);

/// Structure describing properties of a known [Uid]'s
#[derive(Debug, Clone)]
pub struct Meta {
    pub uid: Uid<'static>,
    pub category: Category,
    pub keyword: Cow<'static, str>,
}

/// Enumeration of [Uid] categories. Used primarily in [Meta]
#[derive(Debug, Clone, Default)]
pub enum Category {
    /// Unknown/other category
    #[default]
    Other,
    /// This [Uid] represents a Transfer Syntax
    TransferSyntax,
    /// This [Uid] represents a Service Class.
    ServiceClass,
    /// This [Uid] is a Storage Class. Any SOP instance having this class could
    /// be stored on a disk or transferred with C-STORE.
    StorageSopClass {
        /// Is the object of this class has an image.
        is_imaging: bool,
        /// Expected value of `Modality (0008,0060)` attribute.
        modality: Option<Cow<'static, str>>,
        /// Guessed size of the file with this SOP Class
        guessed_size: Option<usize>,
    },
    WellKnownSopInstance,
}

/// Structure holding a reference to a static list of Uid descriptions (list of [`Meta`]'s)
#[derive(Clone, Copy)]
pub struct StaticMetaList(pub(crate) &'static [Meta]);
inventory::collect!(StaticMetaList);

#[derive(Default, Clone)]
pub struct Dictionary {
    statics: Vec<&'static StaticMetaList>,
    dynamic: Vec<Meta>,
    cache: Vec<Meta>,
}

type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Snafu)]
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

// ---------------------------------------------------------------------------
// Uid struct implementation
// ---------------------------------------------------------------------------

impl<'a> Uid<'a> {
    pub const fn new(v: Cow<'a, str>) -> Self {
        Self(v)
    }

    #[rustfmt::skip]
    pub fn validate(&self) -> Result<()> {
        let value = self.0.as_ref();

        let to_pos = |s: &str, idx: usize| -> usize {
            let byte_offset = s.as_ptr() as usize - value.as_ptr() as usize + idx;
            value.char_indices().enumerate()
                .find_map(|(co, (i,_))| {
                    if i >= byte_offset { Some(co) } else { None }
                }).unwrap_or_else(|| value.len())
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

    pub const fn value<'b>(&'b self) -> &'b Cow<'b, str> {
        &self.0
    }

    pub fn to_owned(self) -> Uid<'static> {
        Uid::<'static>(Cow::Owned(self.0.into_owned()))
    }

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
            prefix
                .map(|p| p.as_ref())
                .unwrap_or_else(|| DEFAULT_UID_ROOT),
            machine_crc,
            std::process::id(),
            counter
        );
        rv.truncate(64);
        rv.into()
    }

    /// Searches Uid information in the current [State](crate::State)
    ///
    /// See also [search](crate::uid::Dictionary::search)
    pub fn meta(&self) -> Option<Meta> {
        crate::State::with_current(|s| s.uid_dictionary().search(self).cloned())
    }

    /// Searches and returns Uid name in the current [State](crate::State)
    ///
    /// See also [search](crate::uid::Dictionary::search)
    pub fn name(&self) -> Option<Cow<'static, str>> {
        crate::State::with_current(|s| s.uid_dictionary().search(self).map(|m| m.keyword.clone()))
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
where 'a: 'b
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
    pub fn new() -> Self {
        Self {
            statics: inventory::iter::<StaticMetaList>.into_iter().collect(),
            ..Default::default()
        }
    }

    pub fn new_empty() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn add_static_list(&mut self, dict: &'static StaticMetaList) {
        // Note: We do not check equality of dictionary content, only check if
        // they are at the same memory address.
        if !self
            .statics
            .iter()
            .any(|e| ::core::ptr::eq((*e) as *const _, dict as *const _))
        {
            self.cache.clear();
            self.statics.push(dict);
        }
    }

    pub fn add_dynamic_list<T: Iterator<Item = Meta>>(&mut self, iter: T) {
        // Invalidate cache early to satisfy SAFETY invariants including safety
        // on "panic" unwind
        self.cache.clear();
        self.dynamic.reserve(iter.size_hint().1.unwrap_or(0));
        for v in iter {
            self.dynamic.push(v);
        }
    }

    pub fn clear_dynamic_list(&mut self) {
        self.cache.clear();
        self.dynamic.clear();
    }

    /// Rebuilds a cache
    pub fn rebuild_cache(&mut self) {
        let mut cache = std::mem::replace(&mut self.cache, Vec::new());

        let guessed_total_count =
            self.statics.iter().fold(0, |acc, dict| acc + dict.0.len()) + self.dynamic.len();
        cache.reserve(guessed_total_count);

        // Reverse order, because after stable sort and dedup, we want
        // to prioritize dynamically added attributes over statically
        // added attributes. Each list of attributes also prioritizes
        // lastly added.
        for m in self.dynamic.iter().rev() {
            cache.push(m.clone());
        }

        for dict in self.statics.iter().rev() {
            for m in dict.0.iter() {
                cache.push(m.clone())
            }
        }

        cache.sort_by(|l, r| l.uid.cmp(&r.uid));
        cache.dedup_by(|l, r| l.uid == r.uid);

        self.cache = cache;
    }

    pub fn search(&self, uid: &Uid) -> Option<&Meta> {
        if !self.cache.is_empty() {
            return match self.cache.binary_search_by(|m| m.uid.cmp(uid)) {
                Ok(idx) => Some(&self.cache[idx]),
                Err(_) => None,
            };
        }

        match self.dynamic.iter().find(|m| m.uid == *uid) {
            Some(m) => return Some(m),
            None => (),
        }

        for s in self.statics.iter() {
            match s.0.iter().find(|m| m.uid == *uid) {
                Some(m) => return Some(m),
                None => (),
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
            self.cache.len(),
        )
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

    mod uids {
        use crate::declare_uids;
        use inventory::submit;
        declare_uids! {
            pub const TEST_UID_LIST = [
                XferLittleEndianImplicit: {"1.2.840.10008.1.2", TransferSyntax},
                SopClassBasicTextSR: {"1.2.840.10008.5.1.4.1.1.88.11", StorageSopClass{
                    is_imaging: false,
                    modality: Some(Cow::Borrowed("SR")),
                    guessed_size: Some(1024),
                }},
                SopClassEnchancedSR: {"1.2.840.10008.5.1.4.1.1.88.11", StorageSopClass{
                    is_imaging: false,
                    modality: Some(Cow::Borrowed("SR")),
                    guessed_size: Some(1024),
                }},
            ];
        }
        submit!(TEST_UID_LIST);
    }

    fn search_uids_in_dict(dict: &Dictionary) {
        fn assert_search(d: &Dictionary, searched: &Uid<'static>, expected: &Uid<'static>) {
            let found = &d
                .search(searched)
                .unwrap_or_else(|| panic!("Uid \"{searched}\" was not found"))
                .uid;
            assert_eq!(found, expected);
        }

        for m in uids::TEST_UID_LIST.value() {
            assert_search(dict, &m.uid, &m.uid);
        }
    }

    #[test]
    fn is_dict_searchable() {
        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&uids::TEST_UID_LIST);

        // Search in non-cached dictionary
        search_uids_in_dict(&dict);
    }

    #[test]
    fn is_dict_cache_searchable() {
        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&uids::TEST_UID_LIST);

        dict.rebuild_cache();
        search_uids_in_dict(&dict);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn is_dict_auto_collects_statics() {
        let dict = Dictionary::new();
        search_uids_in_dict(&dict);
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
    fn fff() {}
}
