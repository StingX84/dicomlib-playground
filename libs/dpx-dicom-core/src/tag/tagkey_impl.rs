use super::*;
use crate::Cow;

// cSpell:ignore xxee

/// An identifier of an Element in a Dataset
///
/// It consists of two 16-bit unsigned integers called "group" and "element".
/// Particular meaning of these components is not important for most of the
/// applications.
///
/// This identifier is somewhat low-level access to DICOM files, that
/// exists for the internal performance purposes. It is expected that
/// applications are rather access the dataset attributes by `dpx_dicom_core::Tag` instead.
///
/// This is because of complexity of handling Private Attributes and their
/// associated Private Reservations. See details in `dpx_dicom_core::Tag` documentation.
///
/// ### Additional constructors:
/// This struct adopts [`From`] trait and can be converted to and from `u32` and `(u32, u32)`:
/// ```
/// # use ::dpx_dicom_core::TagKey;
/// assert_eq!(TagKey::from(0x12345678), TagKey::new(0x1234, 0x5678));
/// assert_eq!(TagKey::from((0x1234, 0x5678)), TagKey::new(0x1234, 0x5678));
/// assert_eq!(0x12345678u32, TagKey::new(0x1234, 0x5678).into());
/// assert_eq!((0x1234, 0x5678), TagKey::new(0x1234, 0x5678).into());
/// ```
/// See other examples in [std::fmt::Display](#method.fmt) and [std::str::FromStr](#method.from_str) trait implementations.
///
/// ### Serde support:
/// If "serde" feature is set, this struct serializes and deserializes as a simple String
/// in format `(gggg,eeee)`, where `gggg` and `eeee` is group and element numbers in hexadecimal form.\
/// The same format used in [std::fmt::Display](#method.fmt) and [std::str::FromStr](#method.from_str) trait implementations.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(try_from = "&str", into = "String"))]
#[repr(transparent)]
pub struct TagKey(
    pub u32
);

impl TagKey {
    /// Creates a key from group and element
    #[inline]
    pub const fn new(g: u16, e: u16) -> Self {
        Self(((g as u32) << 16) | (e as u32))
    }

    /// Creates a key from 32-bit unsigned. Group number in high word, element number in low word
    #[inline]
    pub const fn from_u32(v: u32) -> Self {
        Self(v)
    }

    /// Returns a group number of this key
    #[inline]
    pub const fn group(&self) -> u16 {
        (self.0 >> 16) as u16
    }

    /// Returns an element number of this key
    #[inline]
    pub const fn element(&self) -> u16 {
        (self.0 & 0xFFFFu32) as u16
    }

    /// Returns a group and element number as unsigned 32-bits integer
    /// This is same as:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// assert_eq!(TagKey::new(0x1234, 0x5678).as_u32(), 0x12345678u32)
    /// ```
    #[inline]
    pub const fn as_u32(&self) -> u32 {
        self.0
    }

    /// Returns `true` if this key represents a valid Private Reservation tag
    ///
    /// Private Reservation has form `(gggg,00xx)` where `gggg` - private
    /// attribute group this reservation used for. `xx` - any number in
    /// range 0x10..=0xff
    ///
    /// From [PS3.5 7.8.1 Private Data Element Tags](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_7.8.html):
    /// > Private Creator Data Elements numbered (gggg,0010-00FF) (gggg is odd)
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// assert!(!TagKey::new(0x1234, 0x0000).is_private_reservation());
    /// assert!(!TagKey::new(0x4321, 0x0000).is_private_reservation());
    /// assert!(!TagKey::new(0x4321, 0x0009).is_private_reservation());
    /// assert!(!TagKey::new(0x4321, 0x0110).is_private_reservation());
    /// assert!( TagKey::new(0x4321, 0x0010).is_private_reservation());
    /// assert!( TagKey::new(0x4321, 0x00BB).is_private_reservation());
    /// ```
    pub const fn is_private_reservation(&self) -> bool {
        (self.0 & 0x0001FF00u32) == 0x00010000u32 && (self.0 & 0xFFu32) >= 0x10u32
    }

