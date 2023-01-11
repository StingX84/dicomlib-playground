use super::TagKey;
use crate::Cow;

// cSpell:ignore xxee

/// An identifier of the Attribute in DICOM world.
///
/// This identifier consists of a `group` number, `element` number and
/// optional "private creator" string, which unique identifies the
/// private vendored attributes. Some explanation given in [PS3.6 "5 Conventions"].
///
/// This leads to a two major categories:
/// - Standard attributes
/// - Private attributes
///
/// ### Standard attributes
/// These attributes are specified in the DICOM Standard in [PS3.6 "6 Registry of DICOM Data Elements"],
/// [PS3.6 "7 Registry of DICOM File Meta Elements"], [PS3.6 "8 Registry of DICOM Directory Structuring Elements"],
/// [PS3.6 "Registry of DICOM Dynamic RTP Payload Elements"], [PS3.7 "E.1 Registry of DICOM Command Elements"] and
/// [PS3.7 "E.2 Retired Command Fields"].
/// Some of these attributes has a mask `XX` in their definition `(ggXX,eeee)`. This means that
/// the attribute in a dataset may have any hex digits at the position denoted by `XX`. For example,
/// the Tag `Overlay Data (60xx,3000)` can be written in dataset as (6000,3000), (6001,3000), ...,
/// (60FF,3000).
///
/// ### Private attributes
/// These attributes are vendor-specific, so simple group/element matching will lead to inevitable collisions.
/// Standard defines a way for coexistence of such attributes in a single dataset without colliding. You
/// can learn this in details in [PS3.5 "7.8.1 Private Data Element Tags"]. In short:
/// - Vendor can define any number of element groups.
/// - For each group vendor chooses:
///   - An odd group number
///   - A unique designator for this group. This will be called "Private Creator".
///   - There may be ap to 256 elements in one group.
///   - This will end up with tag `(gggg,xxee)`, where `gggg` and `ee` - numbers specified by the vendor, `xx` - dynamic number from 0x10 to 0xFF.
/// - For each group of the private attributes recorded in the dataset, there must be one "Private Reservation"
///   attribute with the tag `(gggg,00xx)`, where `gggg` is a number of this group; `xx` - is a dynamic number
///   from 0x10 to 0xFF. This dynamic number was chosen by the entity that wrote this attribute. The rules
///   for dynamic number are simple: find some `xx`, with a value of your "Private Creator". IF not found,
///   find non-existent `xx`. This standard algorithm limits the number of different vendors with conflicting
///   groups in the same data set to 240.
///
/// Note for the application developers: When creating own dictionary of private attributes, one can use
/// arbitrary number for `xx` part of the attribute. This particular number will be used when library
/// reads a dataset with absent "Private Reservation", that was created by a non-conforming/buggy software.
/// And, also when library writes attribute with such tag, it will preferably use this number if dataset
/// if allowed.\
/// When no backward compatibility reasons involved, one better choose "0x10", so attribute tag in a
/// dictionary will end up as `(gggg,ee10)`.
///
/// [PS3.6 "5 Conventions"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_5.html
/// [PS3.6 "6 Registry of DICOM Data Elements"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_6.html#table_6-1
/// [PS3.6 "7 Registry of DICOM File Meta Elements"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_7.html#table_7-1
/// [PS3.6 "8 Registry of DICOM Directory Structuring Elements"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_8.html#table_8-1
/// [PS3.6 "9 Registry of DICOM Dynamic RTP Payload Elements"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_9.html#table_9-1
/// [PS3.7 "E.1 Registry of DICOM Command Elements"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part07/chapter_E.html
/// [PS3.7 "E.2 Retired Command Fields"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part07/sect_E.2.html
/// [PS3.5 "7.8.1 Private Data Element Tags"]: https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_7.8.html
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tag<'a> {
    pub key: TagKey,
    pub creator: Option<Cow<'a, str>>,
}

impl<'a> Tag<'a> {
    /// Construct a new tag with a specified `TagKey` and optional "Private Creator"
    ///
    /// You better take one of the constantly known tags, than constructing your own.
    pub const fn new(key: TagKey, creator: Option<Cow<'a, str>>) -> Self {
        Self { key, creator }
    }

