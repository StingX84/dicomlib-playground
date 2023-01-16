use crate::*;
use snafu::{ensure, Snafu};
use std::{fmt::Debug, fmt::Display};

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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uid<'a>(Cow<'a, str>);

/// Structure
#[derive(Debug, Clone)]
pub struct Meta {
    pub uid: Uid<'static>,
    pub category: Category,
    pub keyword: Cow<'static, str>,
}

#[derive(Debug, Clone, Default)]
pub enum Category {
    #[default]
    Other,
    TransferSyntax,
    ServiceClass,
    StorageSopClass {
        is_imaging: bool,
        modality: Option<Cow<'static, str>>,
        guessed_size: Option<usize>,
        subclass_of: Option<Uid<'static>>,
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
            s.as_ptr() as usize - value.as_ptr() as usize + idx
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
        crate::State::with_current(|s| {
            s.uid_dictionary()
                .search(self)
                .map(|m| m.keyword.clone())
        })
    }
}

impl<'a> From<&'a str> for Uid<'a> {
    fn from(value: &'a str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl From<String> for Uid<'static> {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
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
        use inventory::submit;
        use crate::declare_uids;
        declare_uids! {
            pub const TEST_UID_LIST = [
                XferLittleEndianImplicit: {"1.2.840.10008.1.2", TransferSyntax},
                SopClassBasicTextSR: {"1.2.840.10008.5.1.4.1.1.88.11", StorageSopClass{
                    is_imaging: false,
                    modality: Some(Cow::Borrowed("COW")),
                    guessed_size: Some(1024),
                    subclass_of: None,
                }},
                SopClassEnchancedSR: {"1.2.840.10008.5.1.4.1.1.88.11", StorageSopClass{
                    is_imaging: false,
                    modality: Some(Cow::Borrowed("COW")),
                    guessed_size: Some(1024),
                    subclass_of: Some(SopClassBasicTextSR),
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
}
