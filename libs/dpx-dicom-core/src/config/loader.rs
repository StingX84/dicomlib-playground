//! Streaming, schema-driven YAML configuration loader.
//!
//! The loader does not build a whole-document DOM. Instead it drives
//! `serde-saphyr` with [`serde::de::DeserializeSeed`]s guided by the
//! [`ObjectMeta`]: as the parser walks the document, each key is routed against a
//! path index built from the registered keys' [`Key`](super::Key) paths and their
//! [`conditional`](super::meta::KeyMeta::conditional)/[`runtime`](super::meta::KeyMeta::runtime)
//! flags, and each value is mapped to a [`Value`] according to its [`ValueMeta`].
//!
//! Because errors are raised *during* deserialization, `serde-saphyr` stamps
//! them with a precise `line:column`; the loader prepends the file name to
//! produce `file:line:column` diagnostics.
//!
//! ## YAML shape
//!
//! A key's [`Key`](super::Key) path is a dotted path. For plain (non-conditional)
//! keys the path leads straight to the value:
//!
//! ```yaml
//! dicom:
//!   association:
//!     artim_timeout: 10s
//! ```
//!
//! For conditional keys, all but the last segment lead to a *list of objects*;
//! the last segment is the setting key inside an element, which may carry a
//! `when` filter:
//!
//! ```yaml
//! dicom:
//!   association:
//!     - artim_timeout: 10s
//!     - artim_timeout: 5s
//!       when: { peer_aet: PEER }
//! ```

use super::map::{Condition, Map};
use super::meta::{KeyMeta, ValueMeta, ObjectMeta};
use super::value::ConfiguredFile;
use super::{Object, LayerId, OBJECT_LAYER_ID, SubstVars, Value};
use crate::IntoDicomErr;
use crate::network::{HostDefinition, Network, NetworkDefinition};
use crate::{HashMap, dicom_err, ensure, error::Result};

use serde::Deserialize;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::path::{Path, PathBuf};

pub const CONFIG_LAYER_ID: LayerId = LayerId::Borrowed("file");

/// Loads configuration from a YAML file or a directory of `*.yml` files.
pub struct YamlLoader {
    registry: &'static ObjectMeta,
}

impl YamlLoader {
    /// Creates a loader bound to a metadata `object`
    pub fn new(registry: &'static ObjectMeta) -> YamlLoader {
        YamlLoader { registry }
    }

    /// Loads configuration from `path`.
    ///
    /// If `path` is a file, it is the entire configuration. If it is a
    /// directory, every `*.yml` file in it is loaded in alphabetical order, with
    /// later files overriding earlier ones (last value wins).
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Object> {
        let files = collect_files(path.as_ref())?;
        ensure!(
            !files.is_empty(),
            NotFound,
            "no configuration found at {}",
            path.as_ref().display()
        );

        let index = build_index(self.registry)?;
        let acc = Accumulator {
            map: RefCell::new(Map::new()),
        };

        for file in &files {
            let text = std::fs::read_to_string(file).to_dicom_err_with(|| format!("cannot read {}", file.display()))?;
            let ctx = LoadCtx {
                index: &index,
                acc: &acc,
            };
            parse_document(&text, &file.display().to_string(), &ctx)?;
        }

        self.finalize(acc)
    }

    /// Loads configuration from a single in-memory YAML document.
    pub fn load_str(&self, text: &str) -> Result<Object> {
        let index = build_index(self.registry)?;
        let acc = Accumulator {
            map: RefCell::new(Map::new()),
        };
        let ctx = LoadCtx {
            index: &index,
            acc: &acc,
        };
        parse_document(text, "<memory>", &ctx)?;
        self.finalize(acc)
    }

    fn finalize(&self, acc: Accumulator) -> Result<Object> {
        let values = acc.map.into_inner();
        // The root document is itself an object: once every file has merged into
        // `values`, every required top-level key must be present. Unlike a value
        // error, a missing key has no source position, so this carries no
        // `line:column`.
        check_required(self.registry, &values).map_err(|m| dicom_err!(Configuration, "{m}"))?;
        Ok(Object::new(CONFIG_LAYER_ID, self.registry, values))
    }
}

fn collect_files(path: &Path) -> Result<Vec<PathBuf>> {
    let meta = std::fs::metadata(path).to_dicom_err_with(|| format!("cannot access {}", path.display()))?;
    if meta.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(path)
        .to_dicom_err_with(|| format!("cannot read directory {}", path.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "yml"))
        .collect();
    files.sort();
    Ok(files)
}

// ── Path index ──────────────────────────────────────────────────────────────

/// A node of the path index built from the registry's dotted store names.
enum IndexNode<'a> {
    /// A structural object grouping nested keys by segment.
    Branch(HashMap<String, IndexNode<'a>>),
    /// A non-conditional leaf setting.
    Leaf(&'a KeyMeta),
    /// A conditional list base: setting keys allowed inside its elements.
    CondList(HashMap<String, &'a KeyMeta>),
}

fn build_index(registry: &ObjectMeta) -> Result<IndexNode<'_>> {
    let mut root = IndexNode::Branch(HashMap::new());
    for km in registry.iter() {
        if km.runtime {
            continue;
        }
        let segments: Vec<&str> = km.key.0.split('.').collect();
        ensure!(
            !segments.iter().any(|s| s.is_empty()),
            Configuration,
            "empty path segment in {:?}",
            km.key.0
        );
        if km.conditional {
            insert_conditional(&mut root, &segments, km)?;
        } else {
            insert_leaf(&mut root, &segments, km)?;
        }
    }
    Ok(root)
}

/// Recursively descends/creates `Branch` nodes along `path`, returning the
/// branch map at its end. Written recursively to avoid the loop-re borrow that
/// trips the borrow checker.
fn branch_descend<'a, 'n>(
    node: &'n mut IndexNode<'a>,
    path: &[&str],
) -> Result<&'n mut HashMap<String, IndexNode<'a>>> {
    let IndexNode::Branch(map) = node else {
        return Err(dicom_err!(
            Configuration,
            "configuration path conflicts with a non-object key"
        ));
    };
    match path.split_first() {
        None => Ok(map),
        Some((head, rest)) => {
            let child = map
                .entry((*head).to_string())
                .or_insert_with(|| IndexNode::Branch(HashMap::new()));
            branch_descend(child, rest)
        }
    }
}

fn insert_leaf<'a>(root: &mut IndexNode<'a>, segments: &[&str], km: &'a KeyMeta) -> Result<()> {
    let (last, parents) = segments.split_last().expect("non-empty path");
    let map = branch_descend(root, parents)?;
    ensure!(!map.contains_key(*last), Configuration, "duplicate configuration path {last:?}");
    map.insert((*last).to_string(), IndexNode::Leaf(km));
    Ok(())
}