    /// Returns `true` if this key represents a valid Private Attribute tag
    ///
    /// Private Attribute has form `(gggg,xxee)` where `gggg`, `ee` - vendor specific
    /// private attribute group and number, `xx` - number from a corresponding
    /// "Private Reservation" element in the current dataset in range 0x10..=0xFF.
    ///
    /// From [PS3.5 7.8.1 Private Data Element Tags](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_7.8.html):
    /// > Private Creator Data Element (gggg,0010) is required in order to identify Data Elements (gggg,1000-10FF) if present,\
    /// > Private Creator Data Element (gggg,0011) is required in order to identify Data Elements (gggg,1100-11FF) if present,\
    /// > through Private Creator Data Element (gggg,00FF), which identifies Data Elements (gggg,FF00-FFFF) if present.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// assert!(!TagKey::new(0x1234, 0x0000).is_private_attribute());
    /// assert!(!TagKey::new(0x4321, 0x0000).is_private_attribute());
    /// assert!( TagKey::new(0x4321, 0x1000).is_private_attribute());
    /// assert!( TagKey::new(0x4321, 0xAABB).is_private_attribute());
    /// ```
    pub const fn is_private_attribute(&self) -> bool {
        (self.0 & 0x00010000u32) == 0x00010000u32 && (self.0 & 0xFF00u32) >= 0x1000u32
    }

    /// Returns `true` if this key is "Private Attribute" or "Private Reservation" tag
    /// possibly invalid.
    ///
    /// This method simply tests if a group number is odd.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// assert!(!TagKey::new(0x1234, 0x0000).is_private());
    /// assert!( TagKey::new(0x4321, 0x0000).is_private());
    /// ```
    pub const fn is_private(&self) -> bool {
        self.0 & 0x00010000u32 != 0u32
    }

    /// Changes dynamic component of the tag element number to 0x10 and
    /// returns it IF this attribute is a valid "Private Attribute" or
    /// "Private Reservation"
    ///
    /// For clarification:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// // Valid "Private Reservation" is converted
    /// assert_eq!(
    ///          TagKey::new(0x4321, 0x0010).to_canonical_if_private(),
    ///     Some(TagKey::new(0x4321, 0x0010)));
    /// assert_eq!(
    ///          TagKey::new(0x4321, 0x00AA).to_canonical_if_private(),
    ///     Some(TagKey::new(0x4321, 0x0010)));
    /// // Valid "Private Attribute" is converted
    /// assert_eq!(
    ///          TagKey::new(0x4321, 0x10FF).to_canonical_if_private(),
    ///     Some(TagKey::new(0x4321, 0x10FF)));
    /// assert_eq!(
    ///          TagKey::new(0x4321, 0xAAFF).to_canonical_if_private(),
    ///     Some(TagKey::new(0x4321, 0x10FF)));
    /// // Invalid "Private Reservations" is not converted
    /// assert_eq!(TagKey::new(0x4321, 0x0009).to_canonical_if_private(),
    ///     None);
    /// // Invalid "Private Attribute" is not converted
    /// assert_eq!(TagKey::new(0x4321, 0x0F00).to_canonical_if_private(),
    ///     None);
    /// assert_eq!(TagKey::new(0x4321, 0x09AA).to_canonical_if_private(),
    ///     None);
    /// // Non-private attribute is not converted
    /// assert_eq!(TagKey::new(0x1234, 0x0010).to_canonical_if_private(),
    ///     None);
    /// ```
    pub const fn to_canonical_if_private(&self) -> Option<TagKey> {
        if self.0 & 0x0001FF00u32 == 0x00010000u32 && self.0 & 0xFFu32 >= 0x10u32 {
            Some(Self(self.0 & 0xFFFFFF00u32 | 0x10u32)) // private reservation
        } else if (self.0 & 0x00010000u32) == 0x00010000u32 && (self.0 & 0xFF00u32) >= 0x1000u32 {
            Some(Self(self.0 & 0xFFFF00FF | 0x1000u32)) // any other private attribute
        } else {
            None // Not a private attribute or invalid private attribute
        }

    }

