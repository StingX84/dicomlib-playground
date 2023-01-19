use super::*;
use crate::Cow;
use std::{
    cmp::Ordering,
    fmt::Debug,
    io::{self, BufRead},
    ptr::NonNull,
};

/// Structure managing a collection of attributes and their metadata.
///
/// This structure is rarely accessed by the application. All the functions
/// provided by this struct are mirrored in higher-level abstractions.
///
/// The structure itself contains a vector of statically allocated tag
/// description ([`Meta`]) lists and dynamically registered individual tag
/// descriptions.
///
/// Search in the directory is carried out by calling methods
/// [search_by_tag](Self::search_by_tag) or
/// [search_by_key](Self::search_by_key). To speed up the search process,
/// Dictionary supports a "cache". It is invalidated every time content mutated
/// and may be rebuild with [rebuild_cache](Self::rebuild_cache).
///
/// Dictionary may be extended with static lists using
/// [add_static_list](Self::add_static_list) or with dynamic lists with
/// [add_dynamic_list](Self::add_dynamic_list),
/// [add_from_memory](Self::add_from_memory) or
/// [add_from_file](Self::add_from_file).
///
/// Dictionary lookup by higher-level abstractions forwarded through
/// [`State`](crate::State)
///
/// This class supports "automatic" registration of static descriptions list
/// using crate [`inventory`]. Built-in standard DICOM attributes are already
/// registered automatically.
///
/// Example of application custom dictionary loaded automatically:
/// ```
/// // Declare your attributes and register them in the Dictionary
/// mod app {
///     use dpx_dicom_core::declare_tags;
///     use inventory::submit;
///     declare_tags!{
///         pub const ALL_APP_TAGS = [
///             PatientSpacecraftLicense: { (0x4321, 0x1000, "CoolApp Group1"), LO, 1-n, "Patient's Spacecraft License", Vendored(X) },
///             IssuerOfPatientSpacecraftLicense: { (0x4321, 0x1001, "CoolApp Group1"), LO, 1-n, "Issuer of Patient's Spacecraft License", Vendored(None) },
///             DoctorCryingReason: { (0x4323, 0x10BB, "CoolApp Group2"), UT, 1, "Doctor Crying Reason", Vendored(None) },
///         ];
///     }
///     submit!(ALL_APP_TAGS);
/// }
///
/// // Use your attributes anywhere in the application
/// use dpx_dicom_core::tag::Dictionary;
/// # #[cfg(not(miri))]
/// # fn main() {
/// let dict = Dictionary::new();
/// assert_eq!(
///     dict.search_by_tag(&app::DoctorCryingReason).unwrap().name,
///     "Doctor Crying Reason"
/// );
/// # }
/// # #[cfg(miri)]
/// # fn main() {}
/// ```
///
/// Note: automatic registration is currently unsupported under
/// [Miri](https://github.com/rust-lang/miri).
#[derive(Default)]
pub struct Dictionary {
    /// Vector of statically defined attribute lists added with
    /// [add_static_list](Self::add_static_list) or gathered in [new](Self::new)
    /// with [`inventory`]
    statics: Vec<&'static StaticMetaList>,

    /// Vector of dynamically added attributes with
    /// [add_dynamic_list](Self::add_dynamic_list),
    /// [add_from_memory](Self::add_from_memory) or
    /// [add_from_file](Self::add_from_file)
    dynamic: Vec<Meta>,

    /// Contains TagKey (element 0), private creator (element 1) and TagInfo
    /// (element 2) "flattened" from static and dynamic dictionaries sorted by
    /// Tag.
    ///
    /// Private Attributes Tag's are "flattened" in following forms:
    /// 1. original form as given
    /// 2. normalized form IF private creator is `Some`
    cache: Option<DictCache>,
}

// SAFETY:
// The reason this struct is not `Send` nor `Sync` by auto traits
// is presence of "raw" pointers. Struct guarantees, that
// no external shared state involved in those pointers.
unsafe impl Send for Dictionary {}
unsafe impl Sync for Dictionary {}

/// Shorthand for private creator stored in [`Tag`]
type Creator<'a> = Option<Cow<'a, str>>;

