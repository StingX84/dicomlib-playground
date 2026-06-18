use super::{Value, ValueFile, meta::FileType, meta::KeyMeta, meta::ValueMeta};
use crate::{DicomError, ErrContext, Result, dicom_err};

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq)]
struct DisplayDuration(std::time::Duration);

impl std::fmt::Display for DisplayDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.0.as_millis())
    }
}

pub struct Validator<'a> {
    pub key_meta: &'a KeyMeta,
    pub value_meta: &'a ValueMeta,
    pub vec_index: Option<usize>,
    pub map_key: Option<&'a Value>,
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
        self.validate_nullable(value).map_err(|e| self.extend_error(e))?;
        self.validate_value(value).map_err(|e| self.extend_error(e))?;

        Ok(())
    }

    fn validate_nullable(&self, value: &Value) -> Result {
        if let Value::Null = value
            && !self.key_meta.nullable
        {
            Err(dicom_err!(Configuration, "not nullable"))
        } else {
            Ok(())
        }
    }

    fn validate_value(&self, value: &Value) -> Result {
        match (&self.value_meta, value) {
            (ValueMeta::Bool, Value::Bool(_)) => Ok(()),

            (
                ValueMeta::String {
                    regexp,
                    min_length,
                    max_length,
                    ..
                },
                Value::String(s),
            ) => {
                let len = s.chars().count();
                Validator::check_range("length", len, min_length, max_length)?;

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

            (ValueMeta::Int { min, max }, Value::Int(n)) => Validator::check_range("integer", *n, min, max),

            (ValueMeta::Enum { one_of }, Value::Enum(n)) => {
                if one_of.iter().any(|(code, ..)| code == *n) {
                    Ok(())
                } else {
                    Err(dicom_err!(Configuration, "value {n} is not a valid enum choice"))
                }
            }

            (ValueMeta::Duration { min, max }, Value::Duration(d)) => Validator::check_range(
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

            (ValueMeta::Vr { one_of }, Value::Vr(vr)) => match one_of {
                Some(allowed) if !allowed.iter().any(|candidate| candidate == *vr) => {
                    Err(dicom_err!(Configuration, "VR {vr} is not among the allowed VRs"))
                }
                _ => Ok(()),
            },

            (
                ValueMeta::File {
                    ty,
                    allow_relative,
                    allow_content,
                    allow_reload,
                },
                Value::File(f),
            ) => {
                match f {
                    ValueFile::Content(..) => {
                        if !*allow_content {
                            return Err(dicom_err!(Configuration, "inline file content is not allowed here"));
                        }
                    }
                    ValueFile::Name { path, auto_reload } => {
                        if !*allow_relative && std::path::Path::new(path).is_relative() {
                            return Err(dicom_err!(Configuration, "relative paths are not allowed here"));
                        }
                        if *auto_reload && !*allow_reload {
                            return Err(dicom_err!(Configuration, "auto-reload is not allowed here"));
                        }
                        match ty {
                            FileType::ExistingFilePath => {
                                let p = std::path::Path::new(path);
                                if !p.is_file() {
                                    return Err(dicom_err!(
                                        Configuration,
                                        "path {path} does not point to an existing file"
                                    ));
                                }
                            }
                            FileType::ExistingDirPath => {
                                let p = std::path::Path::new(path);
                                if !p.is_dir() {
                                    return Err(dicom_err!(
                                        Configuration,
                                        "path {path} does not point to an existing directory"
                                    ));
                                }
                            }
                            FileType::FilePath => {
                                let p = std::path::Path::new(path);
                                if p.exists() && !p.is_file() {
                                    return Err(dicom_err!(Configuration, "path {path} exists but is not a file"));
                                }
                            }
                            FileType::DirPath => {
                                let p = std::path::Path::new(path);
                                if p.exists() && !p.is_dir() {
                                    return Err(dicom_err!(Configuration, "path {path} exists but is not a directory"));
                                }
                            }
                            FileType::GlobPattern => {
                                glob::Pattern::new(path)
                                    .map_err(|e| dicom_err!(Configuration, "invalid glob pattern {path:?}: {e}"))?;
                                if path.is_empty() {
                                    return Err(dicom_err!(Configuration, "glob pattern cannot be empty"));
                                }
                            }
                        }
                    }
                }
                Ok(())
            }

            (ValueMeta::Object { validate, .. }, Value::Object(conf)) => validate(conf),

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
                Validator::check_range("vector", len, min_length, max_length)?;
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
                        value_meta: items,
                        vec_index: Some(idx),
                        parent: Some(self),
                        ..*self
                    };
                    sub_stack.validate(element)?
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
                Validator::check_range("map", entries.len(), min_length, max_length)?;
                for (k, v) in entries.iter() {
                    let sub_stack = Validator {
                        value_meta: keys,
                        parent: Some(self),
                        ..*self
                    };
                    sub_stack.validate(k).err_context("invalid map key")?;

                    let sub_stack = Validator {
                        map_key: Some(k),
                        parent: Some(self),
                        value_meta: values,
                        ..*self
                    };
                    sub_stack.validate(v).err_context("invalid map value")?;
                }
                Ok(())
            }

            // Complex values are application-defined; the registered codec is the
            // only thing that understands the concrete type, so delegate to it.
            (ValueMeta::Complex { ty, .. }, Value::Complex(any)) => ty.validate(any.as_ref()),

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