fn insert_conditional<'a>(root: &mut IndexNode<'a>, segments: &[&str], km: &'a KeyMeta) -> Result<()> {
    ensure!(
        segments.len() >= 2,
        Configuration,
        "conditional key {:?} needs a list path and a key",
        km.key.0
    );
    let (key_seg, list_path) = segments.split_last().expect("len >= 2");
    ensure!(
        *key_seg != "when",
        Configuration,
        "conditional key must not be named 'when' ({:?})",
        km.key.0
    );
    let (list_seg, parents) = list_path.split_last().expect("len >= 2");
    let map = branch_descend(root, parents)?;
    let entry = map
        .entry((*list_seg).to_string())
        .or_insert_with(|| IndexNode::CondList(HashMap::new()));
    match entry {
        IndexNode::CondList(keys) => {
            ensure!(!keys.contains_key(*key_seg), Configuration, "duplicate conditional key {key_seg:?}");
            keys.insert((*key_seg).to_string(), km);
            Ok(())
        }
        _ => Err(dicom_err!(
            Configuration,
            "conditional list {list_seg:?} conflicts with a non-conditional key"
        )),
    }
}

// ── Load context ────────────────────────────────────────────────────────────

struct Accumulator {
    map: RefCell<Map>,
}

struct LoadCtx<'a> {
    index: &'a IndexNode<'a>,
    acc: &'a Accumulator,
}

thread_local! {
    static CTX: Cell<*const ()> = const { Cell::new(std::ptr::null()) };
}

/// Installs `ctx` for the duration of `f`. The pointer is only read on the same
/// thread within this call, where `ctx` is guaranteed live.
fn with_ctx<T>(ctx: &LoadCtx<'_>, f: impl FnOnce() -> T) -> T {
    let prev = CTX.with(|c| c.replace(ctx as *const LoadCtx as *const ()));
    let rv = f();
    CTX.with(|c| c.set(prev));
    rv
}

/// SAFETY: only called from within [`with_ctx`]'s closure, where the installed
/// pointer refers to a live [`LoadCtx`] on this thread.
fn current_ctx<'a>() -> &'a LoadCtx<'a> {
    let ptr = CTX.with(|c| c.get()) as *const LoadCtx<'a>;
    unsafe { &*ptr }
}

fn parse_document(text: &str, file: &str, ctx: &LoadCtx<'_>) -> Result<()> {
    with_ctx(ctx, || {
        serde_saphyr::from_str::<Document>(text)
            .map(|_| ())
            .map_err(|e| dicom_err!(Configuration, "{file}: {e}"))
    })
}

// ── Root document ───────────────────────────────────────────────────────────

struct Document;

impl<'de> serde::Deserialize<'de> for Document {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let ctx = current_ctx();
        d.deserialize_map(MapWalk {
            branch: root_branch(ctx),
        })?;
        Ok(Document)
    }
}

fn root_branch<'a>(ctx: &'a LoadCtx<'a>) -> &'a HashMap<String, IndexNode<'a>> {
    match ctx.index {
        IndexNode::Branch(m) => m,
        _ => unreachable!("index root is always a branch"),
    }
}

/// What a routed map key resolves to. Resolving (and rejecting unknown keys)
/// happens inside key deserialization so `serde-saphyr` stamps the key's
/// `line:column` onto any error.
enum Routed<'a> {
    Branch(&'a HashMap<String, IndexNode<'a>>),
    Leaf(&'a KeyMeta),
    Cond(&'a HashMap<String, &'a KeyMeta>),
}

struct KeySeed<'a> {
    branch: &'a HashMap<String, IndexNode<'a>>,
}

impl<'de, 'a> DeserializeSeed<'de> for KeySeed<'a> {
    type Value = Routed<'a>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<Routed<'a>, D::Error> {
        d.deserialize_str(self)
    }
}

impl<'de, 'a> Visitor<'de> for KeySeed<'a> {
    type Value = Routed<'a>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a configuration key")
    }
    fn visit_str<E: de::Error>(self, key: &str) -> std::result::Result<Routed<'a>, E> {
        match self.branch.get(key) {
            Some(IndexNode::Branch(child)) => Ok(Routed::Branch(child)),
            Some(IndexNode::Leaf(km)) => Ok(Routed::Leaf(km)),
            Some(IndexNode::CondList(keys)) => Ok(Routed::Cond(keys)),
            None => Err(de::Error::custom(format!("unknown configuration key {key:?}"))),
        }
    }
}

fn validate_leaf(km: &KeyMeta, value: &Value) -> std::result::Result<(), String> {
    let stack = super::validator::Validator {
        key_meta: km,
        value_meta: &km.value_meta,
        vec_index: None,
        map_key: None,
        file: None,
        parent: None,
    };
    stack.validate(value).map_err(|e| format!("{e}"))
}

// ── Structural object walk (root and nested) ────────────────────────────────

struct MapWalk<'a> {
    branch: &'a HashMap<String, IndexNode<'a>>,
}