/// Entry in the dictionary cache [DictCache] for non masked tags
type DictCacheEntryNormal = (TagKey, NonNull<Creator<'static>>, NonNull<Meta>);

/// Entry in the dictionary cache [DictCache] for masked tags
type DictCacheEntryMasked = (TagKey, NonNull<Creator<'static>>, u32, NonNull<Meta>);

/// Shorthand for vector or "flattened" static and dynamic dictionaries
struct DictCache {
    vec: Vec<DictCacheEntryNormal>,
    masked: Vec<DictCacheEntryMasked>,
}

/// metrics from [Dictionary::metrics()]
pub struct DictMetrics {
    pub static_lists: usize,
    pub static_tags: usize,
    pub dynamic_tags: usize,
    pub cached_tags: Option<usize>,
}

// ---------------------------------------------------------------------------
// Dictionary public interface methods implementation
// ---------------------------------------------------------------------------
impl Dictionary {
    /// Constructs the class gathering all the [`StaticMetaList`] objects
    /// registered with [`inventory::submit!`].
    ///
    /// Note: See struct-level [documentation](Self) for examples.
    ///
    /// Note: The created dictionary has no "cache". To speed up searches,
    /// you should call [rebuild_cache](Self::rebuild_cache) after any
    /// mutating functions.
    pub fn new() -> Self {
        Self {
            statics: inventory::iter::<StaticMetaList>.into_iter().collect(),
            ..Default::default()
        }
    }

    /// Constructs the empty class without any statically registered lists.
    pub fn new_empty() -> Self {
        Self {
            ..Default::default()
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
            // Invalidate cache early to satisfy SAFETY invariants including
            // safety on "panic" unwind
            self.cache = None;
            self.statics.push(dict);
        }
    }

    /// Adds custom constant list of ['Meta'] objects.
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

    /// Parses a dictionary from a memory and adds it's content to the
    /// dictionary
    ///
    /// See expected format in [add_from_file](Self::add_from_file) method
    /// documentation.
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    pub fn add_from_memory(&mut self, buf: impl io::Read) -> Result<()> {
        let mut reader = io::BufReader::new(buf);
        let mut dict = Vec::<Meta>::new();

        let mut line_number = 1usize;
        let mut line = String::new();
        line.reserve(1024);
        while let Ok(size) = reader.read_line(&mut line) {
            if size == 0 {
                break;
            }
            match Meta::from_tsv_line(line.trim()) {
                Err(Error::MetaParseFailed { char_pos, msg }) => DictParseFailedSnafu {
                    line_number,
                    char_pos,
                    msg,
                }
                .fail()?,
                Err(e) => DictParseFailedSnafu {
                    line_number,
                    char_pos: 0usize,
                    msg: e.to_string(),
                }
                .fail()?,
                Ok(Some(meta)) => dict.push(meta),
                Ok(None) => (),
            }

            line.clear();
            line_number += 1;
        }
        // Invalidate cache early to satisfy SAFETY invariants including safety on "panic" unwind
        self.cache = None;
        self.add_dynamic_list(dict.into_iter());

        Ok(())
    }

    /// Reads a dictionary file and adds it content to the dictionary
    ///
    /// Note: as any other mutating method, this invalidates a cache. To speed
    /// up searches after mutation, you should call
    /// [rebuild_cache](Self::rebuild_cache).
    ///
    /// File format documentation:\
    /// Each line represents an entry in the data dictionary. Each line has 6
    /// fields `Tag`, `Name`, `Keyword`, `VR`, `VM` and `Version`.
    ///
    /// Entries need not be in ascending tag order. Entries may override
    /// existing entries. Each field must be separated by a single tab.
    ///
    /// `Tag` field must in form `(gggg,eeee[,"creator"])` where `gggg`, `eeee`
    /// must be in hexadecimal form with exception of `X` character, which
    /// denotes "any digit". `creator` string is optional and specifies Private
    /// Attribute creator. If present, it must be enclosed in double quotes and
    /// separated by comma from an adjacent element number.
    ///
    /// `Name` field should contain only graphical ASCII characters and white
    /// space ```[\x20-\x7E]```. Maximum length is 128 bytes.
    ///
    /// `Keyword` field should contain only a subset of ASCII characters
    /// ```[A-Za-z0-9_]``` preferably in CamelCase. Keyword should start with a
    /// letter. Maximum length is 64 bytes.
    ///
    /// `VR` field can contain up to three Value Representation names separated
    /// with " or " Undefined VR should be written as "--".
    ///
    /// `VM` field should contain one of the forms: `B`, `B-E`, `B-n`, `B-Bn`,
    /// where `B` - minimum number of repetitions 0 to 255, `E` - maximum number
    /// of repetitions 1 to 255, `n` - literal "n" symbol, which denotes
    /// "unbounded". Special form `B-Bn` means "arbitrary number multiple of B".
    ///
    /// `Version` field should contain one of the following terms (case
    /// insensitive):
    /// - `DICOM` - standard DICOM attribute
    /// - `DICONDE` - standard DICONDE attribute
    /// - `DICOS` - standard DICOS attribute
    /// - `Ret` - retired attribute from an unspecified source.
    /// - `Priv` - This is a private attribute known not to contain any patient
    ///   identifying information.
    /// - `Priv(X)` - This is a private attribute that contains patient
    ///   identifying information. 'X' specifies a method of "de-identification"
    ///   for this attribute and should be one of the following:
    /// - `D` - replace with a non-zero length value that may be a dummy value
    ///   and consistent with the VR
    /// - `Z` - replace with a zero length value, or a non-zero length value
    ///   that may be a dummy value and consistent with the VR
    /// - `X` - remove
    /// - `U` - replace with a non-zero length UID that is internally consistent
    ///   within a set of Instance
    ///
    /// Comments have a '#' at the beginning of the line. The file should be
    /// encoded as UTF-8 without BOM.
    ///
    /// Example line(tabs were replaced by spaces for documentation):
    /// ```text
    /// (0010,0020) Patient ID  PatientID   LO  1   dicom
    /// ```
    pub fn add_from_file(&mut self, file_name: impl AsRef<Path>) -> Result<()> {
        use std::fs::File;
        let file = File::open(file_name.as_ref()).context(DictFileOpenFailedSnafu {
            file_name: file_name.as_ref().to_path_buf(),
        })?;
        self.add_from_memory(file)
    }

    /// Rebuilds a cache
    pub fn rebuild_cache(&mut self) {
        let mut cache = DictCache {
            vec: Vec::new(),
            masked: Vec::new(),
        };

        let guessed_total_count =
            self.statics.iter().fold(0, |acc, dict| acc + dict.0.len()) + self.dynamic.len();
        cache.vec.reserve(guessed_total_count);

        let masked_total_count = self
            .statics
            .iter()
            .map(|c| c.0.iter())
            .flatten()
            .filter(|e| e.mask != 0xFFFFFFFF)
            .count()
            + self.dynamic.iter().filter(|e| e.mask != 0xFFFFFFFF).count();
        cache.masked.reserve(masked_total_count);

        // Reverse order, because after stable sort and dedup, we want
        // to prioritize dynamically added attributes over statically
        // added attributes. Each list of attributes also prioritizes
        // lastly added.
        for tag_info in self.dynamic.iter().rev() {
            Self::add_cached_tag(&mut cache, tag_info);
        }

        for dict in self.statics.iter().rev() {
            for tag_info in dict.0.iter().rev() {
                Self::add_cached_tag(&mut cache, tag_info)
            }
        }

        cache.vec.sort_by(Self::cmp_cache);
        cache
            .vec
            .dedup_by(|l, r| Self::cmp_cache(l, r) == Ordering::Equal);

        self.cache = Some(cache);
    }

    /// Searches a dictionary for the given [TagKey]
    ///
    /// This method does not honor Private Creators so it's usage
    /// should be carefully judged. It searches for the first
    /// [Meta] entry which `Meta.tag.key` matches the searched [TagKey]
    /// combined with a [Meta::mask]
    ///
    /// If cache is invalidated, linear search is performed against dynamic
    /// and all the static lists. If cache is available, binary search is
    /// performed.
    pub fn search_by_key(&self, key: TagKey) -> Option<&Meta> {
        // Search in the cache if it is available
        if let Some(cache) = &self.cache {
            if let Some(v) = Self::cache_search_sorted(cache, key) {
                return Some(v);
            }
            return Self::cache_search_masked(cache, key);
        }
        // Search the hard-way.
        let tag = Tag::new(key, None);

        let mut best_match = match Self::search_in_ary(self.dynamic.iter().rev(), &tag) {
            None => None,
            Some((true, meta)) => return Some(meta),
            Some((false, meta)) => Some(meta),
        };

        for ary in self.statics.iter().rev() {
            match Self::search_in_ary(ary.0.iter().rev(), &tag) {
                Some((true, meta)) => {
                    return Some(meta);
                }
                Some((false, meta)) => {
                    best_match.get_or_insert(meta);
                }
                None => (),
            }
        }

        best_match
    }

    /// Searches a dictionary for the given [Tag]
    ///
    /// It searches for the first [Meta] entry which `Meta.tag.key` matches the
    /// searched [Tag] combined with a [Meta::mask] exactly.
    ///
    /// In case of private attributes with [Some] creator, method also tried to
    /// find [canonical](TagKey::to_canonical_if_private) representation of the
    /// private attribute.
    ///
    /// If cache is invalidated, linear search is performed against dynamic and
    /// all the static lists. If cache is available, binary search is performed.
    pub fn search_by_tag<'a>(&self, tag: &'a Tag<'a>) -> Option<&Meta> {
        if !tag.key.is_private_attribute() {
            // This search will be slightly faster, because no "creator" comparisons involved
            return self.search_by_key(tag.key);
        }

        // Search in the cache if it is available
        if let Some(cache) = &self.cache {
            if cache.vec.is_empty() {
                return None;
            }
            // Step 1: exact original
            if let Some(meta) = Self::cache_search_sorted_with_creator(cache, tag.key, &tag.creator)
            {
                return Some(meta);
            }
            if tag.creator.is_some() {
                // Step 2 a: if creator: exact canonical
                if let Some(canonical_key) = tag.key.to_canonical_if_private() {
                    if let Some(meta) =
                        Self::cache_search_sorted_with_creator(cache, canonical_key, &tag.creator)
                    {
                        return Some(meta);
                    }
                }
                // Step 3: find an exact attribute with None creator
                if let Some(meta) = Self::cache_search_sorted_with_creator(cache, tag.key, &None) {
                    return Some(meta);
                }
            } else if let Some(meta) = Self::cache_search_sorted(cache, tag.key) {
                // Step 2 b: if no creator: exact ignoring creator
                return Some(meta);
            }
            // Step 3: search in masked
            if let Some(meta) = Self::cache_search_masked_with_creator(cache, tag.key, &tag.creator)
            {
                return Some(meta);
            }
            if tag.creator.is_some() {
                // Step 4 a: if creator: masked canonical
                if let Some(canonical_key) = tag.key.to_canonical_if_private() {
                    if let Some(meta) =
                        Self::cache_search_masked_with_creator(cache, canonical_key, &tag.creator)
                    {
                        return Some(meta);
                    }
                }
                // Step 5: find a masked attribute with None creator
                if let Some(meta) = Self::cache_search_masked_with_creator(cache, tag.key, &None) {
                    return Some(meta);
                }
            } else if let Some(meta) = Self::cache_search_masked(cache, tag.key) {
                // Step 4 b: if no creator: masked ignoring creator
                return Some(meta);
            }

            return None;
        }

        // Search the hard-way.
        let mut best_match = match Self::search_in_ary(self.dynamic.iter().rev(), tag) {
            None => None,
            Some((true, meta)) => return Some(meta),
            Some((false, meta)) => Some(meta),
        };

        for ary in self.statics.iter().rev() {
            match Self::search_in_ary(ary.0.iter().rev(), tag) {
                Some((true, meta)) => {
                    return Some(meta);
                }
                Some((false, meta)) => {
                    best_match.get_or_insert(meta);
                }
                None => (),
            }
        }

        best_match
    }

    /// Returns some simple metrics
    pub fn metrics(&self) -> DictMetrics {
        DictMetrics {
            static_lists: self.statics.len(),
            static_tags: self.statics.iter().fold(0, |v, c| v + c.0.len()),
            dynamic_tags: self.dynamic.len(),
            cached_tags: self.cache.as_ref().map(|c| c.vec.len() + c.masked.len()),
        }
    }

    /// Returns an iterator over all the static and dynamic [Meta] structs.
    ///
    /// Note: no tags deduplication or sorting involved!
    pub fn iter(&self) -> impl Iterator<Item = &Meta> {
        self.statics
            .iter()
            .map(|m| m.0)
            .flatten()
            .chain(self.dynamic.iter())
    }

    /// Returns an iterator over cached array of [Meta] structs.
    ///
    /// If cache was invalidated and not rebuilt, `None` returned.
    ///
    /// The returned structures are sorted by key and deduplicated. Structs for
    /// private attributes, that were given in non
    /// [canonical](TagKey::to_canonical_if_private) form also present in
    /// theirs canonical form.
    pub fn iter_cache(&self) -> Option<impl Iterator<Item = &Meta>> {
        match &self.cache {
            // SAFETY: Pointers in cache are always valid until data mutates,
            // which is not possible while iterator still holds shared reference
            // to the self.
            Some(c) => Some(c.vec.iter().map(|v| unsafe { v.2.as_ref() })),
            None => None,
        }
    }
}

impl Clone for Dictionary {
    fn clone(&self) -> Self {
        Self {
            statics: self.statics.clone(),
            dynamic: self.dynamic.clone(),
            cache: None,
        }
    }
}

impl Debug for Dictionary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = self.metrics();
        write!(
            f,
            "tag::Dictionary(tags: {} lists, {} static, {} dynamic, {:?} cached)",
            m.static_lists, m.static_tags, m.dynamic_tags, m.cached_tags
        )
    }
}

