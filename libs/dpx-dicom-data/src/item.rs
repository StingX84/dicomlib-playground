use std::borrow::Cow;

use bytes::Bytes;
use dpx_dicom_core::error::Result;
use dpx_dicom_core::{Tag, TagKey, Vr, dicom_err};

use crate::convert::{self, FromNumber};
use crate::dataset::Shared;
use crate::sequence::{Sequence, SequenceRef};
use crate::value::{Element, Stored, TagHeader, Value};

/// A data-quality anomaly noticed while adding a parsed element, surfaced so the
/// parser can log it at INFO without the storage layer depending on tracing.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PushNote {
    /// The tag was already present (kept as-is under `file_offsets`, otherwise
    /// the previous value was overwritten).
    Duplicate,
    /// The tag arrived out of ascending order.
    OutOfOrder,
}

/// Flat attribute store: one allocation and a cache-friendly layout for the
/// parse-read-discard workloads this library targets. Without `file_offsets`
/// it is tag-sorted for binary-search lookup; with `file_offsets` it keeps the
/// file's original order (and any duplicates) for a faithful GUI view, at the
/// cost of linear lookup.
#[derive(Debug, Clone, Default)]
pub(crate) struct ElementMap {
    entries: Vec<(TagKey, Element)>,
}

impl ElementMap {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self { entries: Vec::with_capacity(capacity) }
    }
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Locates `key`. With `file_offsets` the map preserves file order (for a
    /// faithful GUI view), so this is a linear scan returning the first match
    /// and `Err(len)` (i.e. "append") on miss. Otherwise the map is tag-sorted
    /// and this is a binary search returning the sorted insertion point on miss.
    #[cfg(not(feature = "file_offsets"))]
    fn search(&self, key: TagKey) -> std::result::Result<usize, usize> {
        self.entries.binary_search_by(|(k, _)| k.0.cmp(&key.0))
    }
    #[cfg(feature = "file_offsets")]
    fn search(&self, key: TagKey) -> std::result::Result<usize, usize> {
        match self.entries.iter().position(|(k, _)| k.0 == key.0) {
            Some(i) => Ok(i),
            None => Err(self.entries.len()),
        }
    }
    pub(crate) fn get(&self, key: TagKey) -> Option<&Element> {
        self.search(key).ok().map(|i| &self.entries[i].1)
    }
    pub(crate) fn get_mut(&mut self, key: TagKey) -> Option<&mut Element> {
        match self.search(key) {
            Ok(i) => Some(&mut self.entries[i].1),
            Err(_) => None,
        }
    }
    pub(crate) fn contains_key(&self, key: TagKey) -> bool {
        self.search(key).is_ok()
    }
    pub(crate) fn insert(&mut self, key: TagKey, element: Element) -> Option<Element> {
        match self.search(key) {
            Ok(i) => Some(std::mem::replace(&mut self.entries[i].1, element)),
            Err(i) => {
                self.entries.insert(i, (key, element));
                None
            }
        }
    }
    /// Adds an element discovered while parsing, tolerating real-world files
    /// whose tags are out of order or duplicated. Returns a [`PushNote`] when
    /// the data was anomalous, so the parser can log it.
    ///
    /// With `file_offsets` the map keeps file order verbatim (duplicates and all)
    /// for a faithful GUI view. Otherwise it keeps the tag-sorted invariant that
    /// the binary-search lookups rely on: an in-order tag is appended (fast
    /// path), an out-of-order one is sorted-inserted, a duplicate replaces the
    /// previous value (last wins, as DCMTK does).
    #[cfg(not(feature = "file_offsets"))]
    pub(crate) fn push_parsed(&mut self, key: TagKey, element: Element) -> Option<PushNote> {
        if self.entries.last().is_none_or(|(k, _)| k.0 < key.0) {
            self.entries.push((key, element));
            None
        } else {
            match self.search(key) {
                Ok(i) => {
                    self.entries[i].1 = element;
                    Some(PushNote::Duplicate)
                }
                Err(i) => {
                    self.entries.insert(i, (key, element));
                    Some(PushNote::OutOfOrder)
                }
            }
        }
    }
    #[cfg(feature = "file_offsets")]
    pub(crate) fn push_parsed(&mut self, key: TagKey, element: Element) -> Option<PushNote> {
        let note = match self.entries.last() {
            Some((k, _)) if k.0 == key.0 => Some(PushNote::Duplicate),
            Some((k, _)) if k.0 > key.0 => Some(PushNote::OutOfOrder),
            _ => None,
        };
        self.entries.push((key, element));
        note
    }
    pub(crate) fn remove(&mut self, key: TagKey) -> Option<Element> {
        match self.search(key) {
            Ok(i) => Some(self.entries.remove(i).1),
            Err(_) => None,
        }
    }
    pub(crate) fn get_or_insert_with(&mut self, key: TagKey, f: impl FnOnce() -> Element) -> &mut Element {
        let i = match self.search(key) {
            Ok(i) => i,
            Err(i) => {
                self.entries.insert(i, (key, f()));
                i
            }
        };
        &mut self.entries[i].1
    }
    pub(crate) fn into_entries(self) -> Vec<(TagKey, Element)> {
        self.entries
    }
    pub(crate) fn entries(&self) -> &[(TagKey, Element)] {
        &self.entries
    }
    pub(crate) fn entries_mut(&mut self) -> &mut [(TagKey, Element)] {
        &mut self.entries
    }
}

