use super::*;
use crate::{Context, dicom_err, ensure, tag, utils::unescape::unescape};
use std::borrow::Cow;

// cSpell:ignore xxee Тест

/// An identifier of the Attribute in DICOM world.
///
/// This identifier consists of a `group` number, `element` number and optional
/// "private creator" string, which unique identifies the private vendored
/// attributes. Some explanation given in [PS3.6 "5 Conventions"].
///
/// This leads to a two major categories:
/// - Standard attributes
/// - Private attributes
///
/// ### Standard attributes
/// These attributes are specified in the DICOM Standard in [PS3.6 "6 Registry
/// of DICOM Data Elements"], [PS3.6 "7 Registry of DICOM File Meta Elements"],
/// [PS3.6 "8 Registry of DICOM Directory Structuring Elements"], [PS3.6
/// "Registry of DICOM Dynamic RTP Payload Elements"], [PS3.7 "E.1 Registry of
/// DICOM Command Elements"] and [PS3.7 "E.2 Retired Command Fields"]. Some of
/// these attributes has a mask `XX` in their definition `(ggXX,eeee)`. This
/// means that the attribute in a dataset may have any hex digits at the
/// position denoted by `XX`. For example, the Tag `Overlay Data (60xx,3000)`
/// can be written in dataset as (6000,3000), (6001,3000), ..., (60FF,3000).
///
/// ### Private attributes
/// These attributes are vendor-specific, so simple group/element matching will
/// lead to inevitable collisions. Standard defines a way for coexistence of
/// such attributes in a single dataset without colliding. You can learn this in
/// details in [PS3.5 "7.8.1 Private Data Element Tags"]. In short:
/// - Vendor can define any number of element groups.
/// - For each group vendor chooses:
///   - An odd group number
///   - A unique designator for this group. This will be called "Private
///     Creator".
///   - There may be up to 256 elements in one group.
///   - This will end up with tag `(gggg,xxee)`, where `gggg` and `ee` - numbers
///     specified by the vendor, `xx` - dynamic number from 0x10 to 0xFF.
/// - For each group of the private attributes recorded in the dataset, there
///   must be one "Private Reservation" attribute with the tag `(gggg,00xx)`,
///   where `gggg` is a number of this group; `xx` - is a dynamic number from
///   0x10 to 0xFF. This dynamic number was chosen by the entity that wrote this
///   attribute. The rules for dynamic number are simple: find some `xx`, with a
///   value of your "Private Creator". IF not found, find non-existent `xx`.
///   This standard algorithm limits the number of different vendors with
///   conflicting groups in the same data set to 240.
///
/// Note for the application developers: When creating own dictionary of private
/// attributes, one can use arbitrary number for `xx` part of the attribute.
/// This particular number will be used when library reads a dataset with absent
/// "Private Reservation", that was created by a non-conforming/buggy software.
/// And, also when library writes attribute with such tag, it will preferably
/// use this number if the dataset allows it.\
/// When backward compatibility is not a concern, the recommended choice is "0x10",
/// so attribute tag in a dictionary will end up as `(gggg,ee10)`.
///
/// [PS3.6 "5 Conventions"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_5.html
/// [PS3.6 "6 Registry of DICOM Data Elements"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_6.html#table_6-1
/// [PS3.6 "7 Registry of DICOM File Meta Elements"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_7.html#table_7-1
/// [PS3.6 "8 Registry of DICOM Directory Structuring Elements"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_8.html#table_8-1
/// [PS3.6 "9 Registry of DICOM Dynamic RTP Payload Elements"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part06/chapter_9.html#table_9-1
/// [PS3.7 "E.1 Registry of DICOM Command Elements"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part07/chapter_E.html
/// [PS3.7 "E.2 Retired Command Fields"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part07/sect_E.2.html
/// [PS3.5 "7.8.1 Private Data Element Tags"]:
///     https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_7.8.html
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tag {
    pub key: TagKey,
    pub creator: Option<Cow<'static, str>>,
}

