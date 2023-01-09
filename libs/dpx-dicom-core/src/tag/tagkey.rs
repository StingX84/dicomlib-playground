use super::*;

#[derive(Clone, Copy, Hash, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(try_from = "&str", into = "String"))]
#[repr(transparent)]
pub struct TagKey(
    pub u32
);

impl TagKey {
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn from_u32(k: u32) -> Self {
        Self(k)
    }

    #[inline]
    pub const fn from_components(g: u16, e: u16) -> Self {
        Self(((g as u32) << 16) | (e as u32))
    }

    #[inline]
    pub const fn group(&self) -> u16 {
        (self.0 >> 16) as u16
    }

    #[inline]
    pub const fn element(&self) -> u16 {
        (self.0 & 0xFFFFu32) as u16
    }

    #[inline]
    pub const fn as_u32(&self) -> u32 {
        self.0
    }

    pub const fn is_private_reservation(&self) -> bool {
        // PS3.5 7.8.1 Private Data Element Tags
        // > Private Creator Data Elements numbered (gggg,0010-00FF) (gggg is odd)
        (self.0 & 0x0001FF00u32) == 0x00010000u32 && (self.0 & 0xFFu32) >= 0x10u32
    }

    pub const fn is_private_attribute(&self) -> bool {
        // PS3.5 7.8.1 Private Data Element Tags
        // > Private Creator Data Element (gggg,0010) is required in order to identify Data Elements (gggg,1000-10FF) if present,
        // > Private Creator Data Element (gggg,0011) is required in order to identify Data Elements (gggg,1100-11FF) if present,
        // > through Private Creator Data Element (gggg,00FF), which identifies Data Elements (gggg,FF00-FFFF) if present.
        (self.0 & 0x00010000u32) == 0x00010000u32 && (self.0 & 0xFF00u32) >= 0x1000u32
    }

    pub const fn is_private_any(&self) -> bool {
        self.0 & 0x00010000u32 != 0u32
    }

    pub const fn to_canonical_if_private(&self) -> Option<TagKey> {
        if ! self.is_private_any() { None } // not a private attribute
        else if self.0 & 0x0000FF00u32 == 0 && self.0 & 0xFFu32 >= 0x10u32 { Some(Self(self.0 & 0xFFFFFF00u32 | 0x10u32)) } // private reservation
        else { Some(Self(self.0 & 0xFFFF00FF | 0x1000u32)) } // any other private attribute
    }

    // cSpell:ignore aabb
    pub const fn is_valid(&self) -> bool {
        // PS3.5 7.8.1 Private Data Element Tags:
        // > "Elements with Tags (0001,xxxx), (0003,xxxx), (0005,xxxx), (0007,xxxx) and (FFFF,xxxx) shall not be used."
        // Note: standard does not explicitly rejects (gggg,aabb), where gggg is odd,
        // aa and bb in range 0x01..0x10, leaving it in a "grey" zone.
        (self.0 & 0x00010000u32 == 0x00u32)
        || (self.0 & 0x00010000u32 == 0x00010000u32 && self.0 >= 0x00090000 && self.0 & 0xFFFF0000u32 != 0xFFFF0000u32)
    }

    pub const fn is_valid_in_dataset(&self) -> bool {
        // PS3.5 7.1 Data elements:
        // > Standard Data Elements have an even Group Number that is not (0000,eeee), (0002,eeee), (0004,eeee), or (0006,eeee).
        // > Note: Usage of these groups is reserved for DIMSE Commands (see PS3.7) and DICOM File Formats.
        // > Private Data Elements have an odd Group Number that is not (0001,eeee), (0003,eeee), (0005,eeee), (0007,eeee), or (FFFF,eeee)
        self.is_valid() && (self.0 & 0xFFFF0000u32) >= 0x00080000u32
    }
}

/// Custom Display trait, that presents a ['TagKey'] with format `(0008,0005)`
impl std::fmt::Display for TagKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:04x},{:04x})", self.0 >> 16, self.0 & 0xFFFFu32)
    }
}

impl From<TagKey> for String {
    fn from(value: TagKey) -> Self {
        format!("({:04x},{:04x})", value.0 >> 16, value.0 & 0xFFFFu32)
    }
}

/// Custom Debug trait, that presents a ['TagKey'] with format `TagKey(0008,0005)`
impl std::fmt::Debug for TagKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TagKey(0x{:08x})", self.0)
    }
}

