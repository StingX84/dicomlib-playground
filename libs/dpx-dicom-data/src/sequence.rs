use std::borrow::Cow;

use dpx_dicom_core::error::Result;
use dpx_dicom_core::{Tag, Vr};

use crate::adapt::adapt_dataset;
use crate::convert::{FromValue, IntoValue};
use crate::dataset::{DataSet, Shared};
use crate::item::{Item, read_accessors, write_accessors};
use crate::value::{TagHeader, Value};

/// Mutable handle to a sequence (`VR = SQ`) value, borrowing the owning root's
/// context and the item list it edits.
pub struct Sequence<'a> {
    pub(crate) shared: &'a Shared,
    pub(crate) items: &'a mut Vec<Item>,
}

impl Sequence<'_> {
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Read access to item `index`.
    pub fn item(&self, index: usize) -> Option<ItemRef<'_>> {
        self.items.get(index).map(|item| ItemRef { shared: self.shared, item })
    }
    /// Mutable access to item `index`.
    pub fn item_mut(&mut self, index: usize) -> Option<ItemMut<'_>> {
        let shared = self.shared;
        self.items.get_mut(index).map(|item| ItemMut { shared, item })
    }
    /// Iterates items for reading.
    pub fn iter(&self) -> impl Iterator<Item = ItemRef<'_>> {
        let shared = self.shared;
        self.items.iter().map(move |item| ItemRef { shared, item })
    }
    /// Appends an empty item, born in this root's context, and returns it for
    /// filling. No adaptation needed (escape hatch from requirement Q1).
    pub fn new_item(&mut self) -> ItemMut<'_> {
        self.items.push(Item::default());
        let index = self.items.len() - 1;
        ItemMut { shared: self.shared, item: &mut self.items[index] }
    }
    /// Appends a foreign data set as an item, adapting it to this root's context.
    pub fn push(&mut self, ds: DataSet) -> Result<()> {
        let item = adapt_dataset(self.shared, ds)?;
        self.items.push(item);
        Ok(())
    }
    /// Removes all items.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

/// Read-only handle to a sequence value.
pub struct SequenceRef<'a> {
    pub(crate) shared: &'a Shared,
    pub(crate) items: &'a [Item],
}

impl SequenceRef<'_> {
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn item(&self, index: usize) -> Option<ItemRef<'_>> {
        self.items.get(index).map(|item| ItemRef { shared: self.shared, item })
    }
    pub fn iter(&self) -> impl Iterator<Item = ItemRef<'_>> {
        let shared = self.shared;
        self.items.iter().map(move |item| ItemRef { shared, item })
    }
}

/// Read handle to a sequence item, carrying the owning root's context.
pub struct ItemRef<'a> {
    pub(crate) shared: &'a Shared,
    pub(crate) item: &'a Item,
}

impl ItemRef<'_> {
    fn ctx(&self) -> (&Shared, &Item) {
        (self.shared, self.item)
    }
    read_accessors!();
}

/// Mutable handle to a sequence item, carrying the owning root's context.
pub struct ItemMut<'a> {
    pub(crate) shared: &'a Shared,
    pub(crate) item: &'a mut Item,
}

impl ItemMut<'_> {
    fn ctx(&self) -> (&Shared, &Item) {
        (self.shared, self.item)
    }
    fn ctx_mut(&mut self) -> (&Shared, &mut Item) {
        (self.shared, self.item)
    }
    /// Nested items have only `&Shared`; charset auto-stamping is a root-only
    /// concern, so this is a no-op here (see `DataSet::before_set`).
    fn before_set(&mut self, _tag: &Tag, _value: &Value) {}
    read_accessors!();
    write_accessors!();
}
