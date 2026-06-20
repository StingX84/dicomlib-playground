//! Streaming, schema-driven YAML configuration loader.
//!
//! The loader does not build a whole-document DOM. Instead it drives
//! `serde-saphyr` with [`serde::de::DeserializeSeed`]s guided by the
//! [`Registry`]: as the parser walks the document, each key is routed against a
//! path index built from the registered [`StoreConcept`](super::meta::StoreConcept)
//! names, and each value is mapped to a [`Value`] according to its
//! [`ValueMeta`].
//!
//! Because errors are raised *during* deserialization, `serde-saphyr` stamps
//! them with a precise `line:column`; the loader prepends the file name to
//! produce `file:line:column` diagnostics.
//!
//! ## YAML shape
//!
//! A key's [`StoreConcept::name`](super::meta::StoreConcept::name) is a dotted
//! path. For non-conditional keys the path leads straight to the value:
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

use super::meta::{KeyMeta, ValueMeta};
use super::settings::{ConditionalKey, ConditionalSettings, Settings};
use super::value::ValueFile;
use super::{Config, ConfigNode, Key, Registry, Value};
use crate::IntoDicomErr;
use crate::{Arc, HashMap, dicom_err, error::Result};

use serde::Deserialize;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

/// Loads configuration from a YAML file or a directory of `*.yml` files.
pub struct YamlLoader {
    registry: Arc<Registry>,
    app_name: String,
    version: u32,
}

impl YamlLoader {
    /// Creates a loader bound to a metadata `registry`, the expected application
    /// name, and the application's current configuration `version`.
    pub fn new(registry: Arc<Registry>, app_name: impl Into<String>, version: u32) -> YamlLoader {
        YamlLoader {
            registry,
            app_name: app_name.into(),
            version,
        }
    }

    /// Loads configuration from `path`.
    ///
    /// If `path` is a file, it is the entire configuration. If it is a
    /// directory, every `*.yml` file in it is loaded in alphabetical order, with
    /// later files overriding earlier ones (last value wins).
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Config> {
        let files = collect_files(path.as_ref())?;
        if files.is_empty() {
            return Err(dicom_err!(
                NotFound,
                "no configuration found at {}",
                path.as_ref().display()
            ));
        }

        let index = build_index(&self.registry)?;
        let acc = Accumulator {
            settings: RefCell::new(Settings::new()),
            conditional: RefCell::new(ConditionalSettings::new()),
            found_version: Cell::new(None),
        };

        for file in &files {
            let text = std::fs::read_to_string(file).to_dicom_err_with(|| format!("cannot read {}", file.display()))?;
            let ctx = LoadCtx {
                index: &index,
                app_name: self.app_name.as_str(),
                current_version: self.version,
                acc: &acc,
            };
            parse_document(&text, &file.display().to_string(), &ctx)?;
        }

        self.finalize(acc)
    }

    /// Loads configuration from a single in-memory YAML document.
    pub fn load_str(&self, text: &str) -> Result<Config> {
        let index = build_index(&self.registry)?;
        let acc = Accumulator {
            settings: RefCell::new(Settings::new()),
            conditional: RefCell::new(ConditionalSettings::new()),
            found_version: Cell::new(None),
        };
        let ctx = LoadCtx {
            index: &index,
            app_name: self.app_name.as_str(),
            current_version: self.version,
            acc: &acc,
        };
        parse_document(text, "<memory>", &ctx)?;
        self.finalize(acc)
    }