// ---------------------------------------------------------------------------
// Dictionary private methods implementation
// ---------------------------------------------------------------------------

// Private implementations
impl Dictionary {
    /// Adds a specified tag to the end of provided vector
    fn add_cached_tag(cache: &mut DictCache, tag_info: &Meta) {
        let key = tag_info.tag.key;

        if tag_info.mask != 0xFFFFFFFFu32 {
            cache.masked.push((
                key,
                NonNull::from(&tag_info.tag.creator),
                tag_info.mask,
                NonNull::from(tag_info),
            ));

            if tag_info.tag.creator.is_some() {
                // Add canonical form
                if let Some(normalized_key) = key.to_canonical_if_private() {
                    cache.masked.push((
                        normalized_key,
                        NonNull::from(&tag_info.tag.creator),
                        tag_info.mask,
                        NonNull::from(tag_info),
                    ));
                }
            }
        } else {
            // Add tag as-is
            cache.vec.push((
                TagKey(tag_info.tag.key.as_u32()),
                NonNull::from(&tag_info.tag.creator),
                NonNull::from(tag_info),
            ));

            // Note: if you did not provide 'private creator' for the private
            // attribute, then it will be matched "exactly". This is a
            // "compatibility" feature with some buggy software, that does not
            // provide any "private creator" for their attributes. If private
            // attribute in non-canonical form and has a creator specified, then it
            // will also lands in cache in canonical form.
            if tag_info.tag.creator.is_some() {
                // Add canonical form
                if let Some(normalized_key) = key.to_canonical_if_private() {
                    cache.vec.push((
                        normalized_key,
                        NonNull::from(&tag_info.tag.creator),
                        NonNull::from(tag_info),
                    ));
                }
            }
        }
    }

