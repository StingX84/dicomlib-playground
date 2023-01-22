use super::*;
use crate::{utils::unescape::unescape, Cow, tag, State};

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
///   - There may be ap to 256 elements in one group.
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
/// use this number if dataset if allowed.\
/// When no backward compatibility reasons involved, one better choose "0x10",
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
    pub const fn standard(g: u16, e: u16) -> Self {
        Self { key: TagKey::new(g, e), creator: None }
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
    pub const fn private(g: u16, e: u16, creator: &'a str) -> Self {
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

    /// Converts this tag into a "owned". one possibly allocating a memory for
    /// `creator` field.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// # use ::std::borrow::Cow;
    /// let creator = String::from("Test");
    /// // This method "borrows" creator and does not allocate
    /// let borrowed = Tag::private(0x4321, 0x1000, creator.as_ref());
    /// assert!(matches!(borrowed.creator, Some(Cow::Borrowed(_))));
    ///
    /// // Cloning borrowed produces also borrowed without allocation
    /// let borrowed_clone = borrowed.clone();
    /// assert!(matches!(borrowed_clone.creator, Some(Cow::Borrowed(_))));
    ///
    /// // Conversion into borrowed allocates string internally
    /// let owned = borrowed_clone.to_owned();
    /// assert!(matches!(owned.creator, Some(Cow::Owned(_))));
    ///
    /// drop(creator);
    /// // Next line would not compile
    /// //println!("{}", borrowed);
    /// println!("{}", owned);
    /// ```
    pub fn to_owned(self) -> Tag<'static> {
        Tag::<'static> {
            key: self.key,
            creator: self.creator.map(|v| Cow::Owned(v.into_owned())),
        }
    }

    /// Searches tag information in the current [State](crate::State)
    ///
    /// See also [search_by_tag](crate::tag::Dictionary::search_by_tag)
    pub fn meta(&self) -> Option<tag::Meta> {
        State::with_current(|s| s.tag_dictionary().search_by_tag(self).cloned())
    }

    /// Searches and returns Tag name in the current [State](crate::State)
    ///
    /// See also [search_by_tag](crate::tag::Dictionary::search_by_tag)
    pub fn name(&self) -> Option<String> {
        State::with_current(|s| {
            s.tag_dictionary()
                .search_by_tag(self)
                .map(|m| m.name.to_string())
        })
    }
}

impl<'a> std::fmt::Display for Tag<'a> {
    /// Outputs this key in format `(gggg,eeee[,"creator"])`, where `gggg` and
    /// `eeee` are the group and element numbers in upper hexadecimal digits,
    /// `creator` - private creator string if present.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// let tag = Tag::private(0x4321, 0x10AA, "Test");
    /// assert_eq!(tag.to_string(), "(4321,10AA,\"Test\")");
    /// assert_eq!(format!("{tag}"), "(4321,10AA,\"Test\")");
    /// assert_eq!(format!("{}", Tag::standard(0x0008, 0x0005)), "(0008,0005)");
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

impl<'a> From<Tag<'a>> for String {
    fn from(value: Tag<'a>) -> Self {
        format!("{value}")
    }
}

impl<'a> std::fmt::Debug for Tag<'a> {
    /// Outputs this key in format `Tag(gggg,eeee[,"creator"])`, where `gggg`
    /// and `eeee` are the group and element numbers in upper hexadecimal
    /// digits, `creator` - private creator string if present (escaped using
    /// [str::escape_default]).
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::{Tag, TagKey};
    /// let tag = Tag::private(0x4321, 0x10AA, "Test");
    /// assert_eq!(format!("{tag:?}"), "Tag(4321,10AA,\"Test\")");
    /// let tag = Tag::standard(0x0008, 0x0005);
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
            write!(
                f,
                "Tag({:04X},{:04X})",
                self.key.group(),
                self.key.element()
            )
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

impl<'a> ::core::str::FromStr for Tag<'a> {
    type Err = Error;
    /// Parses a text representation of the Tag
    ///
    /// Allowed formats:
    /// - `(gggg,eeee)`
    /// - `(gggg,eeee,"creator")`
    ///
    /// Where `gggg` and `eeee` - hexadecimal group and element numbers,
    /// `creator` - private creator with special characters escaped in C-like
    /// escapes (see )
    ///
    /// Examples:
    /// ```
    /// # use ::dpx_dicom_core::{tag::Error, Tag, tag};
    /// # use ::core::str::FromStr;
    /// # fn main() -> Result<(), Error> {
    /// assert_eq!(Tag::from_str("(0008,0005)")?,
    ///     Tag::standard(0x0008, 0x0005));
    /// assert_eq!(Tag::from_str(r#"(4321,AA10,"Test")"#)?,
    ///     Tag::private(0x4321, 0xAA10, "Test"));
    /// assert_eq!(Tag::from_str(r#"(4321,AA10,"💖\tТест")"#)?,
    ///     Tag::private(0x4321, 0xAA10, "💖\tТест"));
    ///
    /// let key: Tag = "(0008,0005)".parse()?;
    /// assert_eq!(key, Tag::standard(0x0008, 0x0005));
    ///
    /// assert!(matches!(Tag::from_str("OOPS"), Err(tag::Error::TagMissingOpeningBrace)));
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Tag::try_from(s)?.to_owned())
    }
}

