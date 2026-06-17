//! Configuration metadata: keys, value descriptors and the registry.
//!
//! This is the vocabulary an application uses to *describe* its configuration
//! surface so it can be validated, edited in a GUI/TUI and documented without
//! hard-coding any particular key. The runtime payload is a
//! [`Value`]; the descriptor here is [`ValueMeta`].

use super::Value;
use crate::{
    HashMap, dicom_err,
    error::{ErrContext, Result},
};

/// A statically- or dynamically-sourced list of choices.
///
/// Enumerations and "one of" constraints are often known at compile time, but
/// some (e.g. the set of supported character sets) are assembled at runtime.
#[derive(Debug, Clone)]
pub enum MaybeGenerated<T>
where
    T: 'static,
{
    Static(&'static [T]),
    Dynamic(fn() -> Box<dyn Iterator<Item = T>>),
}

impl<T: Clone> MaybeGenerated<T> {
    /// Iterates over the choices, materializing the dynamic variant on demand.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T> + '_> {
        match self {
            MaybeGenerated::Static(s) => Box::new(s.iter().cloned()),
            MaybeGenerated::Dynamic(f) => f(),
        }
    }
}

/// Uniquely identifies a configuration key.
///
/// `module` namespaces keys per crate/application; `code` is typically the
/// source line of the key declaration, making collisions within a module
/// impossible by construction.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Key {
    pub module: &'static str,
    pub code: u32,
}

impl Key {
    #[inline]
    pub const fn new(module: &'static str, code: u32) -> Key {
        Key { module, code }
    }
}

/// Human-facing identity of a key or an enum choice.
///
/// `name` is the stable machine identifier used in serialized config; the rest
/// drives presentation in an editor.
#[derive(Debug, Clone)]
pub struct Concept {
    pub name: &'static str,
    pub display_name: &'static str,
    pub help_string: Option<&'static str>,
}

impl Concept {
    pub const fn new(name: &'static str, display_name: &'static str, help_string: Option<&'static str>) -> Concept {
        Self {
            name,
            display_name,
            help_string,
        }
    }
}

