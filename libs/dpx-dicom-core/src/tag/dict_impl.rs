use super::*;
use crate::{Cow, Vr};

use std::io::BufRead;

#[derive(Debug, Clone)]
pub struct TagInfo<'a> {
    pub tag: Tag<'a>,
    pub mask: u32,
    pub vr: Vr,
    pub name: Cow<'a, str>,
    pub level: Level<'a>,
    pub section: Section<'a>,
}

impl<'a> PartialEq for TagInfo<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.tag.eq(&other.tag)
    }
}

impl<'a, 'b> PartialEq<Tag<'b>> for TagInfo<'a> {
    fn eq(&self, other: &Tag<'b>) -> bool {
        self.tag.eq(other)
    }
}

impl<'a> PartialEq<TagKey> for TagInfo<'a> {
    fn eq(&self, other: &TagKey) -> bool {
        self.tag.key.eq(other)
    }
}

impl<'a> Eq for TagInfo<'a> {}

impl<'a> PartialOrd for TagInfo<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.tag.partial_cmp(&other.tag)
    }
}

impl<'a, 'b> PartialOrd<Tag<'b>> for TagInfo<'a> {
    fn partial_cmp(&self, other: &Tag<'b>) -> Option<std::cmp::Ordering> {
        self.tag.partial_cmp(other)
    }
}

impl<'a> PartialOrd<TagKey> for TagInfo<'a> {
    fn partial_cmp(&self, other: &TagKey) -> Option<std::cmp::Ordering> {
        self.tag.key.partial_cmp(other)
    }
}

impl<'a> Ord for TagInfo<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tag.cmp(&other.tag)
    }
}

// ---------------------------------------------------------------------------
// dpx-dicom-core::tag::Level implementation
// ---------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct Level<'a> {
    pub root: Cow<'a, str>,
    pub level: Cow<'a, str>,
}

const ROOT_PATIENT: &str = "patient";
const LEVEL_PATIENT: &str = "patient";
const LEVEL_STUDY: &str = "study";
const LEVEL_SERIES: &str = "series";
const LEVEL_INSTANCE: &str = "instance";

#[derive(Debug, Clone)]
pub enum Section<'a> {
    Dicom,
    Diconde,
    Dicos,
    DicomRetired(Option<Cow<'a, str>>),
    Vendored(Cow<'a, str>),
}

#[derive(Clone)]
pub struct StaticDictionary(&'static [TagInfo<'static>]);
inventory::collect!(StaticDictionary);

// cSpell:ignore aabb
struct DictCache<'a> {
    // Contains Tag (element 0) and TagInfo (element 1) "flattened" from static and
    // dynamic dictionaries sorted by Tag.
    // Private attributes Tag's are "flattened" in 4 different forms:
    // 1. original form as given
    // 2. zeroed `aa` in TagKey (gggg,aabb)
    // 3. original TagKey and "zeroed" `creator` field in Tag
    // 4. 2 and 3 combined
    sorted: Vec<(Tag<'a>, &'a TagInfo<'a>)>,

    // Contains TagKey bitwise and'ed with mask (element 0), mask from TagInfo (element 1)
    // and TagInfo "flattened" from static and dynamic dictionaries.
    // This list contains only elements with a mask other than 0xFFFFFFFF
    masked: Vec<(u32, u32, &'a TagInfo<'a>)>,
}

#[derive(Default)]
pub struct Dictionary<'a> {
    statics: Vec<&'static StaticDictionary>,
    dynamic: Vec<TagInfo<'a>>,
    cache: Option<DictCache<'a>>,
}

impl<'a> Dictionary<'a> {
    // This function requires unstable, because of function "is_sorted_by"
    // Function is enabled only in "debug" builds. Other function variant
    // for "Release" builds is no-op.
    #[cfg(all(feature = "unstable", debug_assertions))]
    fn verify_sorted(dict: &'static StaticDictionary) {
        assert!(
            dict.0.is_sorted_by_key(|i| &i.tag ),
            "array in dpx_dicom_core::tag::StaticDictionary should be sorted by TagKey!"
        );
    }