impl<'a> TryFrom<&'a str> for Tag<'a> {
    type Error = Error;
    /// See trait `FromStr::from_str` implementation for this struct
    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        ensure!(s.starts_with('('), TagMissingOpeningBraceSnafu);
        ensure!(s.ends_with(')'), TagMissingClosingBraceSnafu);

        let mut components = s[1..s.len() - 1].splitn(3, ',');

        let group_chars = components.next().context(TagMissingComponentsSnafu)?;
        let element_chars = components.next().context(TagMissingComponentsSnafu)?;

        let group =
            u16::from_str_radix(group_chars, 16).context(TagContainsNonHexCharactersSnafu)?;

        let element =
            u16::from_str_radix(element_chars, 16).context(TagContainsNonHexCharactersSnafu)?;

        let creator: Option<Cow<'a, str>> = match components.next() {
            None => None,
            Some(creator) => {
                ensure!(creator.starts_with('"'), TagMissingCreatorOpeningQuoteSnafu);
                ensure!(
                    creator[1..].ends_with('"'),
                    TagMissingCreatorClosingQuoteSnafu
                );
                let creator = &creator[1..creator.len() - 1];

                match creator.len() {
                    0 => None,
                    _ => {
                        if !creator.contains('\\') {
                            Some(Cow::Borrowed(creator))
                        } else {
                            Some(Cow::Owned(unescape(creator).map_err(|e| {
                                Error::TagInvalidCreatorString {
                                    message: e.to_string(),
                                }
                            })?))
                        }
                    }
                }
            }
        };

        Ok(Self::new(TagKey::new(group, element), creator))
    }
}

impl TryFrom<String> for Tag<'static> {
    type Error = Error;
    /// See trait `FromStr::from_str` implementation for this struct
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Tag::try_from(value.as_str())?.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::State, state::StateBuilder};

    #[rustfmt::skip]
    #[test]
    fn can_decode() {
        use ::core::str::FromStr;

        assert_eq!(Tag::from_str("(0008,0005)").unwrap(),
            Tag::standard(0x0008, 0x0005));

        assert_eq!(Tag::from_str(r#"(4321,AA10,"Test")"#).unwrap(),
            Tag::private(0x4321, 0xAA10, "Test"));
        assert_eq!(Tag::from_str(r#"(4321,AA10,"Test")"#).unwrap(),
            Tag::private(0x4321, 0xAA10, "Test"));
        assert_eq!(Tag::from_str(r#"(4321,AA10,"")"#).unwrap(),
            Tag::private(0x4321, 0xAA10, ""));

        let key: Tag = "(0008,0005)".parse().unwrap();
        assert_eq!(key, Tag::standard(0x0008, 0x0005));

        // Try all the errors
        use Error::*;
        assert!(matches!(Tag::from_str(""), Err(TagMissingOpeningBrace)));
        assert!(matches!(Tag::from_str("0008,0005)"), Err(TagMissingOpeningBrace)));
        assert!(matches!(Tag::from_str("(0008,0005"), Err(TagMissingClosingBrace)));
        assert!(matches!(Tag::from_str("(00080005)"), Err(TagMissingComponents)));
        assert!(matches!(Tag::from_str("(000Z,0005)"), Err(TagContainsNonHexCharacters{source: _})));
        assert!(matches!(Tag::from_str("(0008,000Z)"), Err(TagContainsNonHexCharacters{source: _})));
        assert!(matches!(Tag::from_str("(0008,0005,)"), Err(TagMissingCreatorOpeningQuote)));
        assert!(matches!(Tag::from_str("(0008,0005,Test)"), Err(TagMissingCreatorOpeningQuote)));
        assert!(matches!(Tag::from_str(r#"(0008,0005,")"#), Err(TagMissingCreatorClosingQuote)));
        assert!(matches!(Tag::from_str(r#"(0008,0005,"\uZ")"#), Err(TagInvalidCreatorString{message: _})));
    }

    #[test]
    fn can_retrieve_meta() {
        crate::declare_tags! {
            const TAGS = [
                TestTag: { (0x4321, 0x10AA, "test"), AE, 1, "Test Tag", Vendored(None) },
            ];
        }

        let mut dict = Dictionary::new_empty();
        dict.add_static_list(&TAGS);

        let state = StateBuilder::new()
            .with_tag_dictionary(dict.clone())
            .build();

        state.provide_current_for(|| {
            // Test tag now may be found once we push dictionary to thread local
            assert_eq!(TestTag.meta().unwrap().name, "Test Tag");
            assert_eq!(TestTag.name().unwrap(), "Test Tag");
        });

        // Test tag should not be found again, because outside of
        // "provide_current_for" scope
        assert_eq!(TestTag.meta().unwrap().name, "Unknown");

        state.into_global();

        // Test tag should be found now, because we make our state global
        assert_eq!(TestTag.meta().unwrap().name, "Test Tag");

        // Restore default globaL state
        State::default().into_global();

        // Test tag should not present in the default global state
        assert_eq!(TestTag.meta().unwrap().name, "Unknown");
    }
}