/// A context-free attribute map: the content of a data set or a sequence item.
///
/// All read/write logic lives here and takes the owning root's [`Shared`]
/// context as a parameter; [`DataSet`](crate::DataSet) and the item handles are
/// thin delegators over these methods.
#[derive(Debug, Clone, Default)]
pub struct Item {
    pub(crate) map: ElementMap,
    /// On-disk headers of this item's own delimiters (Item / Item Delimitation
    /// Item), when parsed from a file under the `file_offsets` feature.
    #[cfg(feature = "file_offsets")]
    pub(crate) header: Vec<TagHeader>,
}

impl Item {
    /// Builds an item from a prepared attribute map (used by the parser and
    /// adaptation).
    pub(crate) fn from_map(map: ElementMap) -> Self {
        Self {
            map,
            #[cfg(feature = "file_offsets")]
            header: Vec::new(),
        }
    }
}

/// Resolves the VR for a write from the active tag dictionary.
pub(crate) fn vr_for_write(tag: &Tag) -> Result<Vr> {
    match tag.meta() {
        Some(m) if m.vr.0 != Vr::Undefined => Ok(m.vr.0),
        _ => Err(dicom_err!(NotFound, "no VR known for tag {tag}; use set_with_vr")),
    }
}

impl Item {
    /// Borrows the raw bytes of an attribute when byte-backed, resolving
    /// `Mapped` slices against `master`. Used by `DataSet::sync_context`.
    pub(crate) fn raw_bytes<'a>(&'a self, master: &'a Bytes, key: TagKey) -> Option<&'a [u8]> {
        match &self.map.get(key)?.value {
            Stored::Mapped(range) => master.get(range.clone()),
            Stored::Owned(bytes) => Some(&bytes[..]),
            Stored::Native(Value::Str(s)) => Some(s.as_bytes()),
            _ => None,
        }
    }

    // --- Tag -> TagKey boundary --------------------------------------------
    // ponytail: private-creator reservation is deferred; both currently map to
    // the numeric key. They stay separate as the seam for that mechanism.
    fn resolve_read_key(&self, _shared: &Shared, tag: &Tag) -> Result<TagKey> {
        Ok(tag.key)
    }
    fn reserve_write_key(&mut self, _shared: &Shared, tag: &Tag) -> Result<TagKey> {
        Ok(tag.key)
    }