/// Describes the admissible shape and constraints of a [`Value`].
#[derive(Debug, Clone)]
pub enum ValueMeta {
    Bool,
    String {
        regexp: Option<&'static str>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    Int {
        min: Option<i64>,
        max: Option<i64>,
    },
    Enum {
        values: MaybeGenerated<(u32, Concept)>,
    },
    Duration {
        min_msec: Option<i64>,
        max_msec: Option<i64>,
    },
    Tag {
        filter_by_vr: Option<&'static [crate::vr::Vr]>,
        one_of: Option<MaybeGenerated<crate::tag::Tag>>,
    },
    Vr {
        one_of: Option<MaybeGenerated<crate::vr::Vr>>,
    },
    File {
        name_only: bool,
        content_only: bool,
    },
    Vec {
        items: &'static ValueMeta,
        min_length: Option<usize>,
        max_length: Option<usize>,
        stride: Option<usize>,
    },
    Map {
        keys: &'static ValueMeta,
        values: &'static ValueMeta,
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    Complex {
        ty: &'static dyn super::ComplexType,
        limits: &'static [(&'static str, &'static str)],
    },
}

impl ValueMeta {
    /// Validates a single [`Value`] against this descriptor.
    ///
    /// This is phase one of configuration validation: it checks that the value
    /// has the expected variant and respects its declared constraints. It does
    /// not perform cross-key consistency checks (phase two).
    ///
    /// `filter_by_vr` on [`ValueMeta::Tag`] is intentionally not checked here:
    /// a tag's VR is resolved through the active dictionary, which belongs to a
    /// later, context-aware validation pass.
    pub fn validate(&self, value: &Value) -> Result<()> {
        match (self, value) {
            (ValueMeta::Bool, Value::Bool(_)) => Ok(()),

            (
                ValueMeta::String {
                    regexp,
                    min_length,
                    max_length,
                },
                Value::String(s),
            ) => {
                let len = s.chars().count();
                check_range("string len", len, min_length, max_length)?;

                if let Some(pattern) = regexp {
                    let re = regex::Regex::new(pattern)
                        .map_err(|e| dicom_err!(Internal, "invalid validation regex {pattern:?}: {e}"))?;
                    if !re.is_match(s) {
                        return Err(dicom_err!(
                            Configuration,
                            "value {s:?} does not match required pattern {pattern:?}"
                        ));
                    }
                }
                Ok(())
            }

            (ValueMeta::Int { min, max }, Value::Int(n)) => check_range("integer", *n, min, max),

            (ValueMeta::Enum { values }, Value::Enum(n)) => {
                if values.iter().any(|(code, _)| code == *n) {
                    Ok(())
                } else {
                    Err(dicom_err!(Configuration, "value {n} is not a valid enum choice"))
                }
            }

            (ValueMeta::Duration { min_msec, max_msec }, Value::Duration(d)) => {
                check_range("duration(ms)", d.as_millis() as i64, min_msec, max_msec)
            }

            (ValueMeta::Tag { one_of, .. }, Value::Tag(t)) => match one_of {
                Some(allowed) if !allowed.iter().any(|candidate| candidate == *t) => {
                    Err(dicom_err!(Configuration, "tag {t} is not among the allowed tags"))
                }
                _ => Ok(()),
            },

            (ValueMeta::Vr { one_of }, Value::Vr(vr)) => match one_of {
                Some(allowed) if !allowed.iter().any(|candidate| candidate == *vr) => {
                    Err(dicom_err!(Configuration, "VR {vr} is not among the allowed VRs"))
                }
                _ => Ok(()),
            },

            (
                ValueMeta::File {
                    name_only,
                    content_only,
                },
                Value::File(f),
            ) => match f {
                super::ValueFile::Name { .. } if *content_only => Err(dicom_err!(
                    Configuration,
                    "file value must be inline content, not a path"
                )),
                super::ValueFile::Content(_) if *name_only => Err(dicom_err!(
                    Configuration,
                    "file value must be a path, not inline content"
                )),
                _ => Ok(()),
            },

            (
                ValueMeta::Vec {
                    items,
                    min_length,
                    max_length,
                    stride,
                },
                Value::Vec(elements),
            ) => {
                let len = elements.len();
                check_range("vector", len, min_length, max_length)?;
                if let Some(stride_v) = *stride
                    && stride_v > 0
                    && len % stride_v != 0
                {
                    return Err(dicom_err!(
                        Configuration,
                        "length {} is not multiple of {}",
                        len,
                        stride_v
                    ));
                }
                for (idx, element) in elements.iter().enumerate() {
                    items.validate(element).err_context_with(|| format!("at index {idx}"))?;
                }
                Ok(())
            }

            (
                ValueMeta::Map {
                    keys,
                    values,
                    min_length,
                    max_length,
                },
                Value::Map(entries),
            ) => {
                check_range("map", entries.len(), min_length, max_length)?;
                for (k, v) in entries.iter() {
                    keys.validate(k).err_context("invalid map key")?;
                    values.validate(v).err_context("invalid map value")?;
                }
                Ok(())
            }

            // Complex values are application-defined; the registered codec is the
            // only thing that understands the concrete type, so delegate to it.
            (ValueMeta::Complex { ty, .. }, Value::Complex(any)) => {
                ty.validate(any.as_ref())
            }

            (meta, value) => Err(dicom_err!(
                Internal,
                "type mismatch: value of kind {} does not fit a {} descriptor",
                value.kind_name(),
                meta.kind_name()
            )),
        }
    }

    /// Returns a short, stable name of the descriptor's variant, for diagnostics.
    pub fn kind_name(&self) -> &'static str {
        match self {
            ValueMeta::Bool => "Bool",
            ValueMeta::String { .. } => "String",
            ValueMeta::Int { .. } => "Int",
            ValueMeta::Enum { .. } => "Enum",
            ValueMeta::Duration { .. } => "Duration",
            ValueMeta::Tag { .. } => "Tag",
            ValueMeta::Vr { .. } => "Vr",
            ValueMeta::File { .. } => "File",
            ValueMeta::Vec { .. } => "Vec",
            ValueMeta::Map { .. } => "Map",
            ValueMeta::Complex { .. } => "Complex",
        }
    }
}