    fn finalize(&self, acc: Accumulator) -> Result<Config> {
        let version = acc.found_version.get().unwrap_or(self.version);
        Ok(Config::builder(self.registry.clone())
            .settings(acc.settings.into_inner())
            .conditional(acc.conditional.into_inner())
            .version(version)
            .build())
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

fn build_index(registry: &Registry) -> Result<IndexNode<'_>> {
    let mut root = IndexNode::Branch(HashMap::new());
    for km in registry.iter() {
        let Some(store) = &km.store else { continue };
        let segments: Vec<&str> = store.name.split('.').collect();
        if segments.iter().any(|s| s.is_empty()) {
            return Err(dicom_err!(Configuration, "empty path segment in {:?}", store.name));
        }
        if store.conditional {
            insert_conditional(&mut root, &segments, km)?;
        } else {
            insert_leaf(&mut root, &segments, km)?;
        }
    }
    Ok(root)
}

/// Recursively descends/creates `Branch` nodes along `path`, returning the
/// branch map at its end. Written recursively to avoid the loop-reborrow that
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
    if map.contains_key(*last) {
        return Err(dicom_err!(Configuration, "duplicate configuration path {last:?}"));
    }
    map.insert((*last).to_string(), IndexNode::Leaf(km));
    Ok(())
}

fn insert_conditional<'a>(root: &mut IndexNode<'a>, segments: &[&str], km: &'a KeyMeta) -> Result<()> {
    if segments.len() < 2 {
        return Err(dicom_err!(
            Configuration,
            "conditional key {:?} needs a list path and a key",
            km.store.as_ref().map(|s| s.name)
        ));
    }
    let (key_seg, list_path) = segments.split_last().expect("len >= 2");
    if *key_seg == "when" {
        return Err(dicom_err!(
            Configuration,
            "conditional key must not be named 'when' ({:?})",
            km.store.as_ref().map(|s| s.name)
        ));
    }
    let (list_seg, parents) = list_path.split_last().expect("len >= 2");
    let map = branch_descend(root, parents)?;
    let entry = map
        .entry((*list_seg).to_string())
        .or_insert_with(|| IndexNode::CondList(HashMap::new()));
    match entry {
        IndexNode::CondList(keys) => {
            if keys.contains_key(*key_seg) {
                return Err(dicom_err!(Configuration, "duplicate conditional key {key_seg:?}"));
            }
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
    settings: RefCell<Settings>,
    conditional: RefCell<ConditionalSettings>,
    found_version: Cell<Option<u32>>,
}

struct LoadCtx<'a> {
    index: &'a IndexNode<'a>,
    app_name: &'a str,
    current_version: u32,
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
            is_root: true,
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
    App,
    Version,
    Branch(&'a HashMap<String, IndexNode<'a>>),
    Leaf(&'a KeyMeta),
    Cond(&'a HashMap<String, &'a KeyMeta>),
}

struct KeySeed<'a> {
    branch: &'a HashMap<String, IndexNode<'a>>,
    is_root: bool,
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
        if self.is_root && key == "app" {
            return Ok(Routed::App);
        }
        if self.is_root && key == "version" {
            return Ok(Routed::Version);
        }
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
    is_root: bool,
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
        while let Some(routed) = map.next_key_seed(KeySeed {
            branch: self.branch,
            is_root: self.is_root,
        })? {
            match routed {
                Routed::App => {
                    let got: String = map.next_value()?;
                    if got != ctx.app_name {
                        return Err(de::Error::custom(format!(
                            "configuration is for application {got:?}, expected {:?}",
                            ctx.app_name
                        )));
                    }
                }
                Routed::Version => {
                    let got: u32 = map.next_value()?;
                    if got > ctx.current_version {
                        return Err(de::Error::custom(format!(
                            "configuration version {got} is newer than supported {}",
                            ctx.current_version
                        )));
                    }
                    ctx.acc.found_version.set(Some(got));
                }
                Routed::Branch(child) => map.next_value_seed(MapWalk {
                    branch: child,
                    is_root: false,
                })?,
                Routed::Leaf(km) => {
                    let value = map.next_value_seed(ValueSeed::new(&km.value_meta, km.nullable))?;
                    validate_leaf(km, &value).map_err(de::Error::custom)?;
                    ctx.acc.settings.borrow_mut().set(km.key, value);
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
        let mut filter = Filter::default();

        while let Some(key) = map.next_key::<String>()? {
            if key == "when" {
                filter = map.next_value_seed(FilterSeed)?;
            } else if let Some(km) = self.keys.get(&key) {
                if setting.is_some() {
                    return Err(de::Error::custom(
                        "a conditional entry must describe exactly one setting",
                    ));
                }
                let value = map.next_value_seed(ValueSeed::new(&km.value_meta, km.nullable))?;
                validate_leaf(km, &value).map_err(de::Error::custom)?;
                setting = Some((km, value));
            } else {
                return Err(de::Error::custom(format!(
                    "unexpected key {key:?} in conditional entry"
                )));
            }
        }

        let (km, value) = setting.ok_or_else(|| de::Error::custom("conditional entry has no known setting key"))?;
        let cond_key = filter.into_key(km.key);
        ctx.acc.conditional.borrow_mut().add(cond_key, value);
        Ok(())
    }
}

#[derive(Default)]
struct Filter {
    peer_aet: Option<String>,
    local_aet: Option<String>,
    peer_ip: Option<IpAddr>,
    local_ip: Option<IpAddr>,
    local_port: Option<u16>,
}

impl Filter {
    fn into_key(self, key: Key) -> ConditionalKey {
        ConditionalKey {
            key,
            peer_aet: self.peer_aet.map(Cow::Owned),
            local_aet: self.local_aet.map(Cow::Owned),
            peer_ip: self.peer_ip,
            local_ip: self.local_ip,
            local_port: self.local_port,
        }
    }
}

struct FilterSeed;

impl<'de> DeserializeSeed<'de> for FilterSeed {
    type Value = Filter;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<Filter, D::Error> {
        d.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for FilterSeed {
    type Value = Filter;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a 'when' filter object")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Filter, A::Error> {
        let mut filter = Filter::default();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "peer_aet" => filter.peer_aet = Some(map.next_value()?),
                "local_aet" => filter.local_aet = Some(map.next_value()?),
                "peer_ip" => filter.peer_ip = Some(parse_ip(&map.next_value::<String>()?)?),
                "local_ip" => filter.local_ip = Some(parse_ip(&map.next_value::<String>()?)?),
                "local_port" => filter.local_port = Some(map.next_value()?),
                other => {
                    return Err(de::Error::custom(format!("unknown 'when' filter {other:?}")));
                }
            }
        }
        Ok(filter)
    }
}

fn parse_ip<E: de::Error>(s: &str) -> std::result::Result<IpAddr, E> {
    s.parse()
        .map_err(|_| de::Error::custom(format!("invalid IP address {s:?}")))
}

// ── Value mapping ───────────────────────────────────────────────────────────

struct ValueSeed<'a> {
    meta: &'a ValueMeta,
    nullable: bool,
}

