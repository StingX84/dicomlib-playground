//! Adaptation of a foreign data set into a context-free [`Item`] when it is
//! inserted as a sequence item under a different root context.

use bytes::Bytes;
use dpx_dicom_core::Vr;
use dpx_dicom_core::dicom_err;
use dpx_dicom_core::error::Result;

use crate::convert;
use crate::dataset::{DataSet, Shared};
use crate::item::{ElementMap, Item};
use crate::value::{Element, Stored};

/// Converts a foreign data set into a context-free [`Item`] under `dest`.
///
/// Native values are charset/endianness-agnostic and move unchanged. Raw
/// (`Mapped`/`Owned`) values in the same context as `dest` move as owned bytes;
/// in a differing context they are decoded to charset/endianness-agnostic
/// [`Native`](Stored::Native) values so `dest` re-encodes them on write.
pub(crate) fn adapt_dataset(dest: &Shared, ds: DataSet) -> Result<Item> {
    let (src, root) = ds.into_parts();
    adapt_item(dest, &src, root)
}

fn adapt_item(dest: &Shared, src: &Shared, item: Item) -> Result<Item> {
    let entries = item.map.into_entries();
    let mut map = ElementMap::with_capacity(entries.len());
    for (key, el) in entries {
        let value = adapt_stored(dest, src, el.vr, el.value)?;
        // Source order is preserved as-is, so just append.
        map.push_parsed(key, Element::new(el.vr, value));
    }
    Ok(Item::from_map(map))
}

fn adapt_stored(dest: &Shared, src: &Shared, vr: Vr, value: Stored) -> Result<Stored> {
    match value {
        // ponytail: tz conversion of naive Native DT and DA/TM pairs across a
        // differing default timezone is deferred (its own algorithm).
        Stored::Native(v) => Ok(Stored::Native(v)),
        Stored::Items(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(adapt_item(dest, src, it)?);
            }
            Ok(Stored::Items(out))
        }
        Stored::Owned(b) => adapt_raw(dest, src, vr, b),
        Stored::Mapped(range) => {
            let bytes = src
                .master()
                .get(range)
                .ok_or_else(|| dicom_err!(Internal, "mapped value range out of bounds"))?;
            adapt_raw(dest, src, vr, Bytes::copy_from_slice(bytes))
        }
    }
}

fn adapt_raw(dest: &Shared, src: &Shared, vr: Vr, bytes: Bytes) -> Result<Stored> {
    if dest.is_little_endian() == src.is_little_endian()
        && dest.charset().specific_character_set() == src.charset().specific_character_set()
    {
        // Same context: the raw bytes are already valid for `dest`.
        Ok(Stored::Owned(bytes))
    } else {
        // Differing context: decode under the source context to a logical value;
        // `dest` will re-encode it in its own charset/byte order on write.
        Ok(Stored::Native(convert::decode(src, vr, &bytes)?))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use dpx_dicom_core::{Vr, tags};

    use crate::dataset::DatasetKind;
    use crate::value::{Element, Stored};
    use crate::DataSet;
    use dpx_dicom_core::TransferSyntax;

    #[test]
    fn push_big_endian_item_adapts_byte_order() {
        // Source data set is Big Endian and carries its US value as raw bytes
        // (as if parsed from a BE file): 258 = 0x0102, big-endian = [0x01, 0x02].
        let mut src = DataSet::parsed(Bytes::new(), &TransferSyntax::ExplicitVRBigEndian, DatasetKind::Dataset);
        src.root_mut().map.insert(
            tags::Rows.key,
            Element::new(Vr::US, Stored::Owned(Bytes::from_static(&[0x01, 0x02]))),
        );
        assert_eq!(src.get::<u16>(&tags::Rows).expect("src reads BE"), 258);

        // Destination is Little Endian; pushing the BE item must adapt it.
        let mut dest = DataSet::new();
        assert!(dest.is_little_endian());
        {
            let mut seq = dest.sequence_mut(&tags::ReferencedSeriesSequence).expect("seq");
            seq.push(src).expect("push BE item");
        }
        let seq = dest.sequence(&tags::ReferencedSeriesSequence).expect("read seq");
        let item = seq.item(0).expect("item 0");
        assert_eq!(item.get::<u16>(&tags::Rows).expect("nested reads after adapt"), 258);
    }
}
