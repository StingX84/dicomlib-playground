use super::{File, Value, meta::KeyMeta, meta::ValueMeta};
use crate::{DicomError, ErrContext, Result, config::map::DEFAULT_CONDITION, dicom_err, ensure};

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
struct DisplayDuration(std::time::Duration);

impl std::fmt::Display for DisplayDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.0.as_millis())
    }
}

pub struct Validator<'a> {
    pub key_meta: &'a KeyMeta,
    pub vec_index: Option<usize>,
    pub value_meta: &'a ValueMeta,
    pub map_key: Option<&'a str>,
    pub file: Option<(&'a str, usize)>,
    pub parent: Option<&'a Validator<'a>>,
}

impl<'a> std::fmt::Display for Validator<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(parent) = self.parent {
            write!(f, "{}.", parent)?;
        }
        write!(f, "{}", self.key_meta)?;
        if let Some(i) = self.vec_index {
            write!(f, "[{}]", i)?
        }
        if let Some(key) = &self.map_key {
            write!(f, "[{:?}]", key)?
        }
        Ok(())
    }
}

impl<'a> Validator<'a> {
    pub fn validate(&self, value: &Value) -> Result {
        if matches!(value, Value::Null) {
            ensure!(self.value_meta.is_optional(), Configuration, "not optional");
            return Ok(());
        }
        self.validate_value(value).map_err(|e| self.extend_error(e))?;

        Ok(())
    }