impl<'de, 'a> DeserializeSeed<'de> for MapWalk<'a> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de, 'a> Visitor<'de> for MapWalk<'a> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a configuration object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
        let ctx = current_ctx();
        while let Some(routed) = map.next_key_seed(KeySeed { branch: self.branch })? {
            match routed {
                Routed::Branch(child) => map.next_value_seed(MapWalk { branch: child })?,
                Routed::Leaf(km) => {
                    let value = map.next_value_seed(ValueSeed::new(&km.value_meta))?;
                    validate_leaf(km, &value).map_err(de::Error::custom)?;
                    ctx.acc.map.borrow_mut().add(km.key, value, None);
                }
                Routed::Cond(keys) => map.next_value_seed(CondListSeed { keys })?,
            }
        }
        Ok(())
    }
}

// ── Conditional list ────────────────────────────────────────────────────────

struct CondListSeed<'a> {
    keys: &'a HashMap<String, &'a KeyMeta>,
}

impl<'de, 'a> DeserializeSeed<'de> for CondListSeed<'a> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}

impl<'de, 'a> Visitor<'de> for CondListSeed<'a> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a list of conditional configuration entries")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<(), A::Error> {
        while seq.next_element_seed(CondElementSeed { keys: self.keys })?.is_some() {}
        Ok(())
    }
}

struct CondElementSeed<'a> {
    keys: &'a HashMap<String, &'a KeyMeta>,
}

impl<'de, 'a> DeserializeSeed<'de> for CondElementSeed<'a> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de, 'a> Visitor<'de> for CondElementSeed<'a> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a conditional entry with one setting and an optional 'when'")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
        let ctx = current_ctx();
        let mut setting: Option<(&KeyMeta, Value)> = None;
        let mut filter = Condition::default();

        while let Some(key) = map.next_key::<String>()? {
            if key == "when" {
                filter = map.next_value_seed(FilterSeed)?;
            } else if let Some(km) = self.keys.get(&key) {
                if setting.is_some() {
                    return Err(de::Error::custom(
                        "a conditional entry must describe exactly one setting",
                    ));
                }
                let value = map.next_value_seed(ValueSeed::new(&km.value_meta))?;
                validate_leaf(km, &value).map_err(de::Error::custom)?;
                setting = Some((km, value));
            } else {
                return Err(de::Error::custom(format!(
                    "unexpected key {key:?} in conditional entry"
                )));
            }
        }

        let (km, value) = setting.ok_or_else(|| de::Error::custom("conditional entry has no known setting key"))?;
        ctx.acc.map.borrow_mut().add(km.key, value, Some(filter));
        Ok(())
    }
}

struct FilterSeed;

impl<'de> DeserializeSeed<'de> for FilterSeed {
    type Value = Condition;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<Condition, D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for FilterSeed {
    type Value = Condition;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 'when' filter object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Condition, A::Error> {
        let mut filter = Condition::default();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "is_tls_used" => filter.is_tls_used = Some(map.next_value()?),
                "is_incoming" => filter.is_incoming = Some(map.next_value()?),
                "is_virtual" => filter.is_virtual = Some(map.next_value()?),
                "peer_aet" => filter.peer_aet = Some(map.next_value()?),
                "local_aet" => filter.local_aet = Some(map.next_value()?),
                "peer_network" => filter.peer_network = Some(parse_ip(&map.next_value::<String>()?)?),
                "local_network" => filter.local_network = Some(parse_ip(&map.next_value::<String>()?)?),
                other => {
                    return Err(de::Error::custom(format!("unknown 'when' filter {other:?}")));
                }
            }
        }
        Ok(filter)
    }
}

fn parse_ip<E: de::Error>(s: &str) -> std::result::Result<Network, E> {
    let definition = s
        .parse::<NetworkDefinition>()
        .map_err(|e| de::Error::custom(format!("{e}")))?;
    definition.resolve_sync().map_err(|e| de::Error::custom(format!("{e}")))
}

// ── Value mapping ───────────────────────────────────────────────────────────

struct ValueSeed<'a> {
    meta: &'a ValueMeta,
}

impl<'a> ValueSeed<'a> {
    fn new(meta: &'a ValueMeta) -> ValueSeed<'a> {
        ValueSeed { meta }
    }
}

impl<'de, 'a> DeserializeSeed<'de> for ValueSeed<'a> {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<Value, D::Error> {
        if let ValueMeta::Custom { ty, .. } = self.meta {
            let node = serde_json::Value::deserialize(d)?;
            return ty
                .decode(&node)
                .map(Value::Custom)
                .map_err(|e| de::Error::custom(format!("{e}")));
        }
        d.deserialize_any(MetaVisitor { meta: self.meta })
    }
}

struct MetaVisitor<'a> {
    meta: &'a ValueMeta,
}

impl<'a> MetaVisitor<'a> {
    fn mismatch<E: de::Error>(&self, got: &str) -> E {
        de::Error::custom(format!("expected {}, found {got}", self.meta.kind_name()))
    }
}