    // cSpell:ignore aabb

    /// Returns `true` if this tag is valid
    ///
    /// From [PS3.5 7.8.1 Private Data Element Tags](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_7.8.html):
    /// > "Elements with Tags (0001,xxxx), (0003,xxxx), (0005,xxxx), (0007,xxxx) and (FFFF,xxxx) shall not be used."
    ///
    /// Note: standard does not explicitly rejects `(gggg,aabb)`, where `gggg` is odd,
    /// `aa` and `bb` in range 0x01..-0x0F, leaving it in a "grey" zone.
    ///
    /// Standard attributes with groups less than 0x0008 are used only in network DIMSE commands
    /// and should never appear in a Dataset. See `is_valid_in_dataset()`
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// assert!( TagKey::new(0x0000, 0x0000).is_valid());
    /// assert!( TagKey::new(0x0006, 0x0000).is_valid());
    /// assert!( TagKey::new(0x0008, 0x0000).is_valid());
    /// assert!( TagKey::new(0x1234, 0x4567).is_valid());
    /// assert!(!TagKey::new(0x0001, 0x1234).is_valid());
    /// assert!(!TagKey::new(0x0003, 0x1234).is_valid());
    /// assert!( TagKey::new(0x0009, 0x0000).is_valid());
    /// assert!(!TagKey::new(0xFFFF, 0x1234).is_valid());
    /// ```
    pub const fn is_valid(&self) -> bool {
        (self.0 & 0x00010000u32 == 0x00u32)
        || (self.0 & 0x00010000u32 == 0x00010000u32 && self.0 >= 0x00090000 && self.0 & 0xFFFF0000u32 != 0xFFFF0000u32)
    }

    /// Returns `true` if this tag may be used in a dataset body or a dataset header
    ///
    /// From [PS3.5 7.8.1 Private Data Element Tags](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_7.8.html):
    /// > Standard Data Elements have an even Group Number that is not (0000,eeee), (0002,eeee), (0004,eeee), or (0006,eeee).\
    /// >   Note: Usage of these groups is reserved for DIMSE Commands (see PS3.7) and DICOM File Formats.\
    /// > Private Data Elements have an odd Group Number that is not (0001,eeee), (0003,eeee), (0005,eeee), (0007,eeee), or (FFFF,eeee)
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// assert!(!TagKey::new(0x0000, 0x0000).is_valid_in_dataset());
    /// assert!(!TagKey::new(0x0006, 0x0000).is_valid_in_dataset());
    /// assert!( TagKey::new(0x0008, 0x0000).is_valid_in_dataset());
    /// assert!( TagKey::new(0x1234, 0x4567).is_valid_in_dataset());
    /// assert!(!TagKey::new(0x0001, 0x1234).is_valid_in_dataset());
    /// assert!(!TagKey::new(0x0003, 0x1234).is_valid_in_dataset());
    /// assert!( TagKey::new(0x0009, 0x0000).is_valid_in_dataset());
    /// assert!(!TagKey::new(0xFFFF, 0x1234).is_valid_in_dataset());
    /// ```
    pub const fn is_valid_in_dataset(&self) -> bool {
        self.is_valid() && (self.0 & 0xFFFF0000u32) >= 0x00080000u32
    }

    /// Searches tag information in the current [State](crate::State)
    ///
    /// See also [search_by_key](crate::tag::Dictionary::search_by_key)
    pub fn meta(&self) -> Option<crate::tag::Meta> {
        crate::State::with_current(|s| s.tag_dictionary().search_by_key(*self).cloned())
    }