impl From<u32> for TagKey {
    #[inline]
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<(u16, u16)> for TagKey {
    #[inline]
    fn from(value: (u16, u16)) -> Self {
        Self((value.0 as u32) << 16 | (value.1 as u32))
    }
}

impl From<[u16; 2]> for TagKey {
    #[inline]
    fn from(value: [u16; 2]) -> Self {
        Self((value[0] as u32) << 16 | (value[1] as u32))
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

impl std::str::FromStr for TagKey {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('(') {
            let (g, e) = s
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .and_then(|s| s.split_once(&[',', ':']))
                .context(TagKeyInBracesMissingSeparatorSnafu)?;

            let ng = u16::from_str_radix(g, 16).context(TagKeyContainsNonHexCharactersSnafu)?;
            let ne = u16::from_str_radix(e, 16).context(TagKeyContainsNonHexCharactersSnafu)?;

            return Ok(Self::from_components(ng, ne));
        }

        if s.len() == 9 && (s.as_bytes()[4] == b':' || s.as_bytes()[4] == b',') {
            // Panic: this will never panic, because we've previously checked, that array contains requested characters
            let (g, e) = s.split_once(&[',', ':']).unwrap();

            let ng = u16::from_str_radix(g, 16).context(TagKeyContainsNonHexCharactersSnafu)?;
            let ne = u16::from_str_radix(e, 16).context(TagKeyContainsNonHexCharactersSnafu)?;

            return Ok(Self::from_components(ng, ne));
        }

        if s.len() == 8 && s.as_bytes().iter().all(|c| c.is_ascii_hexdigit()) {
            let ge = u32::from_str_radix(s, 16).context(TagKeyContainsNonHexCharactersSnafu)?;
            return Ok(Self(ge));
        }

        UnrecognizedTagKeyFormatSnafu.fail()
    }
}

impl<'a> TryFrom<&'a str> for TagKey {
    type Error = <TagKey as std::str::FromStr>::Err;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        <TagKey as std::str::FromStr>::from_str(value)
    }
}

impl TryFrom<String> for TagKey {
    type Error = <TagKey as std::str::FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        <TagKey as std::str::FromStr>::from_str(&value)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_methods() {
        let k = TagKey::from(0x12345678);

        // Generic constructors
        assert_eq!(k, 0x12345678);
        assert_eq!(TagKey::from_components(0x1234, 0x5678), k);
        assert_eq!(TagKey::from((0x1234, 0x5678)), k);
        assert_eq!(TagKey::from([0x1234, 0x5678]), k);
        assert_eq!(TagKey::new(), TagKey::default());

        // Accessors
        assert_eq!(k.group(), 0x1234);
        assert_eq!(k.element(), 0x5678);
        assert_eq!(k.as_u32(), k.0);

        // Private attributes detection
        assert!(TagKey::from(0x43210010).is_private_reservation());
        assert!(!TagKey::from(0x12340010).is_private_reservation());
        assert!(!TagKey::from(0x43210009).is_private_reservation());
        assert!(TagKey::from(0x43211010).is_private_attribute());
        assert!(TagKey::from(0x43211000).is_private_attribute());
        assert!(TagKey::from(0x432110FF).is_private_attribute());
        assert!(TagKey::from(0x4321FFFF).is_private_attribute());
        assert!(!TagKey::from(0x12341000).is_private_attribute());
        assert!(!TagKey::from(0x43210900).is_private_attribute());
        assert!(TagKey::from(0x43210000).is_private_any());
        assert!(TagKey::from(0x4321FFFF).is_private_any());
        assert!(!TagKey::from(0x12341010).is_private_any());

        // Private canonical transformation
        assert_eq!(TagKey(0x43210010).to_canonical_if_private(), Some(TagKey(0x43210010)));
        assert_eq!(TagKey(0x43210011).to_canonical_if_private(), Some(TagKey(0x43210010)));
        assert_eq!(TagKey(0x432100FF).to_canonical_if_private(), Some(TagKey(0x43210010)));
        assert_eq!(TagKey(0x43210009).to_canonical_if_private(), Some(TagKey(0x43211009)));
        assert_eq!(TagKey(0x432101FF).to_canonical_if_private(), Some(TagKey(0x432110FF)));
        assert_eq!(TagKey(0x43210100).to_canonical_if_private(), Some(TagKey(0x43211000)));
        assert_eq!(TagKey(0x43210000).to_canonical_if_private(), Some(TagKey(0x43211000)));
        assert_eq!(TagKey(0x12345678).to_canonical_if_private(), None);

        // Validation
        assert!(TagKey(0x12345678).is_valid());
        assert!(TagKey(0x43215678).is_valid());
        assert!(TagKey(0x00000000).is_valid());
        assert!(!TagKey(0x00010000).is_valid());
        assert!(!TagKey(0x0007FFFF).is_valid());
        assert!(TagKey(0x0008FFFF).is_valid());
        assert!(TagKey(0x0009FFFF).is_valid());
        assert!(TagKey(0x000AFFFF).is_valid());
        assert!(!TagKey(0xFFFFFFFF).is_valid());
        assert!(!TagKey(0xFFFF5678).is_valid());
        assert!(!TagKey(0xFFFF0000).is_valid());
        assert!(TagKey(0xFFFE0000).is_valid());
        assert!(TagKey(0xEFFF0000).is_valid());
        assert!(!TagKey(0x00000000).is_valid_in_dataset());
        assert!(!TagKey(0x00010000).is_valid_in_dataset());
        assert!(!TagKey(0x0007FFFF).is_valid_in_dataset());
        assert!(TagKey(0x00080000).is_valid_in_dataset());
        assert!(TagKey(0x0008FFFF).is_valid_in_dataset());
        assert!(TagKey(0x00090000).is_valid_in_dataset());

        // string transformations
        assert_eq!(k.to_string(), "(1234,5678)");
        assert_eq!(format!("{}", k), "(1234,5678)");
        assert_eq!(format!("{:?}", k), "TagKey(0x12345678)");

        assert_eq!(TagKey::new().to_string(), "(0000,0000)");
        assert_eq!(format!("{}", TagKey::new()), "(0000,0000)");
        assert_eq!(format!("{:?}", TagKey::new()), "TagKey(0x00000000)");

        assert_eq!("(1234,5678)".parse::<TagKey>().unwrap(), k);
        assert_eq!("(0000,0000)".parse::<TagKey>().unwrap(), TagKey::default());
        assert_eq!("(FFFF,FFFF)".parse::<TagKey>().unwrap(), TagKey(0xFFFFFFFF));
        assert_eq!(TagKey::try_from("(1234:5678)").unwrap(), k);
        assert_eq!(TagKey::try_from(String::from("(1234:5678)")).unwrap(), k);
        assert_eq!(TagKey::try_from("(1234,5678)").unwrap(), k);
        assert_eq!(TagKey::try_from("1234,5678").unwrap(), k);
        assert_eq!(TagKey::try_from("1234:5678").unwrap(), k);
        assert_eq!(TagKey::try_from("12345678").unwrap(), k);
        // assert!(matches!(TagKey::try_from("(1234,5678").unwrap_err(), Error::));
        // assert!(matches!(TagKey::try_from("(1234 5678)").unwrap_err(), ErrKind::InvalidTagKey));
        // assert!(matches!(TagKey::try_from("(1234,567z)").unwrap_err(), ErrKind::InvalidTagKey));
        // assert!(matches!(TagKey::try_from("(123z,5678)").unwrap_err(), ErrKind::InvalidTagKey));
        // assert!(matches!(TagKey::try_from("1234,567z").unwrap_err(), ErrKind::InvalidTagKey));
        // assert!(matches!(TagKey::try_from("1234 5678").unwrap_err(), ErrKind::InvalidTagKey));
        // assert!(matches!(TagKey::try_from("1234,567").unwrap_err(), ErrKind::InvalidTagKey));
        // assert!(matches!(TagKey::try_from("1234567").unwrap_err(), ErrKind::InvalidTagKey));
        // assert!(matches!(TagKey::try_from("1234567z").unwrap_err(), ErrKind::InvalidTagKey));

    }

    #[test]
    #[cfg(serde)]
    fn with_serde() {
        use serde_test::{assert_de_tokens, assert_ser_tokens, Token};

        let k = TagKey::from(0x12345678);

        assert_ser_tokens(&k, &[Token::String("(1234,5678)")]);
        assert_de_tokens(&k, &[Token::BorrowedStr("(1234,5678)")]);

        #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
        struct Test {
            key: Option<TagKey>,
        }

        assert_eq!(serde_yaml::from_str::<Test>(r#"key: "(1234,5678)""#).unwrap(), Test{key: Some(k)});
        assert_eq!(serde_yaml::from_str::<Test>(r#"key: null"#).unwrap(), Test{key: None});
        assert_eq!(serde_yaml::to_string(&Test {key: Some(k)}).unwrap(), "key: (1234,5678)\n".to_owned());
        assert_eq!(serde_yaml::to_string(&Test {key: None}).unwrap(), "key: null\n".to_owned());
    }
}