fn check_range<T: Ord + std::fmt::Display>(
    what: &str,
    value: T,
    bound_min: &Option<T>,
    bound_max: &Option<T>,
) -> Result<()> {
    match (bound_min, bound_max) {
        (Some(min), Some(max)) if value < *min || value > *max => Err(dicom_err!(
            Configuration,
            "{}{} outside of required range {}..{}",
            what,
            value,
            *min,
            *max
        )),
        (Some(min), None) if value < *min => Err(dicom_err!(
            Configuration,
            "{}{} < required minimum {}",
            what,
            value,
            *min
        )),
        (None, Some(max)) if value > *max => Err(dicom_err!(
            Configuration,
            "{}{} > allowed maximum {}",
            what,
            value,
            *max
        )),
        _ => Ok(()),
    }
}

/// Full metadata for one configuration key.
#[derive(Debug, Clone)]
pub struct KeyMeta {
    pub key: Key,
    pub is_advanced: bool,
    pub display_section: &'static str,
    pub concept: Concept,
    pub value_meta: ValueMeta,
    pub make_default: fn() -> Option<Value>,
}

/// A statically-registered batch of key descriptors.
///
/// Modules and applications contribute their keys at link time via
/// `inventory::submit! { StaticRegistry(&[ ... ]) }`.
pub struct StaticRegistry(pub &'static [KeyMeta]);
inventory::collect!(StaticRegistry);

/// The assembled set of known keys with their metadata and default values.
#[derive(Debug, Clone)]
pub struct Registry {
    keys: HashMap<Key, &'static KeyMeta>,
    defaults: HashMap<Key, Option<Value>>,
}

impl Registry {
    /// Creates an empty registry with no keys.
    pub fn new_empty() -> Registry {
        Registry {
            keys: HashMap::new(),
            defaults: HashMap::new(),
        }
    }

    /// Creates a registry pre-populated from all link-time [`StaticRegistry`] submissions.
    pub fn new() -> Registry {
        let mut rv = Registry::new_empty();
        for entry in inventory::iter::<StaticRegistry> {
            rv.insert_multi(entry.0.iter());
        }
        rv
    }

    pub fn insert(&mut self, key_meta: &'static KeyMeta) {
        self.defaults.insert(key_meta.key, (key_meta.make_default)());
        self.keys.insert(key_meta.key, key_meta);
    }

    pub fn insert_multi(&mut self, iter: impl Iterator<Item = &'static KeyMeta>) {
        let (_, upper) = iter.size_hint();
        if let Some(l) = upper {
            self.defaults.reserve(l);
            self.keys.reserve(l);
        }
        iter.for_each(|v| self.insert(v));
    }

    pub fn iter(&self) -> impl Iterator<Item = &KeyMeta> {
        self.keys.values().copied()
    }

    pub fn get(&self, key: &Key) -> Option<&'static KeyMeta> {
        self.keys.get(key).copied()
    }

    pub fn default_value_of(&self, key: &Key) -> Option<&Value> {
        self.defaults.get(key)?.as_ref()
    }

    pub fn remove(&mut self, key: &Key) {
        self.keys.remove(key);
        self.defaults.remove(key);
    }

    /// Validates a single value against the descriptor of `key`.
    ///
    /// Returns an error if `key` is unknown or the value violates its descriptor.
    pub fn validate_value(&self, key: &Key, value: &Value) -> Result<()> {
        let meta = self
            .get(key)
            .ok_or_else(|| dicom_err!(InvalidData, "unknown configuration key {}::{}", key.module, key.code))?;
        meta.value_meta
            .validate(value)
            .err_context_with(|| format!("for key {}", meta.concept.name))
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