impl<'de, 'a> Visitor<'de> for MetaVisitor<'a> {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a {} configuration value", self.meta.kind_name())
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Value, E> {
        match self.meta {
            ValueMeta::Bool { .. } => Ok(Value::Bool(v)),
            ValueMeta::Vec { .. } => Ok(Value::Vec(vec![Value::Bool(v)])),
            _ => Err(self.mismatch("a boolean")),
        }
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Value, E> {
        match self.meta {
            ValueMeta::Int { .. } => Ok(Value::Int(v)),
            ValueMeta::Vec { meta, .. } => Ok(Value::Vec(vec![scalar_int(meta, v)?])),
            _ => Err(self.mismatch("an integer")),
        }
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Value, E> {
        let n = i64::try_from(v).map_err(|_| de::Error::custom("integer too large"))?;
        self.visit_i64(n)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Value, E> {
        scalar_str(self.meta, v).map_err(|e| match e {
            ScalarErr::Mismatch => self.mismatch("a string"),
            ScalarErr::Custom(m) => de::Error::custom(m),
        })
    }

    fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Value, E> {
        self.visit_str(&v)
    }

    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        if self.meta.is_nullable() {
            Ok(Value::Null)
        } else {
            Err(de::Error::custom("value must not be null"))
        }
    }

    fn visit_none<E: de::Error>(self) -> std::result::Result<Value, E> {
        self.visit_unit()
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Value, A::Error> {
        let ValueMeta::Vec { meta, .. } = self.meta else {
            return Err(self.mismatch("a list"));
        };
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(ValueSeed::new(meta))? {
            out.push(v);
        }
        Ok(Value::Vec(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> std::result::Result<Value, A::Error> {
        match self.meta {
            ValueMeta::Object { meta, .. } => visit_object(meta(), map),
            ValueMeta::Map { meta, .. } => visit_value_map(meta, map),
            ValueMeta::File { .. } => visit_file_map(map),
            _ => Err(self.mismatch("an object")),
        }
    }
}

fn scalar_int<E: de::Error>(items: &ValueMeta, v: i64) -> std::result::Result<Value, E> {
    match items {
        ValueMeta::Int { .. } => Ok(Value::Int(v)),
        _ => Err(de::Error::custom("list item is not an integer")),
    }
}

enum ScalarErr {
    Mismatch,
    Custom(String),
}

/// Maps a YAML scalar string to a [`Value`] according to `meta`.
///
/// Fields whose meta opts into substitution have `$VAR`/`${VAR}` expanded
/// through the global [`SubstVars`] before mapping. The `Vec` arm recurses with
/// the item meta, so list elements expand based on the item's flag, not the list's.
fn scalar_str(meta: &ValueMeta, v: &str) -> std::result::Result<Value, ScalarErr> {
    let substituted;
    let v: &str = if meta.is_support_subst() {
        substituted = SubstVars::current().expand(v);
        &substituted
    } else {
        v
    };
    match meta {
        ValueMeta::String { .. } => Ok(Value::String(v.to_string())),
        ValueMeta::Int { .. } => v
            .parse()
            .map(Value::Int)
            .map_err(|e| ScalarErr::Custom(format!("invalid integer {v:?}: {e}"))),
        ValueMeta::Enum { one_of, .. } => one_of
            .iter()
            .find(|(_, name, _)| *name == v)
            .map(|(code, _, _)| Value::Enum(code))
            .ok_or_else(|| ScalarErr::Custom(format!("{v:?} is not a valid choice"))),
        ValueMeta::Duration { .. } => humantime::parse_duration(v)
            .map(Value::Duration)
            .map_err(|e| ScalarErr::Custom(format!("invalid duration {v:?}: {e}"))),
        ValueMeta::Tag { .. } => v
            .parse()
            .map(Value::Tag)
            .map_err(|e| ScalarErr::Custom(format!("invalid tag {v:?}: {e}"))),
        ValueMeta::Vr { .. } => v
            .parse()
            .map(Value::Vr)
            .map_err(|e| ScalarErr::Custom(format!("invalid VR {v:?}: {e}"))),
        #[cfg(feature = "uuid")]
        ValueMeta::Uuid { .. } => v
            .parse()
            .map(Value::Uuid)
            .map_err(|e| ScalarErr::Custom(format!("invalid UUID {v:?}: {e}"))),
        ValueMeta::File { .. } => Ok(Value::File(ConfiguredFile::Name {
            path: v.to_string(),
            hot_reload: false,
        })),
        ValueMeta::Network { .. } => v
            .parse::<NetworkDefinition>()
            .and_then(|d| d.resolve_sync())
            .map(Value::Network)
            .map_err(|e| ScalarErr::Custom(format!("invalid network {v:?}: {e}"))),
        ValueMeta::Host { default_port, .. } => {
            let mut def = v
                .parse::<HostDefinition>()
                .map_err(|e| ScalarErr::Custom(format!("invalid host {v:?}: {e}")))?;
            if let Some(port) = default_port {
                def.set_default_port(*port);
            }
            def.resolve_sync()
                .map(Value::Host)
                .map_err(|e| ScalarErr::Custom(format!("invalid host {v:?}: {e}")))
        }
        ValueMeta::Vec { meta, .. } => {
            let item = scalar_str(meta, v)?;
            Ok(Value::Vec(vec![item]))
        }
        _ => Err(ScalarErr::Mismatch),
    }
}

/// Builds a nested [`Value::Object`] from a YAML map routed by `items`.
fn visit_object<'de, A: MapAccess<'de>>(items: &'static ObjectMeta, mut map: A) -> std::result::Result<Value, A::Error> {
    let mut nested = Map::new();
    while let Some(key) = map.next_key::<String>()? {
        let km = items.key_meta_str(&key)
            .ok_or_else(|| de::Error::custom(format!("unknown field {key:?}")))?;
        let value = map.next_value_seed(ValueSeed::new(&km.value_meta))?;
        validate_leaf(km, &value).map_err(de::Error::custom)?;
        nested.add(km.key, value, None);
    }
    check_required(items, &nested).map_err(de::Error::custom)?;
    Ok(Value::Object(Object::new(OBJECT_LAYER_ID, items, nested)))
}

/// Enforces presence of required keys once an object has been fully read.
///
/// A key is *required* when it is non-nullable and has no usable default (its
/// default resolves to [`Value::Null`]): such a key must appear in the object,
/// since nothing else can supply a value. A key with a real default may be
/// omitted — it resolves through [`ObjectMeta::default_of`] at read time —
/// and an explicit `null` for a non-nullable key is rejected earlier, when the
/// value itself is read.
///
/// Runtime keys are never read from a file, and conditional keys are
/// association-matched and resolve through their own fallback; both are skipped.
/// Applies equally to a nested object and to the root document.
fn check_required(items: &'static ObjectMeta, present: &Map) -> std::result::Result<(), String> {
    for km in items.iter() {
        if km.runtime || km.conditional || present.0.contains_key(&km.key) {
            continue;
        }
        if !km.value_meta.is_nullable() && matches!(items.default_of(&km.key), None | Some(Value::Null)) {
            return Err(format!("missing required key {:?}", km.key.0));
        }
    }
    Ok(())
}

/// Builds a [`Value::Map`] of string keys to values typed by `values`.
fn visit_value_map<'de, A: MapAccess<'de>>(values: &ValueMeta, mut map: A) -> std::result::Result<Value, A::Error> {
    let mut out = crate::Map::new();
    while let Some(key) = map.next_key::<String>()? {
        let value = map.next_value_seed(ValueSeed::new(values))?;
        out.insert(key, value);
    }
    Ok(Value::Map(out))
}

/// Parses a file reference expressed as a YAML map.
fn visit_file_map<'de, A: MapAccess<'de>>(mut map: A) -> std::result::Result<Value, A::Error> {
    let mut path: Option<String> = None;
    let mut content: Option<String> = None;
    let mut reload = false;
    while let Some(key) = map.next_key::<String>()? {
        match key.as_str() {
            "file_name" | "path" => path = Some(map.next_value()?),
            "content" => content = Some(map.next_value()?),
            "reload" | "hot_reload" | "auto_reload" => reload = map.next_value()?,
            other => return Err(de::Error::custom(format!("unknown file field {other:?}"))),
        }
    }
    match (path, content) {
        (Some(p), None) => Ok(Value::File(ConfiguredFile::Name {
            path: p,
            hot_reload: reload,
        })),
        (None, Some(c)) => Ok(Value::File(ConfiguredFile::Content(c.into_bytes()))),
        (Some(_), Some(_)) => Err(de::Error::custom("file has both a path and inline content")),
        (None, None) => Err(de::Error::custom("file has neither a path nor content")),
    }
}

// Custom values deserialize straight into `serde_json::Value`, which serde-saphyr
// fills from the YAML stream; the registered `CustomType` then decodes it.

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::{Arc, config_object_meta};
    use super::*;
    use crate::config::meta::{EditName, Choices, KeyMetaBuilder, build};
    use crate::config::{ConfigValues, Key};
    use crate::network::AssocDescription;

    static STRING_ITEM: ValueMeta = build::String::new().build();

    static LISTEN_ITEMS: [KeyMeta; 2] = [
        KeyMetaBuilder::new(Key::new("addr"), build::String::new().build()).build(),
        KeyMetaBuilder::new(Key::new("port"), build::Int::new().min(0).max(65535).nullable().build()).build(),
    ];

    config_object_meta!{ fn listen_meta() = &LISTEN_ITEMS }

    static LISTEN_OBJ: ValueMeta = build::Object::new(listen_meta).build();

    static ENC_CHOICES: [(u32, &str, Option<EditName>); 2] = [
        (
            0,
            "deny",
            Some(EditName {
                display_name: "Deny",
                brief: None,
                help: None,
            }),
        ),
        (
            1,
            "fix",
            Some(EditName {
                display_name: "Fix",
                brief: None,
                help: None,
            }),
        ),
    ];

    const K_ARTIM: Key = Key::new("dicom.association.artim_timeout");
    const K_MAX: Key = Key::new("dicom.association.max");
    const K_LOCAL_AET: Key = Key::new("dicom.local_aet");
    const K_LISTEN: Key = Key::new("dicom.listen");
    const K_MODE: Key = Key::new("mode");
    const K_DELIM: Key = Key::new("delimiters");

    static METAS: [KeyMeta; 6] = [
        KeyMetaBuilder::new(K_ARTIM, build::Duration::new().build()).conditional().build(),
        KeyMetaBuilder::new(K_MAX, build::Int::new().min(0).build()).conditional().build(),
        KeyMetaBuilder::new(K_LOCAL_AET, build::Vec::new(&STRING_ITEM).build())
            .default(|| Value::Vec(Vec::new()))
            .build(),
        KeyMetaBuilder::new(K_LISTEN, build::Vec::new(&LISTEN_OBJ).build())
            .default(|| Value::Vec(Vec::new()))
            .build(),
        KeyMetaBuilder::new(K_MODE, build::Enum::new(Choices::Static(&ENC_CHOICES)).build())
            .default(|| Value::Enum(1))
            .build(),
        KeyMetaBuilder::new(K_DELIM, build::Map::new(&STRING_ITEM).build())
            .default(|| Value::Map(Default::default()))
            .build(),
    ];
    config_object_meta!{ fn meta() = &METAS }

    fn loader() -> YamlLoader {
        YamlLoader::new(meta())
    }

    const DOC: &str = "\
mode: fix
dicom:
  local_aet:
    - SERVER_A
    - SERVER_B
  listen:
    - addr: 127.0.0.1
      port: 104
    - addr: localhost
  association:
    - artim_timeout: 10s
    - max: 5
    - max: 50
      when:
        peer_aet: PEER
";

    #[test]
    fn loads_scalars_lists_objects_and_conditionals() {
        let cfg = loader().load_str(DOC).expect("load");

        // Enum mapped by store name.
        assert!(matches!(cfg.config_get(&K_MODE, None), Some(Value::Enum(1))));

        // Scalar-or-list: explicit list of strings.
        match cfg.config_get(&K_LOCAL_AET, None) {
            Some(Value::Vec(v)) => assert_eq!(v.len(), 2),
            other => panic!("local_aet: {other:?}"),
        }

        // Vec of objects with a nullable field omitted in the 2nd element.
        match cfg.config_get(&K_LISTEN, None) {
            Some(Value::Vec(v)) => assert_eq!(v.len(), 2),
            other => panic!("listen: {other:?}"),
        }

        // Conditional duration (unconditional entry).
        assert!(matches!(
            cfg.config_get(&K_ARTIM, None),
            Some(Value::Duration(d)) if d.as_secs() == 10
        ));

        // Conditional int: base entry without `when`.
        assert!(matches!(cfg.config_get(&K_MAX, None), Some(Value::Int(5))));

        // Conditional int: peer-specific override wins for matching peer.
        let peer = AssocDescription {
            peer_aet: Some("PEER".into()),
            ..Default::default()
        };
        assert!(matches!(cfg.config_get(&K_MAX, Some(&peer)), Some(Value::Int(50))));
    }

    #[test]
    fn loads_string_keyed_map() {
        let cfg = loader()
            .load_str("delimiters:\n  PN: \"^\"\n  DA: \".\"\n")
            .expect("load");
        match cfg.config_get(&K_DELIM, None) {
            Some(Value::Map(m)) => {
                assert_eq!(m.len(), 2);
                assert!(matches!(m.get("PN"), Some(Value::String(s)) if s == "^"));
                assert!(matches!(m.get("DA"), Some(Value::String(s)) if s == "."));
            }
            other => panic!("delimiters: {other:?}"),
        }
    }

    #[test]
    fn scalar_coerces_to_single_element_list() {
        let cfg = loader().load_str("dicom:\n  local_aet: SOLE\n").expect("load");
        match cfg.config_get(&K_LOCAL_AET, None) {
            Some(Value::Vec(v)) => {
                assert_eq!(v.len(), 1);
                assert!(matches!(&v[0], Value::String(s) if s == "SOLE"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unknown_key_reports_location() {
        let err = loader().load_str("bogus: 1\n").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("line 1"), "missing location: {msg}");
        assert!(msg.contains("bogus"), "missing key: {msg}");
    }

    // A `listen` element requires `addr` (non-nullable, no default); `port` is
    // nullable, so it may be omitted.
    #[test]
    fn object_missing_required_key_is_rejected() {
        let err = loader()
            .load_str("dicom:\n  listen:\n    - port: 104\n")
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("missing required key"), "unexpected: {msg}");
        assert!(msg.contains("addr"), "unexpected: {msg}");
    }

    #[test]
    fn object_with_required_key_and_omitted_nullable_loads() {
        let cfg = loader()
            .load_str("dicom:\n  listen:\n    - addr: localhost\n")
            .expect("load");
        match cfg.config_get(&K_LISTEN, None) {
            Some(Value::Vec(v)) => assert_eq!(v.len(), 1),
            other => panic!("listen: {other:?}"),
        }
    }

    // The root document is an object too: a required top-level key absent from
    // every file is rejected at finalize.
    #[test]
    fn root_missing_required_key_is_rejected() {
        static REQUIRED: [KeyMeta; 1] =
            [KeyMetaBuilder::new(Key::new("name"), build::String::new().build()).build()];
        config_object_meta! { fn required_meta() = &REQUIRED }

        let err = YamlLoader::new(required_meta()).load_str("{}\n").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("missing required key"), "unexpected: {msg}");
        assert!(msg.contains("name"), "unexpected: {msg}");
    }

    #[test]
    fn bad_enum_value_reports_location() {
        let err = loader().load_str("mode: nope\n").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("line 1"), "missing location: {msg}");
    }

    #[test]
    fn unexpected_key_in_conditional_entry_fails() {
        let err = loader()
            .load_str("dicom:\n  association:\n    - max: 5\n      bogus: 1\n")
            .unwrap_err();
        assert!(format!("{err}").contains("bogus"));
    }

    // A `null` element is accepted only when the item meta is itself nullable —
    // the loader now derives nullability from the item `ValueMeta`, not a flag
    // hard-coded to `false`.
    #[test]
    fn subst_expands_only_when_meta_opts_in() {
        let _guard = super::super::subst::lock_global_for_test();
        static SUBST_ITEM: ValueMeta = build::String::new().subst().build();
        static SUBST_METAS: [KeyMeta; 2] = [
            KeyMetaBuilder::new(Key::new("greeting"), build::String::new().subst().build()).build(),
            // List whose items opt into substitution; coercion of a bare scalar
            // must still expand via the item meta.
            KeyMetaBuilder::new(Key::new("names"), build::Vec::new(&SUBST_ITEM).build()).build(),
        ];
        // A non-subst field must be left untouched.
        static PLAIN_METAS: [KeyMeta; 1] =
            [KeyMetaBuilder::new(Key::new("greeting"), build::String::new().build()).build()];

        SubstVars::install(Arc::new(SubstVars::builder().var("WHO", "world").build()));

        config_object_meta!{ fn subst_meta() = &SUBST_METAS }

        let cfg = YamlLoader::new(subst_meta())
            .load_str("greeting: hi $WHO\nnames: $WHO\n")
            .expect("load");
        assert!(matches!(cfg.config_get(&Key::new("greeting"), None), Some(Value::String(s)) if s == "hi world"));
        match cfg.config_get(&Key::new("names"), None) {
            Some(Value::Vec(v)) => assert!(matches!(&v[0], Value::String(s) if s == "world")),
            other => panic!("names: {other:?}"),
        }

        config_object_meta!{ fn plain_meta() = &PLAIN_METAS }

        let cfg = YamlLoader::new(plain_meta())
            .load_str("greeting: hi $WHO\n")
            .expect("load");
        assert!(matches!(cfg.config_get(&Key::new("greeting"), None), Some(Value::String(s)) if s == "hi $WHO"));
    }

    #[test]
    fn network_and_host_parse_from_string_literals() {
        let _guard = super::super::subst::lock_global_for_test();
        static NET_HOST: [KeyMeta; 2] = [
            KeyMetaBuilder::new(
                Key::new("bind"),
                build::Network::new().domain().unix().ipv4().ipv6().subst().nullable().build(),
            )
            .build(),
            KeyMetaBuilder::new(
                Key::new("peer"),
                build::Host::new().domain().unix().ipv4().ipv6().default_port(104).nullable().build(),
            )
            .build(),
        ];

        SubstVars::install(Arc::new(SubstVars::builder().var("NET", "127.0.0.1/24").build()));
        config_object_meta!( fn net_host_meta() = &NET_HOST );

        let cfg = YamlLoader::new(net_host_meta())
            .load_str("bind: $NET\npeer: 10.0.0.1\n")
            .expect("load");

        // Network: substituted then parsed.
        match cfg.config_get(&Key::new("bind"), None) {
            Some(Value::Network(n)) => assert_eq!(format!("{}", n.definition), "127.0.0.1/24"),
            other => panic!("bind: {other:?}"),
        }
        // Host: default_port from the meta is applied when omitted.
        match cfg.config_get(&Key::new("peer"), None) {
            Some(Value::Host(h)) => {
                assert!(matches!(
                    h.definition,
                    crate::network::HostDefinition::Ip { port: Some(104), .. }
                ));
            }
            other => panic!("peer: {other:?}"),
        }

        // A malformed literal is rejected.
        let err = YamlLoader::new(net_host_meta())
            .load_str("bind: not a network!!\npeer: 10.0.0.1\n")
            .unwrap_err();
        assert!(format!("{err}").contains("network"));
    }

    #[test]
    fn vec_item_nullability_comes_from_item_meta() {
        static NULLABLE_STR_ITEM: ValueMeta = build::String::new().nullable().build();
        static NULL_LIST: [KeyMeta; 1] =
            [KeyMetaBuilder::new(Key::new("names"), build::Vec::new(&NULLABLE_STR_ITEM).build()).build()];

        config_object_meta!( fn null_list_meta() = &NULL_LIST );
        let loader = YamlLoader::new(null_list_meta());
        let cfg = loader.load_str("names:\n  - A\n  - null\n").expect("load");
        match cfg.config_get(&Key::new("names"), None) {
            Some(Value::Vec(v)) => {
                assert_eq!(v.len(), 2);
                assert!(matches!(&v[0], Value::String(s) if s == "A"));
                assert!(matches!(&v[1], Value::Null));
            }
            other => panic!("names: {other:?}"),
        }

        // A non-nullable item rejects `null`.
        static PLAIN_LIST: [KeyMeta; 1] =
            [KeyMetaBuilder::new(Key::new("names"), build::Vec::new(&STRING_ITEM).build()).build()];
        config_object_meta!( fn plain_list_meta() = &PLAIN_LIST );
        let loader = YamlLoader::new(plain_list_meta());
        assert!(loader.load_str("names:\n  - A\n  - null\n").is_err());
    }

    // Exercises every `ValueMeta` variant through the loader, asserting each maps
    // to the matching `Value`. `Uuid` is feature-gated and covered separately.
    #[test]
    fn loads_every_value_type() {
        use std::any::Any;

        #[derive(Debug)]
        struct Port(u16);
        struct PortType;
        impl crate::config::CustomType for PortType {
            fn name(&self) -> &'static str {
                "port"
            }
            fn decode(&self, node: &serde_json::Value) -> crate::error::Result<Arc<dyn Any + Send + Sync>> {
                let n = node.as_i64().ok_or_else(|| dicom_err!(InvalidData, "port expects an integer"))?;
                Ok(Arc::new(Port(
                    u16::try_from(n).map_err(|_| dicom_err!(InvalidData, "port out of range"))?,
                )))
            }
            fn encode(&self, value: &dyn Any) -> crate::error::Result<serde_json::Value> {
                let p = value.downcast_ref::<Port>().ok_or_else(|| dicom_err!(Internal, "wrong type"))?;
                Ok(serde_json::Value::from(p.0))
            }
        }
        static PORT_TYPE: PortType = PortType;

        static STR_ITEM: ValueMeta = build::String::new().build();
        static INNER: &[KeyMeta] = &[KeyMetaBuilder::new(Key::new("inner"), build::String::new().build()).build()];
        config_object_meta! { fn inner_meta() = INNER }

        static ALL: &[KeyMeta] = &[
            KeyMetaBuilder::new(Key::new("b"), build::Bool::new().build()).build(),
            KeyMetaBuilder::new(Key::new("s"), build::String::new().build()).build(),
            KeyMetaBuilder::new(Key::new("i"), build::Int::new().build()).build(),
            KeyMetaBuilder::new(Key::new("e"), build::Enum::new(Choices::Static(&ENC_CHOICES)).build()).build(),
            KeyMetaBuilder::new(Key::new("dur"), build::Duration::new().build()).build(),
            KeyMetaBuilder::new(Key::new("tag"), build::Tag::new().build()).build(),
            KeyMetaBuilder::new(Key::new("vr"), build::Vr::new().build()).build(),
            KeyMetaBuilder::new(Key::new("file"), build::File::new().allow_content().build()).build(),
            KeyMetaBuilder::new(Key::new("net"), build::Network::new().ipv4().build()).build(),
            KeyMetaBuilder::new(Key::new("host"), build::Host::new().ipv4().default_port(104).build()).build(),
            KeyMetaBuilder::new(Key::new("obj"), build::Object::new(inner_meta).build()).build(),
            KeyMetaBuilder::new(Key::new("vec"), build::Vec::new(&STR_ITEM).build()).build(),
            KeyMetaBuilder::new(Key::new("map"), build::Map::new(&STR_ITEM).build()).build(),
            KeyMetaBuilder::new(Key::new("custom"), build::Custom::new(&PORT_TYPE).build()).build(),
        ];
        config_object_meta! { fn all_meta() = ALL }

        let doc = "\
b: true
s: hello
i: 42
e: fix
dur: 10s
tag: \"(0010,0010)\"
vr: PN
file:
  content: PEM DATA
net: 127.0.0.1
host: 10.0.0.1
obj:
  inner: nested
vec:
  - a
  - b
map:
  k1: v1
custom: 104
";
        let cfg = YamlLoader::new(all_meta()).load_str(doc).expect("load");

        assert!(matches!(cfg.config_get(&Key::new("b"), None), Some(Value::Bool(true))));
        assert!(matches!(cfg.config_get(&Key::new("s"), None), Some(Value::String(s)) if s == "hello"));
        assert!(matches!(cfg.config_get(&Key::new("i"), None), Some(Value::Int(42))));
        assert!(matches!(cfg.config_get(&Key::new("e"), None), Some(Value::Enum(1))));
        assert!(matches!(cfg.config_get(&Key::new("dur"), None), Some(Value::Duration(d)) if d.as_secs() == 10));
        assert!(matches!(cfg.config_get(&Key::new("tag"), None), Some(Value::Tag(_))));
        assert!(matches!(cfg.config_get(&Key::new("vr"), None), Some(Value::Vr(_))));
        assert!(matches!(cfg.config_get(&Key::new("file"), None), Some(Value::File(_))));
        assert!(matches!(cfg.config_get(&Key::new("net"), None), Some(Value::Network(_))));
        assert!(matches!(cfg.config_get(&Key::new("host"), None), Some(Value::Host(_))));
        assert!(matches!(cfg.config_get(&Key::new("obj"), None), Some(Value::Object(_))));
        assert!(matches!(cfg.config_get(&Key::new("vec"), None), Some(Value::Vec(v)) if v.len() == 2));
        assert!(matches!(cfg.config_get(&Key::new("map"), None), Some(Value::Map(m)) if m.len() == 1));
        assert!(matches!(cfg.config_get(&Key::new("custom"), None), Some(Value::Custom(_))));
    }

    // A `serde`-deriving type, adapted via `Serde<T>`, loads from YAML and is
    // retrieved through the global context at every nesting position: as the root
    // value, inside a nested object, inside an array and inside a map.
    #[test]
    fn serde_custom_type_reads_from_global_context_at_every_nesting() {
        use crate::config::{GlobalConfig, subst::lock_global_for_test};
        use serde::{Deserialize, Serialize};

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Endpoint {
            host: String,
            port: u16,
        }
        static ENDPOINT: crate::config::Serde<Endpoint> = crate::config::Serde::new("endpoint");

        static EP_ITEM: ValueMeta = build::Custom::new(&ENDPOINT).build();
        static INNER: &[KeyMeta] = &[KeyMetaBuilder::new(Key::new("ep"), build::Custom::new(&ENDPOINT).build()).build()];
        config_object_meta! { fn inner_meta() = INNER }

        static ROOT: &[KeyMeta] = &[
            KeyMetaBuilder::new(Key::new("ep"), build::Custom::new(&ENDPOINT).build()).build(),
            KeyMetaBuilder::new(Key::new("obj"), build::Object::new(inner_meta).build()).build(),
            KeyMetaBuilder::new(Key::new("arr"), build::Vec::new(&EP_ITEM).build()).build(),
            KeyMetaBuilder::new(Key::new("map"), build::Map::new(&EP_ITEM).build()).build(),
        ];
        config_object_meta! { fn root_meta() = ROOT }

        let doc = "\
ep:
  host: root
  port: 104
obj:
  ep:
    host: nested
    port: 1
arr:
  - host: a
    port: 11
  - host: b
    port: 22
map:
  k1:
    host: m
    port: 33
";
        let cfg = YamlLoader::new(root_meta()).load_str(doc).expect("load");

        let _guard = lock_global_for_test();
        GlobalConfig::set_forced(Arc::new(cfg));
        let cfg = GlobalConfig::current();

        fn endpoint(v: &Value) -> &Endpoint {
            match v {
                Value::Custom(any) => any.downcast_ref::<Endpoint>().expect("Endpoint payload"),
                other => panic!("expected Value::Custom, got {other:?}"),
            }
        }

        // Root.
        assert_eq!(
            endpoint(cfg.config_get(&Key::new("ep"), None).expect("root ep")),
            &Endpoint { host: "root".into(), port: 104 }
        );

        // Nested object.
        let obj = match cfg.config_get(&Key::new("obj"), None).expect("obj") {
            Value::Object(o) => o,
            other => panic!("expected Value::Object, got {other:?}"),
        };
        assert_eq!(
            endpoint(obj.config_get(&Key::new("ep"), None).expect("nested ep")),
            &Endpoint { host: "nested".into(), port: 1 }
        );

        // Array.
        let arr = match cfg.config_get(&Key::new("arr"), None).expect("arr") {
            Value::Vec(items) => items,
            other => panic!("expected Value::Vec, got {other:?}"),
        };
        assert_eq!(arr.len(), 2);
        assert_eq!(endpoint(&arr[0]), &Endpoint { host: "a".into(), port: 11 });
        assert_eq!(endpoint(&arr[1]), &Endpoint { host: "b".into(), port: 22 });

        // Map.
        let map = match cfg.config_get(&Key::new("map"), None).expect("map") {
            Value::Map(m) => m,
            other => panic!("expected Value::Map, got {other:?}"),
        };
        assert_eq!(endpoint(map.get("k1").expect("map k1")), &Endpoint { host: "m".into(), port: 33 });
    }

    // A `serde` decode failure (here: a required field missing) is surfaced as a
    // load error rather than silently dropping the value.
    #[test]
    fn serde_custom_type_rejects_invalid_payload() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct Endpoint {
            host: String,
            port: u16,
        }
        static ENDPOINT: crate::config::Serde<Endpoint> = crate::config::Serde::new("endpoint");

        static ROOT: &[KeyMeta] = &[KeyMetaBuilder::new(Key::new("ep"), build::Custom::new(&ENDPOINT).build()).build()];
        config_object_meta! { fn root_meta() = ROOT }

        let err = YamlLoader::new(root_meta())
            .load_str("ep:\n  port: 104\n")
            .unwrap_err();
        assert!(format!("{err}").contains("host"), "unexpected error: {err}");
    }

    #[cfg(feature = "uuid")]
    #[test]
    fn loads_uuid_value() {
        static UUID_META: &[KeyMeta] =
            &[KeyMetaBuilder::new(Key::new("id"), build::Uuid::new().non_zero().build()).build()];
        config_object_meta! { fn uuid_meta() = UUID_META }

        let cfg = YamlLoader::new(uuid_meta())
            .load_str("id: 550e8400-e29b-41d4-a716-446655440000\n")
            .expect("load");
        assert!(matches!(cfg.config_get(&Key::new("id"), None), Some(Value::Uuid(_))));

        // The nil UUID is rejected under the `non_zero` flag.
        let err = YamlLoader::new(uuid_meta())
            .load_str("id: 00000000-0000-0000-0000-000000000000\n")
            .unwrap_err();
        assert!(format!("{err}").contains("nil UUID"), "unexpected: {err}");
    }
}