    /// Construct a Standard Attribute Tag with a specified `TagKey`
    ///
    /// You better take one of the constantly known tags, than constructing your own.
    ///
    /// ### Panics:
    /// This method panic in in non-optimized builds with '-C debug-assertions`
    /// if the key is not valid (see [is_valid](TagKey)) or is private (see [is_private](TagKey)).
    pub const fn standard(g: u16, e: u16) -> Self {
        let key = TagKey::new(g, e);
        debug_assert!(key.is_valid());
        debug_assert!(!key.is_private());
        Self { key, creator: None }
    }

    /// Construct a Private Attribute Tag with a specified `TagKey` and "Private Creator"
    ///
    /// It is recommended to make a global constant for each of the private attribute
    /// supported and register it in the dictionary rather constructing a Tag
    /// on each use.
    ///
    /// ### Panics:
    /// This method panic in in non-optimized builds with '-C debug-assertions`
    /// if the key is not valid (see [is_valid](TagKey)) or is not private (see [is_private](TagKey)).
    pub const fn private(g: u16, e: u16, creator: &'a str) -> Self {
        let key = TagKey::new(g, e);
        debug_assert!(key.is_valid());
        debug_assert!(key.is_private());
        Self { key, creator: Some(Cow::Borrowed(creator)) }
    }
}

impl<'a> std::fmt::Display for Tag<'a> {
    /// Outputs this key in format `(gggg,eeee[,"creator"])`, where `gggg` and `eeee` is group and element numbers in hexadecimal form,
    /// `creator` - private creator string if present.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// let tag = Tag::private(0x4321, 0x5678, "Test");
    /// assert_eq!(tag.to_string(), "(4321,5678,\"Test\")");
    /// assert_eq!(format!("{tag}"), "(4321,5678,\"Test\")");
    /// assert_eq!(format!("{}", Tag::standard(0x1234, 0x5678)), "(1234,5678)");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = &self.creator {
            write!(f, "({:04x},{:04x},\"{}\")", self.key.group(), self.key.element(), v.escape_default())
        } else {
            write!(f, "({:04x},{:04x})", self.key.group(), self.key.element())
        }
    }
}

impl<'a> From<Tag<'a>> for String {
    fn from(value: Tag<'a>) -> Self {
        format!("{value}")
    }
}

impl<'a> std::fmt::Debug for Tag<'a> {
    /// Outputs this key in format `Tag(gggg,eeee[,"creator"])`, where `gggg` and `eeee` is group and element numbers in hexadecimal form,
    /// `creator` - private creator string if present.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// let tag = Tag::private(0x4321, 0x5678, "Test");
    /// assert_eq!(format!("{tag:?}"), "Tag(4321,5678,\"Test\")");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = &self.creator {
            write!(f, "Tag({:04x},{:04x},\"{}\")", self.key.group(), self.key.element(), v.escape_default())
        } else {
            write!(f, "Tag({:04x},{:04x})", self.key.group(), self.key.element())
        }
    }
}

impl<'a> PartialEq<TagKey> for Tag<'a> {
    fn eq(&self, other: &TagKey) -> bool {
        self.key.eq(other)
    }
}

impl<'a> PartialEq<u32> for Tag<'a> {
    fn eq(&self, other: &u32) -> bool {
        self.key.0.eq(other)
    }
}

impl<'a> PartialOrd<TagKey> for Tag<'a> {
    fn partial_cmp(&self, other: &TagKey) -> Option<std::cmp::Ordering> {
        self.key.partial_cmp(other)
    }
}

impl<'a> PartialOrd<u32> for Tag<'a> {
    fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
        self.key.0.partial_cmp(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

    #[test]
    fn struct_methods() {
        let k = Tag::standard(0x1234, 0x5678);
        // string transformations
        assert_eq!(k.to_string(), "(1234,5678)");
        assert_eq!(format!("{k}"), "(1234,5678)");
        assert_eq!(format!("{k:?}"), "Tag(1234,5678)");

        let k = Tag::new(TagKey::new(0x1234, 0x5678), Some(Cow::Borrowed("Test")));
        assert_eq!(k.to_string(), "(1234,5678,\"Test\")");
        assert_eq!(format!("{k}"), "(1234,5678,\"Test\")");
        assert_eq!(format!("{k:?}"), "Tag(1234,5678,\"Test\")");
    }
}