    fn element_bytes<'a>(&'a self, shared: &'a Shared, el: &'a Element) -> Option<&'a [u8]> {
        match &el.value {
            Stored::Mapped(range) => shared.master().get(range.clone()),
            Stored::Owned(bytes) => Some(&bytes[..]),
            Stored::Native(Value::Str(s)) => Some(s.as_bytes()),
            Stored::Native(Value::Bytes(b)) => Some(&b[..]),
            _ => None,
        }
    }

    fn element_value(&self, shared: &Shared, el: &Element) -> Result<Value> {
        match &el.value {
            Stored::Native(v) => Ok(v.clone()),
            Stored::Owned(bytes) => convert::decode(shared, el.vr, bytes),
            Stored::Mapped(range) => {
                let bytes = shared
                    .master()
                    .get(range.clone())
                    .ok_or_else(|| dicom_err!(Internal, "mapped value range out of bounds"))?;
                convert::decode(shared, el.vr, bytes)
            }
            Stored::Items(_) => Err(dicom_err!(InvalidData, "attribute is a sequence, not a value")),
        }
    }

    fn element_str<'a>(&'a self, shared: &'a Shared, el: &'a Element) -> Result<Cow<'a, str>> {
        if let Stored::Native(Value::Str(s)) = &el.value {
            return Ok(Cow::Borrowed(s.as_str()));
        }
        let bytes = self
            .element_bytes(shared, el)
            .ok_or_else(|| dicom_err!(InvalidData, "value is not textual"))?;
        Ok(convert::decode_str(shared, el.vr, bytes))
    }

    pub(crate) fn value(&self, shared: &Shared, tag: &Tag) -> Result<Value> {
        let key = self.resolve_read_key(shared, tag)?;
        let el = self.map.get(key).ok_or_else(|| dicom_err!(NotFound, "attribute {tag} not found"))?;
        self.element_value(shared, el)
    }
    pub(crate) fn value_some(&self, shared: &Shared, tag: &Tag) -> Option<Value> {
        self.value(shared, tag).ok()
    }
    pub(crate) fn contains(&self, shared: &Shared, tag: &Tag) -> bool {
        self.resolve_read_key(shared, tag).is_ok_and(|k| self.map.contains_key(k))
    }
    pub(crate) fn vr_of(&self, shared: &Shared, tag: &Tag) -> Option<Vr> {
        let key = self.resolve_read_key(shared, tag).ok()?;
        self.map.get(key).map(|el| el.vr)
    }
    pub(crate) fn get_bytes<'a>(&'a self, shared: &'a Shared, tag: &Tag) -> Result<&'a [u8]> {
        let key = self.resolve_read_key(shared, tag)?;
        let el = self.map.get(key).ok_or_else(|| dicom_err!(NotFound, "attribute {tag} not found"))?;
        self.element_bytes(shared, el)
            .ok_or_else(|| dicom_err!(InvalidData, "value has no raw byte form; decode it instead"))
    }
    pub(crate) fn get_bytes_some<'a>(&'a self, shared: &'a Shared, tag: &Tag) -> Option<&'a [u8]> {
        let key = self.resolve_read_key(shared, tag).ok()?;
        self.element_bytes(shared, self.map.get(key)?)
    }
    pub(crate) fn get_str<'a>(&'a self, shared: &'a Shared, tag: &Tag) -> Result<Cow<'a, str>> {
        let key = self.resolve_read_key(shared, tag)?;
        let el = self.map.get(key).ok_or_else(|| dicom_err!(NotFound, "attribute {tag} not found"))?;
        self.element_str(shared, el)
    }
    pub(crate) fn get_str_some<'a>(&'a self, shared: &'a Shared, tag: &Tag) -> Option<Cow<'a, str>> {
        let key = self.resolve_read_key(shared, tag).ok()?;
        self.element_str(shared, self.map.get(key)?).ok()
    }
    /// Decodes `tag` and yields each value coerced to `T`. The iterator owns the
    /// decoded [`Value`], so it survives a `Mapped`/`Owned` source that had to be
    /// decoded into a temporary.
    pub(crate) fn get_iter<T: FromNumber>(&self, shared: &Shared, tag: &Tag) -> Result<convert::OwnedNumbers<T>> {
        let value = self.value(shared, tag)?;
        Ok(convert::owned_numbers(value))
    }

    /// Materializes `tag` to a `Native` string in place, then borrows it as an
    /// iterator over its `'\'`-separated tokens (trailing space/NUL trimmed).
    pub(crate) fn get_str_iter<'a>(
        &'a mut self,
        shared: &Shared,
        tag: &Tag,
    ) -> Result<impl Iterator<Item = &'a str>> {
        let key = self.resolve_read_key(shared, tag)?;
        let el = self.map.get(key).ok_or_else(|| dicom_err!(NotFound, "attribute {tag} not found"))?;
        if !matches!(el.value, Stored::Native(Value::Str(_))) {
            let s = self.element_str(shared, el)?.into_owned();
            // Re-fetch mutably: `element_str` borrowed `self` immutably above.
            let el = self.map.get_mut(key).ok_or_else(|| dicom_err!(NotFound, "attribute {tag} not found"))?;
            el.value = Stored::Native(Value::Str(s));
        }
        let el = self.map.get(key).ok_or_else(|| dicom_err!(NotFound, "attribute {tag} not found"))?;
        match &el.value {
            Stored::Native(Value::Str(s)) => Ok(s.split('\\').map(|t| t.trim_end_matches([' ', '\0']))),
            _ => Err(dicom_err!(InvalidData, "value is not textual")),
        }
    }

    pub(crate) fn set_value(&mut self, shared: &Shared, tag: &Tag, value: Value) -> Result<()> {
        let vr = vr_for_write(tag)?;
        let key = self.reserve_write_key(shared, tag)?;
        self.map.insert(key, Element::new(vr, Stored::Native(value)));
        Ok(())
    }
    pub(crate) fn set_with_vr(&mut self, shared: &Shared, tag: &Tag, vr: Vr, value: Value) -> Result<()> {
        let key = self.reserve_write_key(shared, tag)?;
        self.map.insert(key, Element::new(vr, Stored::Native(value)));
        Ok(())
    }

    /// File-component headers of `tag`, or `&[]` when manually added or the
    /// `file_offsets` feature is off.
    pub(crate) fn header_of(&self, shared: &Shared, tag: &Tag) -> &[TagHeader] {
        #[cfg(feature = "file_offsets")]
        {
            match self.resolve_read_key(shared, tag) {
                Ok(key) => self.map.get(key).map(|el| el.header.as_slice()).unwrap_or(&[]),
                Err(_) => &[],
            }
        }
        #[cfg(not(feature = "file_offsets"))]
        {
            let _ = (shared, tag);
            &[]
        }
    }

    /// This item's own delimiter headers (Item / Item Delimitation Item).
    pub(crate) fn item_headers(&self) -> &[TagHeader] {
        #[cfg(feature = "file_offsets")]
        {
            &self.header
        }
        #[cfg(not(feature = "file_offsets"))]
        {
            &[]
        }
    }

    pub(crate) fn remove(&mut self, shared: &Shared, tag: &Tag) -> bool {
        match self.resolve_read_key(shared, tag) {
            Ok(key) => self.map.remove(key).is_some(),
            Err(_) => false,
        }
    }
    pub(crate) fn sequence_mut<'a>(&'a mut self, shared: &'a Shared, tag: &Tag) -> Result<Sequence<'a>> {
        let key = self.reserve_write_key(shared, tag)?;
        let el = self.map.get_or_insert_with(key, || Element::new(Vr::SQ, Stored::Items(Vec::new())));
        match &mut el.value {
            Stored::Items(items) => Ok(Sequence { shared, items }),
            _ => Err(dicom_err!(InvalidData, "{tag} is present with a non-sequence VR")),
        }
    }
    pub(crate) fn sequence<'a>(&'a self, shared: &'a Shared, tag: &Tag) -> Option<SequenceRef<'a>> {
        let key = self.resolve_read_key(shared, tag).ok()?;
        match &self.map.get(key)?.value {
            Stored::Items(items) => Some(SequenceRef { shared, items }),
            _ => None,
        }
    }
}

