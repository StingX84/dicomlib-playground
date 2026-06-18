//! Application-defined configuration types.
//!
//! [`Value::Complex`](super::Value::Complex) carries an opaque, type-erased
//! payload owned by the application. To let the library load, save and validate
//! such a value without knowing its concrete type, the application supplies a
//! [`ComplexType`] — a fully `static` descriptor referenced from
//! [`ValueMeta::Complex`](super::meta::ValueMeta::Complex).
//!
//! A [`ComplexType`] exchanges values through [`ConfigNode`], a backend-neutral
//! mirror of a serialized subtree that maps one-to-one onto JSON. A GUI/TUI
//! editor selects the right editing component by [`ComplexType::name`] and
//! round-trips the JSON form of the [`ConfigNode`] produced by
//! [`ComplexType::encode`].

use crate::{Arc, error::Result};
use std::any::Any;

/// A backend-neutral, JSON-shaped view of a serialized value subtree.
///
/// Loaders parse their format (YAML today) into this; codecs decode from and
/// encode to it. It is only materialized transiently around load/save.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigNode {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Seq(Vec<ConfigNode>),
    /// Order-preserving object. Linear lookup is adequate at config sizes and
    /// keeps round-trips deterministic.
    Map(Vec<(String, ConfigNode)>),
}

impl ConfigNode {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigNode::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            ConfigNode::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConfigNode::Float(f) => Some(*f),
            ConfigNode::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigNode::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[ConfigNode]> {
        match self {
            ConfigNode::Seq(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&[(String, ConfigNode)]> {
        match self {
            ConfigNode::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the value of map entry `key`, if this is a map containing it.
    pub fn get(&self, key: &str) -> Option<&ConfigNode> {
        self.as_map()?.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Returns a short, stable name of the node's variant, for diagnostics.
    pub fn kind_name(&self) -> &'static str {
        match self {
            ConfigNode::Null => "Null",
            ConfigNode::Bool(_) => "Bool",
            ConfigNode::Int(_) => "Int",
            ConfigNode::Float(_) => "Float",
            ConfigNode::Str(_) => "Str",
            ConfigNode::Seq(_) => "Seq",
            ConfigNode::Map(_) => "Map",
        }
    }
}

/// An application-defined configuration type.
///
/// Implemented by the application — typically on a zero-sized unit struct held
/// in a `static`, so the whole descriptor lives in read-only memory with no
/// heap allocation. The only allocation involved is the [`Arc`](std::sync::Arc) holding an
/// individual decoded value, which is shared cheaply across configuration layers.
///
/// ```
/// use std::any::Any;
/// use std::sync::Arc;
/// use std::net::Ipv4Addr;
/// use dpx_dicom_core::config::{ComplexType, ConfigNode};
/// use dpx_dicom_core::{dicom_err, error::Result};
///
/// #[derive(Debug, PartialEq)]
/// struct IpRange { lo: Ipv4Addr, hi: Ipv4Addr }
///
/// struct IpRangeType;
/// impl ComplexType for IpRangeType {
///     fn name(&self) -> &'static str { "ipRange" }
///     fn decode(&self, node: &ConfigNode) -> Result<Arc<dyn Any + Send + Sync>> {
///         let s = node.as_str()
///             .ok_or_else(|| dicom_err!(InvalidData, "ipRange expects a string"))?;
///         let (lo, hi) = s.split_once('-')
///             .ok_or_else(|| dicom_err!(InvalidData, "ipRange expects 'lo-hi'"))?;
///         Ok(Arc::new(IpRange {
///             lo: lo.trim().parse().map_err(|_| dicom_err!(InvalidData, "bad ip"))?,
///             hi: hi.trim().parse().map_err(|_| dicom_err!(InvalidData, "bad ip"))?,
///         }))
///     }
///     fn encode(&self, value: &dyn Any) -> Result<ConfigNode> {
///         let r = value.downcast_ref::<IpRange>()
///             .ok_or_else(|| dicom_err!(Internal, "ipRange got wrong value type"))?;
///         Ok(ConfigNode::Str(format!("{}-{}", r.lo, r.hi)))
///     }
/// }
///
/// static IP_RANGE: IpRangeType = IpRangeType;
/// let ty: &'static dyn ComplexType = &IP_RANGE;
/// let v = ty.decode(&ConfigNode::Str("10.0.0.1-10.0.0.9".into())).unwrap();
/// assert_eq!(ty.encode(v.as_ref()).unwrap(), ConfigNode::Str("10.0.0.1-10.0.0.9".into()));
/// ```
pub trait ComplexType: Send + Sync + 'static {
    /// Stable machine name of the type, e.g. `"ipRange"`. Used by an editor to
    /// pick the matching editing component.
    fn name(&self) -> &'static str;

    /// Decodes a serialized subtree into the runtime value (deserialization signal).
    fn decode(&self, node: &ConfigNode) -> Result<Arc<dyn Any + Send + Sync>>;

    /// Encodes a runtime value back into a serialized subtree (serialization signal).
    ///
    /// Implementations downcast `value` to their concrete type; a mismatch is a
    /// library invariant violation and should yield an `Internal` error.
    fn encode(&self, value: &dyn Any) -> Result<ConfigNode>;

    /// Phase-one validation of an already-decoded value. Defaults to accepting.
    fn validate(&self, _value: &dyn Any) -> Result<()> {
        Ok(())
    }
}

// `ValueMeta` derives `Debug`; give the trait object a `Debug` so a
// `&'static dyn ComplexType` field does not block that derive.
impl std::fmt::Debug for dyn ComplexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComplexType").field("name", &self.name()).finish()
    }
}