impl Tag {
    /// Construct a new tag with a specified `TagKey` and optional "Private Creator"
    ///
    /// You better take one of the constantly known tags, than constructing your own.
    pub const fn new(key: TagKey, creator: Option<Cow<'static, str>>) -> Self {
        Self { key, creator }
    }

    /// Construct a Standard Attribute Tag with a specified `TagKey`
    ///
    /// You better take one of the constantly known tags, than constructing your own.
    pub const fn new_standard(g: u16, e: u16) -> Self {
        Self {
            key: TagKey::new(g, e),
            creator: None,
        }
    }

    /// Construct a Private Attribute Tag with a specified `TagKey` and "Private Creator"
    ///
    /// It is recommended to make a global constant for each of the private
    /// attribute supported and register it in the dictionary rather
    /// constructing a Tag on each use.
    ///
    /// ### Panics:
    /// This method panic in in non-optimized builds with '-C debug-assertions`
    /// if the key is not private (see [is_private](TagKey)).
    pub const fn new_private(g: u16, e: u16, creator: &'static str) -> Self {
        let key = TagKey::new(g, e);
        debug_assert!(key.is_private());
        Self {
            key,
            creator: match creator.len() {
                0 => None,
                _ => Some(Cow::Borrowed(creator)),
            },
        }
    }

    /// Construct a Private Attribute Tag with a specified `TagKey` and "Private Creator"
    ///
    /// It is recommended to make a global constant for each of the private
    /// attribute supported and register it in the dictionary rather
    /// constructing a Tag on each use.
    ///
    /// ### Panics:
    /// This method panic in in non-optimized builds with '-C debug-assertions`
    /// if the key is not private (see [is_private](TagKey)).
    pub fn new_private_cow<T: Into<String>>(g: u16, e: u16, creator: T) -> Self {
        let key = TagKey::new(g, e);
        debug_assert!(key.is_private());
        Self {
            key,
            creator: Some(Cow::Owned(creator.into())),
        }
    }

    /// Ensures the `creator` field owns its string, allocating if it was a
    /// borrowed (`'static`) string literal.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// # use ::std::borrow::Cow;
    /// // A tag built from a string literal borrows it without allocation
    /// let borrowed = Tag::new_private(0x4321, 0x1000, "Test");
    /// assert!(matches!(borrowed.creator, Some(Cow::Borrowed(_))));
    ///
    /// // Conversion into owned allocates the string on the heap
    /// let owned = borrowed.to_owned();
    /// assert!(matches!(owned.creator, Some(Cow::Owned(_))));
    /// ```
    pub fn to_owned(self) -> Tag {
        Tag {
            key: self.key,
            creator: self.creator.map(|v| Cow::Owned(v.into_owned())),
        }
    }

    /// Searches tag information in the current [Context]
    ///
    /// See also [search_by_tag](crate::tag::Dictionary::search_by_tag)
    pub fn meta(&self) -> Option<tag::Meta> {
        Context::with_current(|ctx| ctx.tag_dict().search_by_tag(self).cloned())
    }

    /// Searches and returns Tag name in the current [Context]
    ///
    /// See also [search_by_tag](crate::tag::Dictionary::search_by_tag)
    pub fn name(&self) -> Option<String> {
        Context::with_current(|ctx| ctx.tag_dict().search_by_tag(self).map(|m| m.name.to_string()))
    }
}

impl std::fmt::Display for Tag {
    /// Outputs this key in format `(gggg,eeee[,"creator"])`, where `gggg` and
    /// `eeee` are the group and element numbers in upper hexadecimal digits,
    /// `creator` - private creator string if present.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// let tag = Tag::new_private(0x4321, 0x10AA, "Test");
    /// assert_eq!(tag.to_string(), "(4321,10AA,\"Test\")");
    /// assert_eq!(format!("{tag}"), "(4321,10AA,\"Test\")");
    /// assert_eq!(format!("{}", Tag::new_standard(0x0008, 0x0005)), "(0008,0005)");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = &self.creator {
            write!(
                f,
                "({:04X},{:04X},\"{}\")",
                self.key.group(),
                self.key.element(),
                v.escape_default()
            )
        } else {
            write!(f, "({:04X},{:04X})", self.key.group(), self.key.element())
        }
    }
}