    #[cfg(not(all(feature = "unstable", debug_assertions)))]
    const fn verify_sorted(_: &'static StaticDictionary) {}

    pub fn new() -> Self {
        let statics: Vec<&'static StaticDictionary> =
            inventory::iter::<StaticDictionary>.into_iter().collect();
        for dict in statics.iter() {
            Self::verify_sorted(dict);
        }

        Self {
            statics,
            dynamic: Vec::new(),
            cache: None,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            statics: Vec::new(),
            dynamic: Vec::new(),
            cache: None,
        }
    }

    pub fn add_static_dict(&mut self, dict: &'static StaticDictionary) {
        if !self
            .statics
            .iter()
            .any(|e| core::ptr::eq((*e) as *const _, dict as *const _))
        {
            Self::verify_sorted(dict);
            self.statics.push(dict);
            self.cache = None;
        }
    }

    pub fn add_dynamic_dict<'b: 'a, T: Iterator<Item = TagInfo<'b>>>(&mut self, iter: T) {
        self.dynamic.reserve(iter.size_hint().1.unwrap_or(0));
        for v in iter {
            self.dynamic.push(v);
        }
        self.cache = None;
    }

    pub fn add_memory_dict(&mut self, buf: &mut impl std::io::Read) -> Result<()> {
        let reader = std::io::BufReader::new(buf);
        let mut dict = Vec::<TagInfo<'static>>::new();

        for line in reader.buffer().lines() {
            let line = line.context(FailedToReadTagDictionaryFileSnafu)?;
            if let Some(tag_info) = Self::parse_file_line(line.trim())? {
                dict.push(tag_info);
            }
        }
        self.add_dynamic_dict(dict.into_iter());
        Ok(())
    }

    pub fn add_file_dict(&mut self, file_name: impl AsRef<Path>) -> Result<()> {
        use std::fs::File;
        let mut file = File::open(file_name.as_ref()).context(FailedToOpenTagDictionaryFileSnafu{file_name: file_name.as_ref().to_path_buf()})?;
        self.add_memory_dict(&mut file)
    }

    pub fn rebuild_cache(&'a mut self) {
        let mut cache = DictCache::<'a> {
            sorted: Vec::new(),
            masked: Vec::new(),
        };

        let guessed_total_count =
            self.statics.iter().fold(0, |acc, dict| acc + dict.0.len()) + self.dynamic.len();
        cache.sorted.reserve(guessed_total_count);

        let guessed_masked_count = self
            .statics
            .iter()
            .map(|dict| dict.0.iter().filter(|v| v.mask != 0xFFFFFFFFu32).count())
            .sum::<usize>()
            + self
                .dynamic
                .iter()
                .filter(|v| v.mask != 0xFFFFFFFFu32)
                .count();
        cache.masked.reserve(guessed_masked_count);

        // Reverse order, because after stable sort and dedup, we want
        // to prioritize dynamically added attributes over statically
        // added attributes. Each list of attributes also prioritizes
        // lastly added.
        for tag_info in self.dynamic.iter().rev() {
            Self::add_cached_tag(tag_info, &mut cache);
        }

        for dict in self.statics.iter().rev() {
            for tag_info in dict.0.iter() {
                Self::add_cached_tag(tag_info, &mut cache)
            }
        }

        cache.sorted.sort_by(|l, r| l.0.cmp(&r.0));
        cache.sorted.dedup_by(|l, r| l.0 == r.0);

        self.cache = Some(cache);
    }

    pub fn get_by_tag_key(&self, key: TagKey) -> Option<&'a TagInfo> {
        // Search in the cache if it is available
        if let Some(c) = &self.cache {
            // first, search directly in a sorted "flattened" array
            if let Ok(index) = c.sorted.binary_search_by(|v| v.0.key.0.cmp(&key.0)) {
                // Safety: we've got this VALID index from the vector method and there is no way
                // to mutate vector content after the search.
                return Some(unsafe { c.sorted.get_unchecked(index).1 });
            }

            // Then, search in an UNSORTED array containing only tags with mask
            // This array expected to be small enough to do a O(N) search.
            return c.masked.iter().find(|v| v.0 == key.0 & v.1).map(|v| v.2);
        }

        // Search the hard-way.
        let tag = Tag::new(key, None);
        if let Some(v) = Self::search_in_ary(self.dynamic.iter(), &tag) {
            return Some(v);
        }
        for ary in self.statics.iter().rev() {
            if let Some(v) = Self::search_in_ary(ary.0.iter(), &tag) {
                return Some(v);
            }
        }

        None
    }

    pub fn get_by_tag(&self, tag: &Tag) -> Option<&'a TagInfo> {
        // Search in the cache if it is available
        if let Some(c) = &self.cache {
            // first, search directly in a sorted "flattened" array
            if let Ok(index) = c.sorted.binary_search_by(|v| v.0.cmp(tag)) {
                // Safety: we've got this VALID index from the vector method and there is no way
                // to mutate vector content after the search.
                return Some(unsafe { c.sorted.get_unchecked(index).1 });
            }

            // Then, search in an UNSORTED array containing only tags with mask
            // This array expected to be small enough to do a O(N) search.
            return c
                .masked
                .iter()
                .find(|v| v.0 == tag.key.0 & v.1)
                .map(|v| v.2);
        }

        // Search the hard-way.
        if let Some(v) = Self::search_in_ary(self.dynamic.iter(), tag) {
            return Some(v);
        }
        for ary in self.statics.iter().rev() {
            if let Some(v) = Self::search_in_ary(ary.0.iter(), tag) {
                return Some(v);
            }
        }

        None
    }
}