    /// Performs a binary search of [Tag] in the list of [Meta]'s.
    ///
    /// Private creator is matched exactly as passed.
    ///
    /// Supports masked values by positioning at "lower_bound" of the searched text
    /// and rewinding back.
    fn cache_search_sorted_with_creator<'a, 'b>(
        c: &'a DictCache,
        key: TagKey,
        creator: &'b Option<Cow<'b, str>>,
    ) -> Option<&'a Meta> {
        match c
            .vec
            .binary_search_by(|v| Self::cmp_cache_key_creator(v, key, creator))
        {
            Ok(index) => {
                // Exact match found

                // SAFETY "get_unchecked": we've got this VALID index from the
                // vector method and there is no way to mutate vector content
                // after the search. SAFETY "deref *const": all pointers are
                // invalidated when data they point to mutates, so there is no
                // chance for pointer to dangle.
                return Some(unsafe { c.vec.get_unchecked(index).2.as_ref() });
            }
            Err(_lower_bound) => {
                // Non exact match found. Index - lower bound
                ()
                // for index in (0..lower_bound).rev() {
                //     // SAFETY "get_unchecked": lower_bound is less or equal to
                //     // vector.len(), so index in range to "0 .. lower_bound"
                //     // will never got beyond array length. If array is empty,
                //     // this range will not yield any indices. SAFETY "deref
                //     // *const": all pointers are invalidated when data they
                //     // point to mutates, so there is no chance for pointer to
                //     // dangle.
                //     let info = unsafe { c.vec.get_unchecked(index).2.as_ref() };
                //     // We must account possible mask in the meta description.
                //     let tag_key_masked = TagKey(key.as_u32() & info.mask);
                //     // Early bail out if moved to another key
                //     if info.tag.key != tag_key_masked {
                //         break;
                //     }
                //     // Match private creator exactly
                //     if info.tag.creator != *creator {
                //         continue;
                //     }
                //     return Some(info);
                // }
            }
        };
        None
    }

    /// Performs a binary search of [Tag] in the list of [Meta]'s.
    ///
    /// This method ignores private creator on initial binary search, but when
    /// positioned to "lower_bound" of a searched string peeks one element ahead
    /// and one element behind for the match.
    fn cache_search_sorted(c: &DictCache, key: TagKey) -> Option<&Meta> {
        match c
            .vec
            .binary_search_by(|v| Self::cmp_cache_key_creator(v, key, &None))
        {
            Ok(index) => {
                // Exact match found

                // SAFETY "get_unchecked": we've got this VALID index from the
                // vector method and there is no way to mutate vector content
                // after the search. SAFETY "deref *const": all pointers are
                // invalidated when data they point to mutates, so there is no
                // chance for pointer to dangle.
                return Some(unsafe { c.vec.get_unchecked(index).2.as_ref() });
            }
            Err(lower_bound) => {
                // Non exact match found. Index - lower bound. Next entries MAY
                // contain same key, but with private creator set
                if lower_bound < c.vec.len() {
                    // SAFETY "get_unchecked": we've got this VALID index from
                    // the vector method and there is no way to mutate vector
                    // content after the search. SAFETY "deref *const": all
                    // pointers are invalidated when data they point to mutates,
                    // so there is no chance for pointer to dangle.
                    let info = unsafe { c.vec.get_unchecked(lower_bound).2.as_ref() };

                    if info.tag.key == key {
                        // Found same key ignoring private creator
                        return Some(info);
                    }
                }
                // // Lower - ranked entry may contain our key if it is masked
                // if lower_bound > 0 {
                //     // SAFETY "get_unchecked": we've got this VALID index from
                //     // the vector method and there is no way to mutate vector
                //     // content after the search. SAFETY "deref *const": all
                //     // pointers are invalidated when data they point to mutates,
                //     // so there is no chance for pointer to dangle.
                //     let info = unsafe { c.vec.get_unchecked(lower_bound - 1).2.as_ref() };
                //     // We must account possible mask in the meta description.
                //     let tag_key_masked = TagKey(key.as_u32() & info.mask);
                //     // Early bail out if moved to another key
                //     if info.tag.key == tag_key_masked {
                //         // Found some key with mask ignoring creator
                //         return Some(info);
                //     }
                // }
            }
        };
        None
    }

    fn cache_search_masked_with_creator<'a, 'b>(
        c: &'a DictCache,
        key: TagKey,
        creator: &'b Option<Cow<'b, str>>,
    ) -> Option<&'a Meta> {
        let mut matched: Option<&'a Meta> = None;
        for e in c.masked.iter() {
            if e.0.as_u32() == key.as_u32() & e.2 {
                let e_creator = unsafe { e.1.as_ref() };
                let e_meta = unsafe { e.3.as_ref() };
                if e_creator.is_none() {
                    matched = Some(e_meta);
                } else if e_creator == creator {
                    return Some(e_meta);
                }
            }
        }
        matched
    }

    fn cache_search_masked<'a, 'b>(c: &'a DictCache, key: TagKey) -> Option<&'a Meta> {
        for e in c.masked.iter() {
            if e.0.as_u32() == key.as_u32() & e.2 {
                let e_meta = unsafe { e.3.as_ref() };
                return Some(e_meta);
            }
        }
        None
    }

    /// Comparator function for sorting `DictCache::sorted` array.
    /// It compares elements 0 and 1 of a given tuples.
    fn cmp_cache(l: &DictCacheEntryNormal, r: &DictCacheEntryNormal) -> Ordering {
        match l.0.as_u32().cmp(&r.0.as_u32()) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            // SAFETY: Cache is always cleared when any data it points to
            // mutated, so there is no chance to get a dangling pointer.
            Ordering::Equal => unsafe { (l.1).as_ref().cmp(r.1.as_ref()) },
        }
    }

    /// Comparator function for searching `DictCache::sorted` array
    /// by [TagKey] ignoring any private creators
    fn cmp_cache_key(l: &DictCacheEntryNormal, r: TagKey) -> Ordering {
        l.0.as_u32().cmp(&r.as_u32())
    }

    /// Comparator function for searching `DictCache::sorted` array by [TagKey]
    /// and private creator
    fn cmp_cache_key_creator<'a>(
        l: &DictCacheEntryNormal,
        r_key: TagKey,
        r_creator: &'a Creator<'a>,
    ) -> Ordering {
        match l.0.as_u32().cmp(&r_key.as_u32()) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            // SAFETY: Cache is always cleared when any data it points to
            // mutated, so there is no chance to get a dangling pointer.
            Ordering::Equal => unsafe { l.1.as_ref().cmp(r_creator) },
        }
    }

    /// Performs a linear search of the [Tag] in a specified iterator.
    ///
    /// Returns Some if some match found or None if no matches found. Some
    /// contains a tuple of `bool` and matched `Meta`. `bool` indicates a
    /// confidentiality of a match. `true` - match exacted, `false` match with
    /// some "generalization".
    fn search_in_ary<'a, T: Iterator<Item = &'a Meta>>(
        iter: T,
        tag: &Tag,
    ) -> Option<(bool, &'a Meta)> {
        let mut matched = None;

        if !tag.key.is_private_attribute() {
            // Search for a regular tag
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    return Some((true, v));
                }
            }
        } else if tag.creator.is_none() {
            // Search for a private tag without known "creator".
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    return Some((v.tag.creator.is_none(), v));
                }
            }
        } else if let Some(canonical_key) =
            tag.key.to_canonical_if_private().filter(|v| *v != tag.key)
        {
            // Search for a private attribute with a known "creator"
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    if v.tag.creator.is_none() {
                        if matched.is_none() { // there may be a better alternative with a known creator
                            matched = Some((false, v));
                        }
                    } else if v.tag.creator == tag.creator {
                        return Some((true, v));
                    }
                }

                if v.tag.key.as_u32() == canonical_key.as_u32() & v.mask
                    && v.tag.creator == tag.creator
                {
                    return Some((true, v));
                }
            }
        } else {
            // Search for a private attribute in canonical form with a known "creator"
            for v in iter {
                if v.tag.key.as_u32() == tag.key.as_u32() & v.mask {
                    if v.tag.creator.is_none() {
                        if matched.is_none() { // there may be a better alternative with a known creator
                            matched = Some((false, v));
                        }
                    } else if v.tag.creator == tag.creator {
                        return Some((true, v));
                    }
                }
            }
        }

        matched
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod keys {
        use crate::declare_tags;
        use inventory::submit;
        declare_tags! {
            /// Test tags dictionary
            pub const TEST_TAG_LIST = [
                GenericGroupLength: { (0x0000, 0x0000) & 0x0001FFFF, UL, 1, "Generic Group Length", Dicom },
                PrivateGroupLength: { (0x0001, 0x0000) & 0x0001FFFF, UL, 1, "Private Group Length", Dicom },
                PrivateReservation: { (0x0001, 0x0000) & 0x0001FF00, LO,  1, "Private Reservation", Dicom },
                SpecificCharacterSet: { (0x0008, 0x0005), CS, 1-n, "Specific Character Set", Dicom },
                EscapeTriplet: { (0x1000, 0x0000) & 0xFFFF000F, US, 3, "Escape Triplet", Retired },
                ZonalMap: { (0x1010, 0x0000) & 0xFFFF0000, US, 1-n, "Zonal Map", Retired },
                OverlayRows: { (0x6000, 0x0010) & 0xFF00FFFF, US, 1, "Overlay Rows", Dicom },
                PixelData: { (0x7FE0, 0x0010), OB or OW, 1, "Pixel Data", Dicom },
                Item: { (0xFFFE, 0xE000), Undefined, 1, "Item", Dicom },
                ItemDelimitationItem: { (0xFFFE, 0xE00D), Undefined, 1, "Item Delimitation Item", Dicom},
                SequenceDelimitationItem: { (0xFFFE, 0xE0DD), Undefined, 1-10 n, "Sequence Delimitation Item", Dicom },
            ];

            pub const TEST_PRIVATE_TAG_LIST = [
                Vendor1_4321_AB: { (0x4321, 0x10AB), AS, 1, "", Vendored(None) },
                Vendor2_4321_AB: { (0x4321, 0x10AB, "vendor2"), AS, 1, "", Vendored(None) },
                Vendor3_4321_AB: { (0x4321, 0x10AB, "vendor3"), AS, 1, "", Vendored(None) },
                Vendor4_4321_AB: { (0x4321, 0x12AB), AS, 1, "", Vendored(None) },
                Vendor5_4321_AB: { (0x4321, 0x13AB), AS, 1, "", Vendored(None) },
                Vendor6_4321_AB: { (0x4321, 0x14AB, "vendor5"), AS, 1, "", Vendored(None) },
            ];
        }

        submit!(TEST_TAG_LIST);
        submit!(TEST_PRIVATE_TAG_LIST);
    }

    fn search_tags_in_dict(dict: &Dictionary) {
        fn assert_search(d: &Dictionary, searched: &Tag, expected: &Tag) {
            let found = d
                .search_by_tag(searched)
                .unwrap_or_else(|| panic!("tag \"{searched}\" was not found"));
            assert_eq!(found, expected);
        }

        // Standard attributes should always be searchable by it's tag
        for m in keys::TEST_TAG_LIST.value() {
            assert_search(dict, &m.tag, &m.tag);
        }
        // Private attributes should always be searchable by it's tag
        for m in keys::TEST_TAG_LIST.value() {
            assert_search(dict, &m.tag, &m.tag);
        }

        assert_search(dict, &Tag::standard(0x1010, 0x1234), &keys::ZonalMap);

        assert_search(dict, &Tag::standard(0x6001, 0x0010), &keys::OverlayRows);

        // If input has a creator, dict should ignore 0x12AB with no creator and fall back to 0x10AB
        assert_search(
            dict,
            &Tag::private(0x4321, 0x12AB, "vendor2"),
            &keys::Vendor2_4321_AB,
        );

        // If input has no creator, and dict has no creator dict should match 0x12AB
        assert_search(dict, &Tag::standard(0x4321, 0x12AB), &keys::Vendor4_4321_AB);

        // If input has no creator, but dict has one dict should also match
        assert_search(dict, &Tag::standard(0x4321, 0x14AB), &keys::Vendor6_4321_AB);

        // If input has a creator, and dict has no creator, it should match
        assert_search(
            dict,
            &Tag::private(0x4321, 0x12AB, "unknown"),
            &keys::Vendor4_4321_AB,
        );

        // Should not coerce to canonical form if no private creator given
        assert!(dict.search_by_tag(&Tag::standard(0x4321, 0x15AB)).is_none());

        // Should not match 0x14AB because of different creator. Also should not match
        // "canonical" 0x10AB, because "canonical" form requires exact creator match.
        assert!(dict
            .search_by_tag(&Tag::private(0x4321, 0x14AB, "unknown"))
            .is_none());

        // Should find private reservations
        assert_search(dict, &Tag::standard(0x1221, 0x0022), &keys::PrivateReservation);
        // Should find arbitrary group length
        assert_search(dict, &Tag::standard(0xB00A, 0x0000), &keys::GenericGroupLength);
        // Should find private group length
        assert_search(dict, &Tag::standard(0xB00B, 0x0000), &keys::PrivateGroupLength);
    }

    #[test]
    fn is_dict_searchable() {
        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&keys::TEST_TAG_LIST);
        dict.add_static_list(&keys::TEST_PRIVATE_TAG_LIST);

        // Search in non-cached dictionary
        search_tags_in_dict(&dict);
    }

    #[test]
    fn is_dict_cache_searchable() {
        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&keys::TEST_TAG_LIST);
        dict.add_static_list(&keys::TEST_PRIVATE_TAG_LIST);

        dict.rebuild_cache();
        search_tags_in_dict(&dict);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn is_dict_auto_collects_statics() {
        let dict = Dictionary::new();
        search_tags_in_dict(&dict);
    }

    #[test]
    #[cfg(not(miri))]
    #[allow(deprecated)]
    fn can_parse_bundled_tsv() {
        const FILE_NAMES: &[&str] = &[
            concat!(env!("CARGO_MANIFEST_DIR"), "/etc/dicom.tsv"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/etc/diconde.tsv"),
        ];

        let mut dict = Dictionary::new_empty();
        for file_name in FILE_NAMES {
            match dict.add_from_file(file_name) {
                Ok(_) => (),
                Err(e) => panic!("Unable to parse {file_name}: {e}"),
            }
        }
        fn check_attributes(dict: &Dictionary) {
            // Should find masked tags
            assert_eq!(dict.search_by_tag(&dicom::EscapeTriplet).unwrap().tag.key, dicom::EscapeTriplet);
            assert_eq!(dict.search_by_tag(&Tag::standard(0x1221, 0x0022)).unwrap().tag.key, dicom::PrivateReservation);
            // Should find arbitrary group length
            assert_eq!(dict.search_by_tag(&Tag::standard(0xB00A, 0x0000)).unwrap().tag.key, dicom::GroupLength);
            // Should find private group length
            assert_eq!(dict.search_by_tag(&Tag::standard(0xB00B, 0x0000)).unwrap().tag.key, dicom::PrivateGroupLength);
        }
        check_attributes(&dict);
        dict.rebuild_cache();
        check_attributes(&dict);

    }
}