impl From<Tag> for String {
    fn from(value: Tag) -> Self {
        format!("{value}")
    }
}

impl std::fmt::Debug for Tag {
    /// Outputs this key in format `Tag(gggg,eeee[,"creator"])`, where `gggg`
    /// and `eeee` are the group and element numbers in upper hexadecimal
    /// digits, `creator` - private creator string if present (escaped using
    /// [str::escape_default]).
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// let tag = Tag::new_private(0x4321, 0x10AA, "Test");
    /// assert_eq!(format!("{tag:?}"), "Tag(4321,10AA,\"Test\")");
    /// let tag = Tag::new_standard(0x0008, 0x0005);
    /// assert_eq!(format!("{tag:?}"), "Tag(0008,0005)");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = &self.creator {
            write!(
                f,
                "Tag({:04X},{:04X},\"{}\")",
                self.key.group(),
                self.key.element(),
                v.escape_default()
            )
        } else {
            write!(f, "Tag({:04X},{:04X})", self.key.group(), self.key.element())
        }
    }
}

impl PartialEq<TagKey> for Tag {
    fn eq(&self, other: &TagKey) -> bool {
        self.key.eq(other)
    }
}

impl PartialEq<u32> for Tag {
    fn eq(&self, other: &u32) -> bool {
        self.key.0.eq(other)
    }
}

impl PartialOrd<TagKey> for Tag {
    fn partial_cmp(&self, other: &TagKey) -> Option<std::cmp::Ordering> {
        self.key.partial_cmp(other)
    }
}

impl PartialOrd<u32> for Tag {
    fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
        self.key.0.partial_cmp(other)
    }
}

impl ::core::str::FromStr for Tag {
    type Err = DicomError;
    /// Parses a text representation of the Tag
    ///
    /// Allowed formats:
    /// - `(gggg,eeee)`
    /// - `(gggg,eeee,"creator")`
    ///
    /// Where `gggg` and `eeee` - hexadecimal group and element numbers,
    /// `creator` - private creator with special characters escaped in C-like
    /// escapes (see [std::ascii::escape_default])
    ///
    /// Examples:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, DicomError};
    /// # use ::core::str::FromStr;
    /// # fn main() -> Result<(), DicomError> {
    /// assert_eq!(Tag::from_str("(0008,0005)")?,
    ///     Tag::new_standard(0x0008, 0x0005));
    /// assert_eq!(Tag::from_str(r#"(4321,AA10,"Test")"#)?,
    ///     Tag::new_private(0x4321, 0xAA10, "Test"));
    /// assert_eq!(Tag::from_str(r#"(4321,AA10,"💖\tТест")"#)?,
    ///     Tag::new_private(0x4321, 0xAA10, "💖\tТест"));
    ///
    /// let key: Tag = "(0008,0005)".parse()?;
    /// assert_eq!(key, Tag::new_standard(0x0008, 0x0005));
    ///
    /// assert!(Tag::from_str("OOPS").is_err());
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tag::try_from(s)
    }
}

impl TryFrom<&str> for Tag {
    type Error = DicomError;
    /// See trait `FromStr::from_str` implementation for this struct
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        ensure!(
            s.starts_with('('),
            InvalidData,
            "missing opening brace for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"
        );
        ensure!(
            s.ends_with(')'),
            InvalidData,
            "missing closing brace for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"
        );

        let mut components = s[1..s.len() - 1].splitn(3, ',');

        let group_chars = components.next().ok_or_else(|| {
            dicom_err!(
                InvalidData,
                "not enough components for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"
            )
        })?;
        let element_chars = components.next().ok_or_else(|| {
            dicom_err!(
                InvalidData,
                "not enough components for Tag (expecting: `(gggg,eeee[,\"creator\"])`)"
            )
        })?;

        let group = u16::from_str_radix(group_chars, 16)
            .map_err(|e| dicom_err!(InvalidData, "unable to parse hex in Tag: {e:?}"))?;