// Private implementations
impl<'a> Dictionary<'a> {
    fn parse_file_line(line: &str) -> Result<Option<TagInfo<'static>>> {
        if line.starts_with('#') {
            return Ok(None);
        }
        unimplemented!()
    }
    fn add_cached_tag(tag_info: &'a TagInfo<'a>, cache: &'_ mut DictCache<'a>) {
        let key = &tag_info.tag.key;

        // Add tag as-is
        cache.sorted.push((tag_info.tag.clone(), tag_info));

        // Add "masked" tag in a special array
        if tag_info.mask != 0xFFFFFFFFu32 {
            debug_assert_eq!(key.as_u32() & tag_info.mask, key.as_u32(),
                "TagInfo for tag {} in dpx_dicom_core::tag::StaticDictionary must be pre-multiplied by it's mask {:08x}",
                tag_info.tag, tag_info.mask);
            cache.masked.push((key.as_u32(), tag_info.mask, tag_info));
        }

        // Note: if you did not provide 'private creator' for the private attribute,
        // then it will be matched "exactly". This is a "compatibility" feature with
        // some buggy software, that does not provide any "private creator" for their
        // attributes.
        if key.is_private() && tag_info.tag.creator.is_some() {
            // Add private attribute without "private creator"
            if key.is_private_attribute() {
                cache.sorted.push((
                    Tag {
                        creator: None,
                        ..tag_info.tag
                    },
                    tag_info,
                ));
            }

            // Add normalized attribute
            if let Some(normalized_key) = key.to_canonical_if_private() {
                cache.sorted.push((
                    Tag {
                        key: normalized_key,
                        ..tag_info.tag.clone()
                    },
                    tag_info,
                ));
            }
        }
    }

    fn search_in_ary<T: Iterator<Item = &'a TagInfo<'a>>>(
        iter: T,
        tag: &'_ Tag<'_>,
    ) -> Option<&'a TagInfo<'a>> {
        let mut matched = None;

        if !tag.key.is_private() || tag.creator.is_none() {
            // Search for a regular tag OR private without known "creator".
            // We can't coerce private attributes and reservations to it's canonical form,
            // because without known "creator" we will collide with someone other private attribute.
            for v in iter {
                // Condition 1: exact match
                if v.tag == *tag {
                    matched = Some(v);
                    break;
                }

                // Condition 2: is searched tag in the masked range
                if v.mask != 0xFFFFFFFFu32 && v.tag.key.0 & v.mask == tag.key.0 & v.mask {
                    matched = Some(v);
                    break;
                }
            }
        } else if let Some(canonical_key) =
            tag.key.to_canonical_if_private().filter(|v| *v != tag.key)
        {
            const WEIGHT_MATCH_KEY: u8 = 4;
            const WEIGHT_MATCH_MASK: u8 = 3;
            const WEIGHT_MATCH_CANONICAL_KEY: u8 = 2;
            const WEIGHT_MATCH_CANONICAL_MASK: u8 = 1;
            let mut matched_weight = 0u8;

            // This is a private reservation or attribute in non-canonical form with a known searched "creator".
            for v in iter {
                // Condition 1: searched Tag has exact match
                if v.tag == *tag {
                    matched = Some(v);
                    break;
                }

                // Mask is non zero and contains high 16-bits of the original mask IF
                // it contains non-zero bits in high 16-bits and has no masked out bits in lower 16-bits
                let mask = {
                    if v.mask != 0xFFFFFFFFu32 && v.mask & 0x0000FFFFu32 == 0x0000FFFFu32 {
                        v.mask & 0xFFFF0000u32
                    } else {
                        0u32
                    }
                };

                // Condition 2: "creator" matches and TagKey from searched Tag is in masked range
                if mask != 0u32
                    && v.tag.key.0 & mask == tag.key.0 & mask
                    && v.tag.creator == tag.creator
                {
                    matched = Some(v);
                    break;
                }

                // Condition 3: "creator" matches and canonical TagKey from searched Tag matches.
                if v.tag.key == canonical_key && v.tag.creator == tag.creator {
                    return Some(v);
                }

                // Condition 4: "creator" matches and canonical TagKey from searched Tag in masked range.
                if mask != 0u32
                    && v.tag.key.0 & mask == canonical_key.0 & mask
                    && v.tag.creator == tag.creator
                {
                    matched = Some(v);
                    break;
                }

                // Other matches are the "best guess" if our dictionary has no "creator"
                if matched_weight < WEIGHT_MATCH_KEY && v.tag.creator.is_none() {
                    // Condition 5: dict has no "creator", but TagKey exactly matched
                    if v.tag.key == tag.key {
                        matched_weight = WEIGHT_MATCH_KEY;
                        matched = Some(v);
                        continue;
                    }

                    if matched_weight < WEIGHT_MATCH_MASK {
                        // Condition 6: dict has no "creator", bur TagKey falls into the masked range
                        if mask != 0u32 && v.tag.key.0 & mask == tag.key.0 & mask {
                            matched_weight = WEIGHT_MATCH_MASK;
                            matched = Some(v);
                            continue;
                        }

                        if matched_weight < WEIGHT_MATCH_CANONICAL_KEY {
                            // Condition 7: dict has no "creator", but canonical TagKey matches
                            if v.tag.key == canonical_key {
                                matched_weight = WEIGHT_MATCH_CANONICAL_MASK;
                                matched = Some(v);
                                continue;
                            }

                            // Condition 8: dict has no "creator"
                            if matched_weight == 0
                                && mask != 0u32
                                && v.tag.key.0 & mask == canonical_key.0 & mask
                            {
                                matched_weight = WEIGHT_MATCH_CANONICAL_MASK;
                                matched = Some(v);
                                continue;
                            }
                        }
                    }
                }
            }
        } else {
            const WEIGHT_MATCH_KEY: u8 = 2;
            const WEIGHT_MATCH_MASK: u8 = 1;
            let mut matched_weight = 0u8;

            // This is a private reservation or attribute in non-canonical form with a known searched "creator".
            for v in iter {
                // Condition 1: searched Tag has exact match
                if v.tag == *tag {
                    matched = Some(v);
                    break;
                }

                // Mask is non zero and contains high 16-bits of the original mask IF
                // it contains non-zero bits in high 16-bits and has no masked out bits in lower 16-bits
                let mask = {
                    if v.mask != 0xFFFFFFFFu32 && v.mask & 0x0000FFFFu32 == 0x0000FFFFu32 {
                        v.mask & 0xFFFF0000u32
                    } else {
                        0u32
                    }
                };

                // Condition 2: "creator" matches and TagKey from searched Tag is in masked range
                if mask != 0u32
                    && v.tag.key.0 & mask == tag.key.0 & mask
                    && v.tag.creator == tag.creator
                {
                    matched = Some(v);
                    break;
                }

                // Other matches are the "best guess" if our dictionary has no "creator"
                if matched_weight < WEIGHT_MATCH_KEY && v.tag.creator.is_none() {
                    // Condition 5: dict has no "creator", but TagKey exactly matched
                    if v.tag.key == tag.key {
                        matched_weight = WEIGHT_MATCH_KEY;
                        matched = Some(v);
                        continue;
                    }

                    // Condition 6: dict has no "creator", bur TagKey falls into the masked range
                    if matched_weight < WEIGHT_MATCH_MASK
                        && mask != 0u32
                        && v.tag.key.0 & mask == tag.key.0 & mask
                    {
                        matched_weight = WEIGHT_MATCH_MASK;
                        matched = Some(v);
                        continue;
                    }
                }
            }
        }

        matched
    }
}