    fn validate_value(&self, value: &Value) -> Result {
        match (&self.value_meta, value) {
            (ValueMeta::Bool { .. }, Value::Bool(_)) => Ok(()),

            (
                ValueMeta::String {
                    regexp,
                    min: min_length,
                    max: max_length,
                    ..
                },
                Value::String(s),
            ) => {
                let len = s.chars().count();
                Validator::check_range("length", len, min_length, max_length)?;

                if let Some(pattern) = regexp {
                    let re = regex::Regex::new(pattern)
                        .map_err(|e| dicom_err!(Internal, "invalid validation regex {pattern:?}: {e}"))?;
                    ensure!(
                        re.is_match(s),
                        Configuration,
                        "value {s:?} does not match required pattern {pattern:?}"
                    );
                }
                Ok(())
            }

            (ValueMeta::Int { min, max, .. }, Value::Int(n)) => Validator::check_range("integer", *n, min, max),

            (ValueMeta::Enum { one_of, .. }, Value::Enum(n)) => {
                if one_of.iter().any(|(code, ..)| code == *n) {
                    Ok(())
                } else {
                    Err(dicom_err!(Configuration, "value {n} is not a valid enum choice"))
                }
            }

            (ValueMeta::Duration { min, max, .. }, Value::Duration(d)) => Validator::check_range(
                "duration(ms)",
                DisplayDuration(*d),
                &min.map(DisplayDuration),
                &max.map(DisplayDuration),
            ),

            (ValueMeta::Tag { one_of, .. }, Value::Tag(t)) => match one_of {
                Some(allowed) if !allowed.iter().any(|candidate| candidate == *t) => {
                    Err(dicom_err!(Configuration, "tag {t} is not among the allowed tags"))
                }
                _ => Ok(()),
            },

            (ValueMeta::Vr { one_of, .. }, Value::Vr(vr)) => match one_of {
                Some(allowed) if !allowed.iter().any(|candidate| candidate == *vr) => {
                    Err(dicom_err!(Configuration, "VR {vr} is not among the allowed VRs"))
                }
                _ => Ok(()),
            },

            #[cfg(feature = "uuid")]
            (ValueMeta::Uuid { non_zero, .. }, Value::Uuid(u)) => {
                ensure!(
                    !*non_zero || !u.is_nil(),
                    Configuration,
                    "UUID must not be the nil UUID"
                );
                Ok(())
            }

            (
                ValueMeta::File {
                    allow_content,
                    allow_dir,
                    allow_file,
                    allow_glob,
                    hot_reload: meta_hot_reload,
                    should_exist,
                    should_not_exist,
                    ..
                },
                Value::File(f),
            ) => {
                match f {
                    File::Content(..) => {
                        ensure!(*allow_content, Configuration, "inline file content is not allowed here");
                    }
                    File::Name { path, hot_reload } => {
                        ensure!(!path.is_empty(), Configuration, "file path cannot be empty");
                        ensure!(
                            !*hot_reload || *meta_hot_reload,
                            Configuration,
                            "hot-reload is not allowed here"
                        );
                        if *allow_glob {
                            let paths = glob::glob(path)
                                .map_err(|e| dicom_err!(Configuration, "invalid glob pattern {path:?}: {e}"))?;
                            for f in paths.into_iter() {
                                let path = f.map_err(|e| dicom_err!(Io, "failed to read glob path: {e}"))?;
                                ensure!(
                                    path.is_absolute(),
                                    Configuration,
                                    "glob pattern {path:?} matches relative path {path:?}"
                                );
                                if !*allow_dir {
                                    ensure!(
                                        !path.is_dir(),
                                        Configuration,
                                        "glob pattern {path:?} matches a directory, which is not allowed here"
                                    );
                                }
                                if !*allow_file {
                                    ensure!(
                                        !path.is_file(),
                                        Configuration,
                                        "glob pattern {path:?} matches a file, which is not allowed here"
                                    );
                                }
                            }
                        } else {
                            let path = std::path::Path::new(path);
                            ensure!(path.is_absolute(), Configuration, "file path {path:?} must be absolute");
                            if path.exists() {
                                ensure!(!*should_not_exist, Configuration, "file path {path:?} must not exist");
                            } else {
                                ensure!(!*should_exist, Configuration, "file path {path:?} must exist");
                            }
                            if !*allow_dir {
                                ensure!(
                                    !path.is_dir(),
                                    Configuration,
                                    "path {path:?} points to a directory, which is not allowed here"
                                );
                            }
                            if !*allow_file {
                                ensure!(
                                    !path.is_file(),
                                    Configuration,
                                    "path {path:?} points to a file, which is not allowed here"
                                );
                            }
                        }
                    }
                }
                Ok(())
            }

            (
                ValueMeta::Network {
                    domain,
                    unix,
                    ipv4,
                    ipv6,
                    ..
                },
                Value::Network(network),
            ) => {
                match network.definition {
                    crate::network::NetworkDefinition::HostName { .. } if !*domain && !*ipv4 && !*ipv6 => {
                        return Err(dicom_err!(Configuration, "host addresses are not allowed here"));
                    }
                    crate::network::NetworkDefinition::UnixSocket(_) if !*unix => {
                        return Err(dicom_err!(Configuration, "Unix socket addresses are not allowed here"));
                    }
                    crate::network::NetworkDefinition::Ip { addr, .. } => match addr {
                        std::net::IpAddr::V4(_) if !*ipv4 => {
                            return Err(dicom_err!(Configuration, "IPv4 addresses are not allowed here"));
                        }
                        std::net::IpAddr::V6(_) if !*ipv6 => {
                            return Err(dicom_err!(Configuration, "IPv6 addresses are not allowed here"));
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Ok(())
            }

            (
                ValueMeta::Host {
                    domain,
                    unix,
                    ipv4,
                    ipv6,
                    ..
                },
                Value::Host(host),
            ) => {
                match host.definition {
                    crate::network::HostDefinition::HostName { .. } if !*domain && !*ipv4 && !*ipv6 => {
                        return Err(dicom_err!(Configuration, "host addresses are not allowed here"));
                    }
                    crate::network::HostDefinition::UnixSocket { .. } if !*unix => {
                        return Err(dicom_err!(Configuration, "Unix socket addresses are not allowed here"));
                    }
                    crate::network::HostDefinition::Ip { addr, .. } => match addr {
                        std::net::IpAddr::V4(_) if !*ipv4 => {
                            return Err(dicom_err!(Configuration, "IPv4 addresses are not allowed here"));
                        }
                        std::net::IpAddr::V6(_) if !*ipv6 => {
                            return Err(dicom_err!(Configuration, "IPv6 addresses are not allowed here"));
                        }
                        _ => {}
                    },
                    _ => {}
                }
                Ok(())
            }

            (ValueMeta::Object { meta, .. }, Value::Object(obj)) => {
                for (key, conditionals) in obj.values().iter() {
                    ensure!(
                        std::ptr::eq(meta(), obj.object_meta()),
                        Configuration,
                        "object has unexpected field {key:?}"
                    );
                    let Some(key_meta) = obj.object_meta.key_meta(key) else {
                        return Err(dicom_err!(Configuration, "object has unexpected field {key:?}"));
                    };

                    let sub_stack = Validator {
                        key_meta,
                        value_meta: &key_meta.value_meta,
                        vec_index: None,
                        map_key: None,
                        file: self.file,
                        parent: Some(self),
                    };
                    for (value, cond) in conditionals.0.iter() {
                        ensure!(
                            key_meta.conditional || cond == &DEFAULT_CONDITION,
                            Configuration,
                            "object field {key:?} is not conditional but has a condition"
                        );
                        sub_stack.validate(value).err_context("invalid object field")?;
                    }
                }
                Ok(())
            }

            (
                ValueMeta::Vec {
                    meta, min, max, stride, ..
                },
                Value::Vec(elements),
            ) => {
                let len = elements.len();
                Validator::check_range("vector", len, min, max)?;
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
                    let sub_stack = Validator {
                        value_meta: meta,
                        vec_index: Some(idx),
                        parent: Some(self),
                        ..*self
                    };
                    sub_stack.validate(element)?
                }
                Ok(())
            }

            (ValueMeta::Map { meta, min, max, .. }, Value::Map(entries)) => {
                Validator::check_range("map", entries.len(), min, max)?;
                for (k, v) in entries.iter() {
                    let sub_stack = Validator {
                        map_key: Some(k),
                        parent: Some(self),
                        value_meta: meta,
                        ..*self
                    };
                    sub_stack.validate(v).err_context("invalid map value")?;
                }
                Ok(())
            }

            // Custom values are application-defined; the registered codec is the
            // only thing that understands the concrete type, so delegate to it.
            #[cfg(feature = "serde")]
            (ValueMeta::Custom { ty, .. }, Value::Custom(any)) => ty.validate(any.as_ref()),

            (key, value) => Err(dicom_err!(
                Internal,
                "type mismatch: value of kind {} does not fit a {} descriptor",
                value.kind_name(),
                key.kind_name()
            )),
        }
    }

    fn extend_error(&self, mut e: DicomError) -> DicomError {
        let mut msg = match e.message {
            Some(existing) => format!("key {self}: {existing}"),
            None => format!("key {self}: {}", e.kind),
        };
        if let Some(file) = self.file {
            msg = format!("{} (file {}:{})", msg, file.0, file.1);
        }
        e.message = Some(msg);
        e
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
                "{} {} outside of required range {}..{}",
                what,
                value,
                *min,
                *max
            )),
            (Some(min), None) if value < *min => Err(dicom_err!(
                Configuration,
                "{} {} < required minimum {}",
                what,
                value,
                *min
            )),
            (None, Some(max)) if value > *max => Err(dicom_err!(
                Configuration,
                "{} {} > allowed maximum {}",
                what,
                value,
                *max
            )),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde")]
    use super::super::CustomType;
    use super::super::{Key, meta::*};
    use super::*;
    use crate::Arc;
    #[cfg(feature = "serde")]
    use serde_json::Value as JsonValue;
    use std::any::Any;

    fn validate(meta: &ValueMeta, value: &Value) -> crate::error::Result<()> {
        let key_meta = KeyMetaBuilder::new(Key::new("test"), meta.clone()).runtime().build();
        let stack = Validator {
            key_meta: &key_meta,
            value_meta: meta,
            vec_index: None,
            map_key: None,
            file: None,
            parent: None,
        };
        stack.validate(value)
    }

    #[test]
    fn string_validation_respects_length_and_pattern() {
        let meta = build::String::new().regexp(r"^[A-Z]+$").min(2).max(4).build();
        assert!(validate(&meta, &Value::String("ABC".into())).is_ok());
        // too short
        assert!(validate(&meta, &Value::String("A".into())).is_err());
        // too long
        assert!(validate(&meta, &Value::String("ABCDE".into())).is_err());
        // pattern mismatch
        assert!(validate(&meta, &Value::String("abc".into())).is_err());
    }

    #[test]
    fn int_range_is_enforced() {
        let meta = build::Int::new().min(0).max(10).build();
        assert!(validate(&meta, &Value::Int(5)).is_ok());
        assert!(validate(&meta, &Value::Int(-1)).is_err());
        assert!(validate(&meta, &Value::Int(11)).is_err());
    }

    #[test]
    fn enum_membership_is_checked() {
        static CHOICES: [(u32, &str, Option<EnumVisual>); 2] = [
            (
                1,
                "a",
                Some(EnumVisual {
                    display_name: "a",
                    help: None,
                }),
            ),
            (
                2,
                "b",
                Some(EnumVisual {
                    display_name: "b",
                    help: None,
                }),
            ),
        ];
        let meta = build::Enum::new(Choices::Static(&CHOICES)).build();
        assert!(validate(&meta, &Value::Enum(1)).is_ok());
        assert!(validate(&meta, &Value::Enum(3)).is_err());
    }

    #[test]
    fn type_mismatch_is_rejected() {
        let meta = build::Bool::new().build();
        let err = validate(&meta, &Value::Int(1)).unwrap_err();
        assert_eq!(err.kind, crate::ErrorKind::Internal);
    }

    #[test]
    fn vec_validates_each_element() {
        static ITEM: ValueMeta = build::Int::new().min(0).build();
        let meta = build::Vec::new(&ITEM).min(1).max(3).build();
        assert!(validate(&meta, &Value::Vec(vec![Value::Int(1), Value::Int(2)])).is_ok());
        // element out of range
        assert!(validate(&meta, &Value::Vec(vec![Value::Int(-1)])).is_err());
        // too many items
        let many4 = Value::Vec(vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)]);
        assert!(validate(&meta, &many4).is_err());
        // stride is one
        let meta = build::Vec::new(&ITEM).stride(1).build();
        assert!(validate(&meta, &many4).is_ok());
        let many3 = Value::Vec(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert!(validate(&meta, &many3).is_ok());
        let empty = Value::Vec(Vec::new());
        assert!(validate(&meta, &empty).is_ok());
        // stride is even
        let meta = build::Vec::new(&ITEM).stride(2).build();
        assert!(validate(&meta, &many4).is_ok());
        assert!(validate(&meta, &many3).is_err());
        assert!(validate(&meta, &empty).is_ok());
    }