        let element = u16::from_str_radix(element_chars, 16)
            .map_err(|e| dicom_err!(InvalidData, "unable to parse hex in Tag: {e:?}"))?;

        let creator: Option<Cow<'static, str>> = match components.next() {
            None => None,
            Some(creator) => {
                ensure!(
                    creator.starts_with('"'),
                    InvalidData,
                    "missing opening quote in Tag creator (expecting: `(gggg,eeee[,\"creator\"])`)"
                );
                ensure!(
                    creator[1..].ends_with('"'),
                    InvalidData,
                    "missing closing quote in Tag creator (expecting: `(gggg,eeee[,\"creator\"])`)"
                );
                let creator = &creator[1..creator.len() - 1];

                match creator.len() {
                    0 => None,
                    _ => {
                        if !creator.contains('\\') {
                            Some(Cow::Owned(creator.to_owned()))
                        } else {
                            Some(Cow::Owned(unescape(creator).map_err(|e| {
                                dicom_err!(InvalidData, "unable to parse Tag creator: {e}")
                            })?))
                        }
                    }
                }
            }
        };

        Ok(Self::new(TagKey::new(group, element), creator))
    }
}

impl TryFrom<String> for Tag {
    type Error = DicomError;
    /// See trait `FromStr::from_str` implementation for this struct
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Tag::try_from(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arc, Context};

    #[rustfmt::skip]
    #[test]
    fn can_decode() {
        use ::core::str::FromStr;

        assert_eq!(Tag::from_str("(0008,0005)").unwrap(),
            Tag::new_standard(0x0008, 0x0005));

        assert_eq!(Tag::from_str(r#"(4321,AA10,"Test")"#).unwrap(),
            Tag::new_private(0x4321, 0xAA10, "Test"));
        assert_eq!(Tag::from_str(r#"(4321,AA10,"Test")"#).unwrap(),
            Tag::new_private(0x4321, 0xAA10, "Test"));
        assert_eq!(Tag::from_str(r#"(4321,AA10,"")"#).unwrap(),
            Tag::new_private(0x4321, 0xAA10, ""));

        let key: Tag = "(0008,0005)".parse().unwrap();
        assert_eq!(key, Tag::new_standard(0x0008, 0x0005));

        // Try all the errors
        assert!(Tag::from_str("").is_err());
        assert!(Tag::from_str("0008,0005)").is_err());
        assert!(Tag::from_str("(0008,0005").is_err());
        assert!(Tag::from_str("(00080005)").is_err());
        assert!(Tag::from_str("(000Z,0005)").is_err());
        assert!(Tag::from_str("(0008,000Z)").is_err());
        assert!(Tag::from_str("(0008,0005,)").is_err());
        assert!(Tag::from_str("(0008,0005,Test)").is_err());
        assert!(Tag::from_str(r#"(0008,0005,")"#).is_err());
        assert!(Tag::from_str(r#"(0008,0005,"\uZ")"#).is_err());
    }

    #[test]
    fn can_retrieve_meta() {
        // Installs into the process-global context; serialize against every
        // other test that swaps global state.
        let _guard = crate::config::subst::lock_global_for_test();
        crate::declare_tags! {
            const TAGS = [
                TestTag: { (0x4321, 0x10AA, "test"), AE, 1, "Test Tag", Vendored(None) },
            ];
        }

        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&TAGS);
        let dict = Arc::new(dict);

        Context::extend().tag_dict(Arc::clone(&dict)).provide(|| {
            assert_eq!(TestTag.meta().unwrap().name, "Test Tag");
            assert_eq!(TestTag.name().unwrap(), "Test Tag");
        });

        assert_eq!(TestTag.meta().unwrap().name, "Unknown");

        let prev_global = Context::extend().tag_dict(Arc::clone(&dict)).install_global();
        assert_eq!(TestTag.meta().unwrap().name, "Test Tag");

        Context::global().store(prev_global);
        assert_eq!(TestTag.meta().unwrap().name, "Unknown");
    }
}
