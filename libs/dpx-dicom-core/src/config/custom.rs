//! Application-defined configuration types.
//!
//! [`Value::Custom`](super::Value::Custom) carries an opaque, type-erased
//! payload owned by the application. To let the library load, save and validate
//! such a value without knowing its concrete type, the application supplies a
//! [`CustomType`] — a fully `static` descriptor referenced from
//! [`ValueMeta::Custom`](super::meta::ValueMeta::Custom).
//!
//! A [`CustomType`] exchanges values through [`serde_json::Value`], a
//! backend-neutral mirror of a serialized subtree. A GUI/TUI editor selects the
//! right editing component by [`CustomType::name`] and round-trips the JSON
//! form produced by [`CustomType::encode`].
//!
//! Most applications need no hand-written codec: [`Serde<T>`] adapts any type
//! deriving [`serde::Serialize`]/[`serde::Deserialize`] into a [`CustomType`].

use crate::{Arc, dicom_err, error::Result};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value as JsonValue;
use std::any::Any;
use std::marker::PhantomData;

/// An application-defined configuration type.
///
/// Implemented by the application — typically on a zero-sized unit struct held
/// in a `static`, so the whole descriptor lives in read-only memory with no
/// heap allocation. The only allocation involved is the [`Arc`](std::sync::Arc) holding an
/// individual decoded value, which is shared cheaply across configuration layers.
///
/// For types that derive serde, prefer [`Serde<T>`] over a hand-written impl.
///
/// ```
/// use std::any::Any;
/// use std::sync::Arc;
/// use std::net::Ipv4Addr;
/// use serde_json::Value as JsonValue;
/// use dpx_dicom_core::config::CustomType;
/// use dpx_dicom_core::{dicom_err, error::Result};
///
/// #[derive(Debug, PartialEq)]
/// struct IpRange { lo: Ipv4Addr, hi: Ipv4Addr }
///
/// struct IpRangeType;
/// impl CustomType for IpRangeType {
///     fn name(&self) -> &'static str { "ipRange" }
///     fn decode(&self, node: &JsonValue) -> Result<Arc<dyn Any + Send + Sync>> {
///         let s = node.as_str()
///             .ok_or_else(|| dicom_err!(InvalidData, "ipRange expects a string"))?;
///         let (lo, hi) = s.split_once('-')
///             .ok_or_else(|| dicom_err!(InvalidData, "ipRange expects 'lo-hi'"))?;
///         Ok(Arc::new(IpRange {
///             lo: lo.trim().parse().map_err(|_| dicom_err!(InvalidData, "bad ip"))?,
///             hi: hi.trim().parse().map_err(|_| dicom_err!(InvalidData, "bad ip"))?,
///         }))
///     }
///     fn encode(&self, value: &dyn Any) -> Result<JsonValue> {
///         let r = value.downcast_ref::<IpRange>()
///             .ok_or_else(|| dicom_err!(Internal, "ipRange got wrong value type"))?;
///         Ok(JsonValue::String(format!("{}-{}", r.lo, r.hi)))
///     }
/// }
///
/// static IP_RANGE: IpRangeType = IpRangeType;
/// let ty: &'static dyn CustomType = &IP_RANGE;
/// let v = ty.decode(&JsonValue::String("10.0.0.1-10.0.0.9".into())).unwrap();
/// assert_eq!(ty.encode(v.as_ref()).unwrap(), JsonValue::String("10.0.0.1-10.0.0.9".into()));
/// ```
pub trait CustomType: Send + Sync + 'static {
    /// Stable machine name of the type, e.g. `"ipRange"`. Used by an editor to
    /// pick the matching editing component.
    fn name(&self) -> &'static str;

    /// Decodes a serialized subtree into the runtime value (deserialization signal).
    fn decode(&self, node: &JsonValue) -> Result<Arc<dyn Any + Send + Sync>>;

    /// Encodes a runtime value back into a serialized subtree (serialization signal).
    ///
    /// Implementations downcast `value` to their concrete type; a mismatch is a
    /// library invariant violation and should yield an `Internal` error.
    fn encode(&self, value: &dyn Any) -> Result<JsonValue>;

    /// Phase-one validation of an already-decoded value. Defaults to accepting.
    fn validate(&self, _value: &dyn Any) -> Result<()> {
        Ok(())
    }
}

// `ValueMeta` derives `Debug`; give the trait object a `Debug` so a
// `&'static dyn CustomType` field does not block that derive.
impl std::fmt::Debug for dyn CustomType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomType").field("name", &self.name()).finish()
    }
}

/// Adapts any serde type into a [`CustomType`], so an application gets load and
/// save handling for free from a `derive`.
///
/// Hold it in a `static`, naming the editor component:
///
/// ```
/// use serde::{Serialize, Deserialize};
/// use std::net::Ipv4Addr;
/// use dpx_dicom_core::config::Serde;
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct IpRange { lo: Ipv4Addr, hi: Ipv4Addr }
///
/// static IP_RANGE: Serde<IpRange> = Serde::new("ipRange");
/// ```
///
/// The wire format is whatever serde derives for `T`. When a non-derivable
/// representation or semantic [`validate`](CustomType::validate) is needed,
/// implement [`CustomType`] by hand instead.
pub struct Serde<T> {
    name: &'static str,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Serde<T> {
    pub const fn new(name: &'static str) -> Self {
        Self { name, _marker: PhantomData }
    }
}

impl<T> CustomType for Serde<T>
where
    T: Serialize + DeserializeOwned + Any + Send + Sync,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn decode(&self, node: &JsonValue) -> Result<Arc<dyn Any + Send + Sync>> {
        let v: T = serde_json::from_value(node.clone())
            .map_err(|e| dicom_err!(InvalidData, "{}: {e}", self.name))?;
        Ok(Arc::new(v))
    }

    fn encode(&self, value: &dyn Any) -> Result<JsonValue> {
        let v = value
            .downcast_ref::<T>()
            .ok_or_else(|| dicom_err!(Internal, "{} got wrong value type", self.name))?;
        serde_json::to_value(v).map_err(|e| dicom_err!(Internal, "{}: {e}", self.name))
    }
}