    /// Searches and returns Tag name in the current [State](crate::State)
    ///
    /// See also [search_by_key](crate::tag::Dictionary::search_by_key)
    pub fn name(&self) -> Option<Cow<'static, str>> {
        crate::State::with_current(|s| {
            s.tag_dictionary()
                .search_by_key(*self)
                .map(|m| m.name.clone())
        })
    }
}

impl std::fmt::Display for TagKey {
    /// Outputs this key in format `(gggg,eeee)`, where `gggg` and `eeee`
    /// are group and element numbers in upper hexadecimal digits.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// let key = TagKey::new(0x4321, 0x10AA);
    /// assert_eq!(format!("{key}"), "(4321,10AA)");
    /// assert_eq!(TagKey::new(0x0008, 0x0005).to_string(), "(0008,0005)");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:04X},{:04X})", self.0 >> 16, self.0 & 0xFFFFu32)
    }
}

impl From<TagKey> for String {
    /// Outputs this key in format `(gggg,eeee)`, where `gggg` and `eeee`
    /// are group and element numbers in upper hexadecimal digits.
    ///
    /// See Display trait implementation [fmt](#method.fmt)
    fn from(value: TagKey) -> Self {
        format!("({:04X},{:04X})", value.0 >> 16, value.0 & 0xFFFFu32)
    }
}

impl From<TagKey> for u32 {
    /// Returns unsigned 32-bit representation of TagKey. group number in high word and element number in low word.
    fn from(value: TagKey) -> Self {
       value.0
    }
}

impl From<TagKey> for (u16, u16) {
    /// Returns unsigned 32-bit representation of TagKey. group number in high word and element number in low word.
    fn from(value: TagKey) -> Self {
       (value.group(), value.element())
    }
}

impl std::fmt::Debug for TagKey {
    /// Outputs this key in format `TagKey(gggg,eeee)`, where `gggg` and `eeee`
    /// are group and element numbers in upper hexadecimal digits.
    ///
    /// Example:
    /// ```
    /// # use ::dpx_dicom_core::TagKey;
    /// assert_eq!(
    ///     format!("{:?}", TagKey::new(0x4321, 0x10AA)),
    ///     "TagKey(4321,10AA)");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TagKey({:04X},{:04X})", self.0 >> 16, self.0 & 0xFFFFu32)
    }
}

impl From<u32> for TagKey {
    /// Construct a key from 32-bits unsigned. High word should contain group number and low word - element number.
    #[inline]
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<(u16, u16)> for TagKey {
    /// Construct a key from a tuple of two 16-bits unsigned. First - group number, second - element number.
    #[inline]
    fn from(value: (u16, u16)) -> Self {
        Self((value.0 as u32) << 16 | (value.1 as u32))
    }
}

impl<'a> PartialEq<Tag<'a>> for TagKey {
    #[inline]
    fn eq(&self, other: &Tag<'a>) -> bool {
        self.eq(&other.key)
    }
}

impl PartialEq<u32> for TagKey {
    #[inline]
    fn eq(&self, other: &u32) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<(u16, u16)> for TagKey {
    #[inline]
    fn eq(&self, other: &(u16, u16)) -> bool {
        self.eq(&Self::from(*other))
    }
}

impl<'a> PartialOrd<Tag<'a>> for TagKey {
    #[inline]
    fn partial_cmp(&self, other: &Tag<'a>) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.key)
    }
}

impl PartialOrd<u32> for TagKey {
    #[inline]
    fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<(u16, u16)> for TagKey {
    #[inline]
    fn partial_cmp(&self, other: &(u16, u16)) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&Self::from(*other))
    }
}

impl ::core::str::FromStr for TagKey {
    type Err = Error;
    /// Parses a text representation of the TagKey
    ///
    /// Allowed formats:
    /// - `(gggg,eeee)`
    ///
    /// Where `gggg` and `eeee` - hexadecimal group and element numbers.
    ///
    /// Examples:
    /// ```
    /// # use ::dpx_dicom_core::{tag::Error, TagKey, tag};
    /// # use ::core::str::FromStr;
    /// # fn main() -> Result<(), Error> {
    /// let expected = TagKey::new(0x0008, 0x0005);
    /// assert_eq!(TagKey::from_str("(0008,0005)")?, expected);
    ///
    /// let key: TagKey = "(0008,0005)".parse()?;
    /// assert_eq!(key, expected);
    ///
    /// assert!(matches!(TagKey::from_str("OOPS"), Err(tag::Error::TagKeyMissingOpeningBrace)));
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ensure!(s.starts_with('('), TagKeyMissingOpeningBraceSnafu);
        ensure!(s.ends_with(')'), TagKeyMissingClosingBraceSnafu);