    // ── Custom application-defined types ─────────────────────────────────────

    #[cfg(feature = "serde")]
    #[derive(Debug, PartialEq)]
    struct Port(u16);

    #[cfg(feature = "serde")]
    struct PortType;
    #[cfg(feature = "serde")]
    impl CustomType for PortType {
        fn name(&self) -> &'static str {
            "port"
        }
        fn decode(&self, node: &JsonValue) -> crate::error::Result<Arc<dyn Any + Send + Sync>> {
            let n = node
                .as_i64()
                .ok_or_else(|| crate::dicom_err!(InvalidData, "port expects an integer"))?;
            Ok(Arc::new(Port(
                u16::try_from(n).map_err(|_| crate::dicom_err!(InvalidData, "port out of range"))?,
            )))
        }
        fn encode(&self, value: &dyn Any) -> crate::error::Result<JsonValue> {
            let p = value
                .downcast_ref::<Port>()
                .ok_or_else(|| crate::dicom_err!(Internal, "port got wrong value type"))?;
            Ok(JsonValue::from(p.0))
        }
        fn validate(&self, value: &dyn Any) -> crate::error::Result<()> {
            let p = value
                .downcast_ref::<Port>()
                .ok_or_else(|| crate::dicom_err!(Internal, "port got wrong value type"))?;
            if p.0 == 0 {
                return Err(crate::dicom_err!(InvalidData, "port must not be zero"));
            }
            Ok(())
        }
    }

    #[cfg(feature = "serde")]
    static PORT_TYPE: PortType = PortType;

    #[cfg(feature = "serde")]
    #[test]
    fn custom_type_round_trips_through_json() {
        let ty: &'static dyn CustomType = &PORT_TYPE;
        let decoded = ty.decode(&JsonValue::from(104)).unwrap();
        assert_eq!(decoded.downcast_ref::<Port>(), Some(&Port(104)));
        assert_eq!(ty.encode(decoded.as_ref()).unwrap(), JsonValue::from(104));
    }

    // ── File ─────────────────────────────────────────────────────────────────

    fn name_file(path: &str, hot_reload: bool) -> Value {
        Value::File(File::Name {
            path: path.to_owned(),
            hot_reload,
        })
    }

    #[test]
    fn file_content_is_gated_by_allow_content() {
        let content = Value::File(File::Content(vec![1, 2, 3]));

        let allowed = build::File::new().allow_content().build();
        assert!(validate(&allowed, &content).is_ok());

        let denied = build::File::new().allow_file().build();
        assert!(validate(&denied, &content).is_err());
    }

    #[test]
    fn file_path_must_be_non_empty_and_absolute() {
        let meta = build::File::new().allow_dir().allow_file().build();
        assert!(validate(&meta, &name_file("", false)).is_err());
        assert!(validate(&meta, &name_file("relative/path", false)).is_err());
    }

    #[test]
    fn file_hot_reload_requires_meta_permission() {
        // A value asking for hot-reload against a meta that forbids it.
        let no_reload = build::File::new().allow_dir().allow_file().build();
        assert!(validate(&no_reload, &name_file("/some/absolute", true)).is_err());

        let with_reload = build::File::new().allow_dir().allow_file().hot_reload().build();
        assert!(validate(&with_reload, &name_file("/some/absolute", true)).is_ok());
    }

    #[test]
    fn file_existence_constraints_are_enforced() {
        let dir = std::env::temp_dir().join(format!("dpx_file_validate_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let existing = dir.join("present.txt");
        std::fs::write(&existing, b"x").unwrap();
        let missing = dir.join("absent.txt");

        let must_exist = build::File::new().allow_file().should_exist().build();
        assert!(validate(&must_exist, &name_file(existing.to_str().unwrap(), false)).is_ok());
        assert!(validate(&must_exist, &name_file(missing.to_str().unwrap(), false)).is_err());

        let must_not_exist = build::File::new().allow_file().should_not_exist().build();
        assert!(validate(&must_not_exist, &name_file(missing.to_str().unwrap(), false)).is_ok());
        assert!(validate(&must_not_exist, &name_file(existing.to_str().unwrap(), false)).is_err());

        // A directory where only files are allowed is rejected.
        let file_only = build::File::new().allow_file().build();
        assert!(validate(&file_only, &name_file(dir.to_str().unwrap(), false)).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn custom_value_meta_delegates_validation_to_type() {
        let meta = build::Custom::new(&PORT_TYPE).build();
        let good: Arc<dyn Any + Send + Sync> = Arc::new(Port(104));
        assert!(validate(&meta, &Value::Custom(good)).is_ok());

        let bad: Arc<dyn Any + Send + Sync> = Arc::new(Port(0));
        assert!(validate(&meta, &Value::Custom(bad)).is_err());
    }
}