/// Generates the read accessors that delegate to the [`Item`] logic. The host
/// type provides `ctx(&self) -> (&Shared, &Item)`.
macro_rules! read_accessors {
    () => {
        /// Decodes `tag` to a dynamic [`Value`](crate::Value). Absent is an error.
        pub fn value(&self, tag: &Tag) -> Result<Value> {
            let (s, i) = self.ctx();
            i.value(s, tag)
        }
        /// Decodes `tag` to a [`Value`](crate::Value), or `None` if absent/undecodable.
        pub fn value_some(&self, tag: &Tag) -> Option<Value> {
            let (s, i) = self.ctx();
            i.value_some(s, tag)
        }
        /// Reads `tag` as `T` (first value for VM>1). Absent is an error.
        pub fn get<T: FromValue>(&self, tag: &Tag) -> Result<T> {
            T::from_value(&self.value(tag)?)
        }
        /// Reads `tag` as `T`, or `None` if absent or on any conversion error.
        pub fn get_some<T: FromValue>(&self, tag: &Tag) -> Option<T> {
            T::from_value(&self.value_some(tag)?).ok()
        }
        /// Reads every value of `tag` as `T`.
        pub fn get_all<T: FromValue>(&self, tag: &Tag) -> Result<Vec<T>> {
            T::from_value_all(&self.value(tag)?)
        }
        /// Iterates every value of `tag` coerced to numeric `T` (the same coercion
        /// `set`/write use: integer/float casts, string tokens parsed). The
        /// iterator owns the decoded value, so it is valid even when `tag` was a
        /// raw mapped/owned element. For text multi-values use [`Self::get_str_iter`].
        pub fn get_iter<T: $crate::convert::FromNumber>(&self, tag: &Tag) -> Result<impl Iterator<Item = T>> {
            let (s, i) = self.ctx();
            i.get_iter::<T>(s, tag)
        }
        /// Whether `tag` is present.
        pub fn contains(&self, tag: &Tag) -> bool {
            let (s, i) = self.ctx();
            i.contains(s, tag)
        }
        /// The stored Value Representation of `tag`, if present.
        pub fn vr(&self, tag: &Tag) -> Option<Vr> {
            let (s, i) = self.ctx();
            i.vr_of(s, tag)
        }
        /// Borrows the raw bytes of `tag` (zero-copy).
        pub fn get_bytes(&self, tag: &Tag) -> Result<&[u8]> {
            let (s, i) = self.ctx();
            i.get_bytes(s, tag)
        }
        /// Borrows the raw bytes of `tag`, or `None` if absent or non-byte.
        pub fn get_bytes_some(&self, tag: &Tag) -> Option<&[u8]> {
            let (s, i) = self.ctx();
            i.get_bytes_some(s, tag)
        }
        /// Reads `tag` as a string view, borrowing when the charset allows.
        pub fn get_str(&self, tag: &Tag) -> Result<Cow<'_, str>> {
            let (s, i) = self.ctx();
            i.get_str(s, tag)
        }
        /// Reads `tag` as a string view, or `None` if absent or non-textual.
        pub fn get_str_some(&self, tag: &Tag) -> Option<Cow<'_, str>> {
            let (s, i) = self.ctx();
            i.get_str_some(s, tag)
        }
        /// Read access to the sequence at `tag`, if present and `VR = SQ`.
        pub fn sequence(&self, tag: &Tag) -> Option<SequenceRef<'_>> {
            let (s, i) = self.ctx();
            i.sequence(s, tag)
        }
        /// On-disk component headers of `tag` (hex-dump / inspection). Empty if
        /// the attribute was added manually or `file_offsets` is off.
        pub fn headers(&self, tag: &Tag) -> &[TagHeader] {
            let (s, i) = self.ctx();
            i.header_of(s, tag)
        }
        /// This container's own delimiter headers (Item / Item Delimitation Item).
        pub fn item_headers(&self) -> &[TagHeader] {
            self.ctx().1.item_headers()
        }
    };
}

