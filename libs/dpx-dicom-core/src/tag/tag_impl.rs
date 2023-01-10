use super::TagKey;
use crate::{ Vr, Cow };

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tag<'a> {
    pub key: TagKey,
    pub creator: Option<Cow<'a, str>>,
    pub vr: Option<Vr>,
}

impl<'a> Tag<'a> {
    pub const fn new(key: TagKey, creator: Option<Cow<'a, str>>, vr: Option<Vr>,) -> Self {
        Self {
            key,
            creator,
            vr,
        }
    }

    pub const fn from_key(key: TagKey) -> Self {
        Self {
            key,
            creator: None,
            vr: None,
        }
    }
}

/// Custom Display trait, that presents a ['Tag'] with format `(4321,0010,"PrivateCreator"):LO`
impl<'a> std::fmt::Display for Tag<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = &self.creator {
            if let Some(vr) = &self.vr {
                write!(f, "({:04x},{:04x},\"{}\"):{}", self.key.group(), self.key.element(), v.escape_default(), vr)
            } else {
                write!(f, "({:04x},{:04x},\"{}\")", self.key.group(), self.key.element(), v.escape_default())
            }
        } else if let Some(vr) = &self.vr {
            write!(f, "({:04x},{:04x}):{}", self.key.group(), self.key.element(), vr)
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

/// Custom Debug trait, that presents a ['Tag'] with format `Tag(0x00080005,"PrivateCreator",Vr)`
impl<'a> std::fmt::Debug for Tag<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tag(0x{:08x}", self.key.as_u32())?;
        if let Some(v) = &self.creator {
            write!(f, ",\"{}\"", v.escape_default())?;
        }
        if let Some(v) = &self.vr {
            write!(f, ",{v}")?;
        }
        f.write_str(")")
    }
}

impl From<TagKey> for Tag<'static> {
    fn from(value: TagKey) -> Self {
        Self::from_key(value)
    }
}

impl From<u32> for Tag<'static> {
    fn from(value: u32) -> Self {
        Self::from_key(value.into())
    }
}

impl From<(u16, u16)> for Tag<'static> {
    fn from(value: (u16, u16)) -> Self {
        Self::from_key(value.into())
    }
}

impl From<[u16; 2]> for Tag<'static> {
    fn from(value: [u16; 2]) -> Self {
        Self::from_key(value.into())
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
        let k = Tag::from(0x12345678);

        // Generic constructors


        // string transformations
        assert_eq!(k.to_string(), "(1234,5678)");
        assert_eq!(format!("{k}"), "(1234,5678)");
        assert_eq!(format!("{k:?}"), "Tag(0x12345678)");

        assert_eq!(Tag::default().to_string(), "(0000,0000)");
        assert_eq!(format!("{}", Tag::default()), "(0000,0000)");
        assert_eq!(format!("{:?}", Tag::default()), "Tag(0x00000000)");

        let k = Tag::new(TagKey(0x123456), Some(Cow::Borrowed("Test")), Some(Vr::UN));
        println!("{k}");
        println!("{k:?}");
    }
}