        let mut components = s[1..s.len() - 1].splitn(3, ',');

        let group_chars = components.next().context(TagKeyMissingComponentsSnafu)?;
        let element_chars = components.next().context(TagKeyMissingComponentsSnafu)?;

        let group =
            u16::from_str_radix(group_chars, 16).context(TagKeyContainsNonHexCharactersSnafu)?;

        let element =
            u16::from_str_radix(element_chars, 16).context(TagKeyContainsNonHexCharactersSnafu)?;

        Ok(Self::new(group, element))
    }
}

impl<'a> TryFrom<&'a str> for TagKey {
    type Error = <TagKey as std::str::FromStr>::Err;
    /// See trait `FromStr::from_str` implementation for this struct
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        <TagKey as std::str::FromStr>::from_str(value)
    }
}

impl TryFrom<String> for TagKey {
    type Error = <TagKey as std::str::FromStr>::Err;
    /// See trait `FromStr::from_str` implementation for this struct
    fn try_from(value: String) -> Result<Self, Self::Error> {
        <TagKey as std::str::FromStr>::from_str(&value)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::StateBuilder, State};

    #[test]
    fn struct_methods() {
        let k = TagKey::from(0x12345678);

        // Generic constructors
        assert_eq!(k, 0x12345678);
        assert_eq!(TagKey::new(0x1234, 0x5678), k);
        assert_eq!(TagKey::from((0x1234, 0x5678)), k);
        assert_eq!(TagKey::from(0x12345678), k);

        // Accessors
        assert_eq!(k.group(), 0x1234);
        assert_eq!(k.element(), 0x5678);
        assert_eq!(k.as_u32(), 0x12345678);
        assert_eq!(0x12345678u32, k.into());
        assert_eq!((0x1234u16, 0x5678u16), k.into());

        // Try all the errors
        use Error::*;
        use ::core::str::FromStr;
        assert!(matches!(TagKey::from_str(""), Err(TagKeyMissingOpeningBrace)));
        assert!(matches!(TagKey::from_str("0008,0005)"), Err(TagKeyMissingOpeningBrace)));
        assert!(matches!(TagKey::from_str("(0008,0005"), Err(TagKeyMissingClosingBrace)));
        assert!(matches!(TagKey::from_str("(00080005)"), Err(TagKeyMissingComponents)));
        assert!(matches!(TagKey::from_str("(000Z,0005)"), Err(TagKeyContainsNonHexCharacters{source: _})));
        assert!(matches!(TagKey::from_str("(0008,000Z)"), Err(TagKeyContainsNonHexCharacters{source: _})));
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
        assert!(TestTag.meta().is_none());

        state.into_global();

        // Test tag should be found now, because we make our state global
        assert_eq!(TestTag.meta().unwrap().name, "Test Tag");

        // Restore default globaL state
        State::default().into_global();

        // Test tag should not present in the default global state
        assert!(TestTag.meta().is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn with_serde() {
        use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

        let k = TagKey::from(0x12345678);

        assert_ser_tokens(&k, &[Token::String("(1234,5678)")]);
        assert_de_tokens(&k, &[Token::BorrowedStr("(1234,5678)")]);
    }
}