/// Generates the write accessors. The host type provides
/// `ctx_mut(&mut self) -> (&Shared, &mut Item)`.
macro_rules! write_accessors {
    () => {
        /// Stores `value` under `tag`, resolving the VR from the dictionary.
        pub fn set_value(&mut self, tag: &Tag, value: Value) -> Result<()> {
            self.before_set(tag, &value);
            let (s, i) = self.ctx_mut();
            i.set_value(s, tag, value)
        }
        /// Stores `value` under `tag` with an explicit VR.
        pub fn set_with_vr(&mut self, tag: &Tag, vr: Vr, value: Value) -> Result<()> {
            self.before_set(tag, &value);
            let (s, i) = self.ctx_mut();
            i.set_with_vr(s, tag, vr, value)
        }
        /// Stores a typed value under `tag`.
        pub fn set<T: IntoValue>(&mut self, tag: &Tag, value: T) -> Result<()> {
            self.set_value(tag, value.into_value())
        }
        /// Materializes `tag` to a decoded string and iterates its `'\'`-separated
        /// tokens as zero-copy `&str` (trailing space/NUL trimmed). Needs `&mut`
        /// because it caches the decoded form in place.
        pub fn get_str_iter(&mut self, tag: &Tag) -> Result<impl Iterator<Item = &str>> {
            let (s, i) = self.ctx_mut();
            i.get_str_iter(s, tag)
        }
        /// Removes `tag`, returning whether it was present.
        pub fn remove(&mut self, tag: &Tag) -> bool {
            let (s, i) = self.ctx_mut();
            i.remove(s, tag)
        }
        /// Create-or-get the sequence at `tag` for editing.
        pub fn sequence_mut(&mut self, tag: &Tag) -> Result<Sequence<'_>> {
            let (s, i) = self.ctx_mut();
            i.sequence_mut(s, tag)
        }
    };
}

pub(crate) use read_accessors;
pub(crate) use write_accessors;