impl<'a> ValueSeed<'a> {
    fn new(meta: &'a ValueMeta, nullable: bool) -> ValueSeed<'a> {
        ValueSeed { meta, nullable }
    }
}

impl<'de, 'a> DeserializeSeed<'de> for ValueSeed<'a> {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<Value, D::Error> {
        if let ValueMeta::Complex { ty, .. } = self.meta {
            let node = ConfigNode::deserialize(d)?;
            return ty
                .decode(&node)
                .map(Value::Complex)
                .map_err(|e| de::Error::custom(format!("{e}")));
        }
        d.deserialize_any(MetaVisitor {
            meta: self.meta,
            nullable: self.nullable,
        })
    }
}

struct MetaVisitor<'a> {
    meta: &'a ValueMeta,
    nullable: bool,
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
            ValueMeta::Bool => Ok(Value::Bool(v)),
            ValueMeta::Vec { items, .. } => single_vec(items, Value::Bool(v)),
            _ => Err(self.mismatch("a boolean")),
        }
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Value, E> {
        match self.meta {
            ValueMeta::Int { .. } => Ok(Value::Int(v)),
            ValueMeta::Vec { items, .. } => single_vec(items, scalar_int(items, v)?),
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
        if self.nullable {
            Ok(Value::Null)
        } else {
            Err(de::Error::custom("value must not be null"))
        }
    }

    fn visit_none<E: de::Error>(self) -> std::result::Result<Value, E> {
        self.visit_unit()
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Value, A::Error> {
        let ValueMeta::Vec { items, .. } = self.meta else {
            return Err(self.mismatch("a list"));
        };
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(ValueSeed::new(items, false))? {
            out.push(v);
        }
        Ok(Value::Vec(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> std::result::Result<Value, A::Error> {
        match self.meta {
            ValueMeta::Object { items, .. } => visit_object(items, map),
            ValueMeta::Map { values, .. } => visit_value_map(values, map),
            ValueMeta::File { .. } => visit_file_map(map),
            _ => Err(self.mismatch("an object")),
        }
    }
}

/// Wraps a single value into a one-element vector (scalar-to-list coercion).
fn single_vec<E: de::Error>(items: &ValueMeta, v: Value) -> std::result::Result<Value, E> {
    // The item meta must accept the value; it was produced for that meta below.
    let _ = items;
    Ok(Value::Vec(vec![v]))
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
fn scalar_str(meta: &ValueMeta, v: &str) -> std::result::Result<Value, ScalarErr> {
    match meta {
        ValueMeta::String { .. } => Ok(Value::String(v.to_string())),
        ValueMeta::Enum { one_of } => one_of
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
        ValueMeta::File { .. } => Ok(Value::File(ValueFile::Name {
            path: v.to_string(),
            auto_reload: false,
        })),
        ValueMeta::Vec { items, .. } => {
            let item = scalar_str(items, v)?;
            Ok(Value::Vec(vec![item]))
        }
        _ => Err(ScalarErr::Mismatch),
    }
}

/// Builds a nested [`Value::Object`] from a YAML map routed by `items`.
fn visit_object<'de, A: MapAccess<'de>>(items: &'static [KeyMeta], mut map: A) -> std::result::Result<Value, A::Error> {
    let mut nested = Settings::new();
    while let Some(key) = map.next_key::<String>()? {
        let km = items
            .iter()
            .find(|k| k.store.as_ref().is_some_and(|s| s.name == key))
            .ok_or_else(|| de::Error::custom(format!("unknown field {key:?}")))?;
        let value = map.next_value_seed(ValueSeed::new(&km.value_meta, km.nullable))?;
        validate_leaf(km, &value).map_err(de::Error::custom)?;
        nested.set(km.key, value);
    }
    let registry = Arc::new(Registry::new_from(items));
    Ok(Value::Object(Config::builder(registry).settings(nested).build()))
}

/// Builds a [`Value::Map`] of string keys to values typed by `values`.
fn visit_value_map<'de, A: MapAccess<'de>>(values: &ValueMeta, mut map: A) -> std::result::Result<Value, A::Error> {
    let mut out = crate::Map::new();
    while let Some(key) = map.next_key::<String>()? {
        let value = map.next_value_seed(ValueSeed::new(values, false))?;
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
        (Some(p), None) => Ok(Value::File(ValueFile::Name {
            path: p,
            auto_reload: reload,
        })),
        (None, Some(c)) => Ok(Value::File(ValueFile::Content(c.into_bytes()))),
        (Some(_), Some(_)) => Err(de::Error::custom("file has both a path and inline content")),
        (None, None) => Err(de::Error::custom("file has neither a path nor content")),
    }
}

// ── ConfigNode deserialization (for Complex values) ─────────────────────────

impl<'de> serde::Deserialize<'de> for ConfigNode {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        d.deserialize_any(ConfigNodeVisitor)
    }
}

struct ConfigNodeVisitor;

impl<'de> Visitor<'de> for ConfigNodeVisitor {
    type Value = ConfigNode;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "any YAML value")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Bool(v))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Int(v))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Int(i64::try_from(v).unwrap_or(i64::MAX)))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Float(v))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Str(v.to_string()))
    }
    fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Str(v))
    }
    fn visit_unit<E: de::Error>(self) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Null)
    }
    fn visit_none<E: de::Error>(self) -> std::result::Result<ConfigNode, E> {
        Ok(ConfigNode::Null)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> std::result::Result<ConfigNode, D::Error> {
        ConfigNode::deserialize(d)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<ConfigNode, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = seq.next_element::<ConfigNode>()? {
            out.push(v);
        }
        Ok(ConfigNode::Seq(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<ConfigNode, A::Error> {
        let mut out = Vec::new();
        while let Some((k, v)) = map.next_entry::<String, ConfigNode>()? {
            out.push((k, v));
        }
        Ok(ConfigNode::Map(out))
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::meta::{EditName, MaybeGenerated, StoreConcept};
    use crate::config::settings::MatchAttributes;

    const fn sc(name: &'static str, conditional: bool) -> Option<StoreConcept> {
        Some(StoreConcept { name, conditional })
    }

    static STRING_ITEM: ValueMeta = ValueMeta::String {
        regexp: None,
        min_length: None,
        max_length: None,
        support_subst: false,
    };

    static LISTEN_ITEMS: [KeyMeta; 2] = [
        KeyMeta {
            key: Key::new("t.obj", 0),
            edit: None,
            store: sc("addr", false),
            nullable: false,
            default: None,
            value_meta: ValueMeta::String {
                regexp: None,
                min_length: None,
                max_length: None,
                support_subst: false,
            },
        },
        KeyMeta {
            key: Key::new("t.obj", 1),
            edit: None,
            store: sc("port", false),
            nullable: true,
            default: None,
            value_meta: ValueMeta::Int {
                min: Some(0),
                max: Some(65535),
            },
        },
    ];
    static LISTEN_OBJ: ValueMeta = ValueMeta::Object {
        items: &LISTEN_ITEMS,
        validate: |_| Ok(()),
    };

    static ENC_CHOICES: [(u32, &str, EditName); 2] = [
        (
            0,
            "deny",
            EditName {
                display_name: "Deny",
                brief: None,
                help: None,
            },
        ),
        (
            1,
            "fix",
            EditName {
                display_name: "Fix",
                brief: None,
                help: None,
            },
        ),
    ];

    const K_ARTIM: Key = Key::new("t", 0);
    const K_MAX: Key = Key::new("t", 1);
    const K_LOCAL_AET: Key = Key::new("t", 2);
    const K_LISTEN: Key = Key::new("t", 3);
    const K_MODE: Key = Key::new("t", 4);
    const K_DELIM: Key = Key::new("t", 5);

    static METAS: [KeyMeta; 6] = [
        KeyMeta {
            key: K_ARTIM,
            edit: None,
            store: sc("dicom.association.artim_timeout", true),
            nullable: false,
            default: None,
            value_meta: ValueMeta::Duration { min: None, max: None },
        },
        KeyMeta {
            key: K_MAX,
            edit: None,
            store: sc("dicom.association.max", true),
            nullable: false,
            default: None,
            value_meta: ValueMeta::Int {
                min: Some(0),
                max: None,
            },
        },
        KeyMeta {
            key: K_LOCAL_AET,
            edit: None,
            store: sc("dicom.local_aet", false),
            nullable: false,
            default: None,
            value_meta: ValueMeta::Vec {
                items: &STRING_ITEM,
                min_length: None,
                max_length: None,
                stride: None,
            },
        },
        KeyMeta {
            key: K_LISTEN,
            edit: None,
            store: sc("dicom.listen", false),
            nullable: false,
            default: None,
            value_meta: ValueMeta::Vec {
                items: &LISTEN_OBJ,
                min_length: None,
                max_length: None,
                stride: None,
            },
        },
        KeyMeta {
            key: K_MODE,
            edit: None,
            store: sc("mode", false),
            nullable: false,
            default: None,
            value_meta: ValueMeta::Enum {
                one_of: MaybeGenerated::Static(&ENC_CHOICES),
            },
        },
        KeyMeta {
            key: K_DELIM,
            edit: None,
            store: sc("delimiters", false),
            nullable: false,
            default: None,
            value_meta: ValueMeta::Map {
                values: &STRING_ITEM,
                min_length: None,
                max_length: None,
            },
        },
    ];

    fn loader() -> YamlLoader {
        YamlLoader::new(Arc::new(Registry::new_from(&METAS)), "testapp", 1)
    }

    const DOC: &str = "\
app: testapp
version: 1
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
        let none = MatchAttributes::default();

        // Enum mapped by store name.
        assert!(matches!(cfg.get(&K_MODE, &none), Some(Value::Enum(1))));

        // Scalar-or-list: explicit list of strings.
        match cfg.get(&K_LOCAL_AET, &none) {
            Some(Value::Vec(v)) => assert_eq!(v.len(), 2),
            other => panic!("local_aet: {other:?}"),
        }

        // Vec of objects with a nullable field omitted in the 2nd element.
        match cfg.get(&K_LISTEN, &none) {
            Some(Value::Vec(v)) => assert_eq!(v.len(), 2),
            other => panic!("listen: {other:?}"),
        }

        // Conditional duration (unconditional entry).
        assert!(matches!(
            cfg.get(&K_ARTIM, &none),
            Some(Value::Duration(d)) if d.as_secs() == 10
        ));

        // Conditional int: base entry without `when`.
        assert!(matches!(cfg.get(&K_MAX, &none), Some(Value::Int(5))));

        // Conditional int: peer-specific override wins for matching peer.
        let peer = MatchAttributes {
            peer_aet: Some("PEER"),
            ..Default::default()
        };
        assert!(matches!(cfg.get(&K_MAX, &peer), Some(Value::Int(50))));
    }

    #[test]
    fn loads_string_keyed_map() {
        let cfg = loader()
            .load_str("app: testapp\ndelimiters:\n  PN: \"^\"\n  DA: \".\"\n")
            .expect("load");
        match cfg.get(&K_DELIM, &MatchAttributes::default()) {
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
        let cfg = loader()
            .load_str("app: testapp\ndicom:\n  local_aet: SOLE\n")
            .expect("load");
        match cfg.get(&K_LOCAL_AET, &MatchAttributes::default()) {
            Some(Value::Vec(v)) => {
                assert_eq!(v.len(), 1);
                assert!(matches!(&v[0], Value::String(s) if s == "SOLE"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unknown_key_reports_location() {
        let err = loader().load_str("app: testapp\nbogus: 1\n").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("line 2"), "missing location: {msg}");
        assert!(msg.contains("bogus"), "missing key: {msg}");
    }

    #[test]
    fn bad_enum_value_reports_location() {
        let err = loader().load_str("app: testapp\nmode: nope\n").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("line 2"), "missing location: {msg}");
    }

    #[test]
    fn app_mismatch_fails() {
        let err = loader().load_str("app: other\n").unwrap_err();
        assert!(format!("{err}").contains("other"));
    }

    #[test]
    fn newer_version_fails() {
        let err = loader().load_str("app: testapp\nversion: 99\n").unwrap_err();
        assert!(format!("{err}").contains("newer"));
    }

    #[test]
    fn unexpected_key_in_conditional_entry_fails() {
        let err = loader()
            .load_str("app: testapp\ndicom:\n  association:\n    - max: 5\n      bogus: 1\n")
            .unwrap_err();
        assert!(format!("{err}").contains("bogus"));
    }
}
