//! Shell-style variable substitution (`$VAR` / `${VAR}`).
//!
//! [`SubstVars`] holds a flat set of name→value pairs plus the application's
//! well-known directories ([`AppDir`]). It expands `$VAR`/`${VAR}` references in
//! a text buffer. The process environment is snapshotted at build time, so
//! lookups never touch a lock or the (globally mutable) environment afterwards.
//!
//! It is a global singleton, swappable only as a whole: read with
//! [`SubstVars::current`], replace with [`SubstVars::install`].

use crate::Arc;
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

/// A well-known application directory, addressable by constant.
///
/// Each maps to a substitution variable of the same name, so `$CONF_DIR` in a
/// text buffer expands to the path set via [`Builder::dir`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppDir {
    Conf,
    Var,
    Log,
}

impl AppDir {
    /// The substitution variable name backing this directory.
    pub const fn var_name(self) -> &'static str {
        match self {
            AppDir::Conf => "CONF_DIR",
            AppDir::Var => "VAR_DIR",
            AppDir::Log => "LOG_DIR",
        }
    }
}

static GLOBAL: LazyLock<ArcSwap<SubstVars>> =
    LazyLock::new(|| ArcSwap::from_pointee(SubstVars::builder().build()));

/// An immutable set of substitution variables and application directories.
#[derive(Debug, Clone, Default)]
pub struct SubstVars {
    // Directories are stored here too, keyed by `AppDir::var_name`, so `$CONF_DIR`
    // and `dir(AppDir::Conf)` resolve from one map.
    vars: HashMap<String, String>,
}

impl SubstVars {
    /// Starts a [`Builder`] pre-populated with a snapshot of the process
    /// environment.
    pub fn builder() -> Builder {
        Builder {
            vars: std::env::vars().collect(),
        }
    }

    /// Returns the live global instance. Lock-free.
    pub fn current() -> Arc<SubstVars> {
        GLOBAL.load_full()
    }

    /// Atomically replaces the global instance.
    pub fn install(vars: Arc<SubstVars>) {
        GLOBAL.store(vars);
    }

    /// Returns the value of `name`, if set.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    /// Returns the path of a well-known directory, if set.
    pub fn dir(&self, dir: AppDir) -> Option<&Path> {
        self.vars.get(dir.var_name()).map(Path::new)
    }

    /// Expands `$VAR` and `${VAR}` references in `input`. `$$` yields a literal
    /// `$`. Unknown variables are left verbatim rather than blanked, so a stray
    /// `$` in non-variable text survives.
    pub fn expand(&self, input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '$' {
                out.push(c);
                continue;
            }
            match chars.peek().copied() {
                Some('$') => {
                    chars.next();
                    out.push('$');
                }
                Some('{') => {
                    chars.next();
                    let mut name = String::new();
                    let mut closed = false;
                    for nc in chars.by_ref() {
                        if nc == '}' {
                            closed = true;
                            break;
                        }
                        name.push(nc);
                    }
                    match (closed, self.vars.get(&name)) {
                        (true, Some(v)) => out.push_str(v),
                        (true, None) => {
                            out.push_str("${");
                            out.push_str(&name);
                            out.push('}');
                        }
                        // Unterminated `${...`: leave the lot verbatim.
                        (false, _) => {
                            out.push_str("${");
                            out.push_str(&name);
                        }
                    }
                }
                Some(nc) if nc == '_' || nc.is_ascii_alphabetic() => {
                    let mut name = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc == '_' || nc.is_ascii_alphanumeric() {
                            name.push(nc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    match self.vars.get(&name) {
                        Some(v) => out.push_str(v),
                        None => {
                            out.push('$');
                            out.push_str(&name);
                        }
                    }
                }
                _ => out.push('$'),
            }
        }
        out
    }
}

/// Builder for [`SubstVars`]. Obtained from [`SubstVars::builder`], which seeds
/// it with the current environment.
#[derive(Debug, Clone, Default)]
pub struct Builder {
    vars: HashMap<String, String>,
}

impl Builder {
    /// Adds or overrides a single substitution variable.
    pub fn var(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(name.into(), value.into());
        self
    }

    /// Adds or overrides multiple substitution variables.
    pub fn vars<K, V>(mut self, pairs: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.vars
            .extend(pairs.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Sets a well-known application directory, also exposing it as a
    /// substitution variable (e.g. `$CONF_DIR`).
    ///
    /// Non-UTF-8 path components are replaced lossily, since substitution targets
    /// text. 
    pub fn dir(mut self, dir: AppDir, path: impl AsRef<Path>) -> Self {
        self.vars.insert(
            dir.var_name().to_owned(),
            path.as_ref().to_string_lossy().into_owned(),
        );
        self
    }

    pub fn build(self) -> SubstVars {
        SubstVars { vars: self.vars }
    }
}

/// Serializes every test that swaps process-global state — the global
/// [`Context`](crate::Context) root (tag/uid dictionaries, base config) and the
/// installed [`SubstVars`]. They all share one `ArcSwap`, so without a single
/// crate-wide lock their save/restore dances clobber each other.
#[cfg(test)]
pub(crate) static GLOBAL_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Locks [`GLOBAL_TEST_LOCK`], tolerating poisoning from a panicking sibling
/// test so one failure does not cascade into spurious failures.
#[cfg(test)]
pub(crate) fn lock_global_for_test() -> std::sync::MutexGuard<'static, ()> {
    GLOBAL_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_known_unknown_braced_and_escaped() {
        let s = SubstVars::builder()
            .var("NAME", "world")
            .dir(AppDir::Conf, "/etc/app")
            .build();

        assert_eq!(s.expand("hi $NAME"), "hi world");
        assert_eq!(s.expand("hi ${NAME}!"), "hi world!");
        assert_eq!(s.expand("$NAME/${NAME}"), "world/world");
        // Directory exposed both ways.
        assert_eq!(s.expand("$CONF_DIR/x"), "/etc/app/x");
        assert_eq!(s.dir(AppDir::Conf), Some(Path::new("/etc/app")));
        assert_eq!(s.dir(AppDir::Var), None);
        // Unknown left verbatim; `$$` escapes.
        assert_eq!(s.expand("$NOPE and $$5"), "$NOPE and $5");
        assert_eq!(s.expand("${NOPE}"), "${NOPE}");
        // Unterminated brace.
        assert_eq!(s.expand("${oops"), "${oops");
        // Boundary: var name stops at non-word char.
        assert_eq!(s.expand("$NAME."), "world.");
    }

    #[test]
    fn env_is_snapshotted() {
        // SAFETY: single-threaded test; no other thread reads the environment here.
        unsafe { std::env::set_var("DPX_SUBST_TEST", "from_env") };
        let s = SubstVars::builder().build();
        assert_eq!(s.get("DPX_SUBST_TEST"), Some("from_env"));
        assert_eq!(s.expand("$DPX_SUBST_TEST"), "from_env");
    }

    #[test]
    fn install_swaps_global() {
        let _guard = lock_global_for_test();
        let s = Arc::new(SubstVars::builder().var("K", "v").build());
        SubstVars::install(s);
        assert_eq!(SubstVars::current().get("K"), Some("v"));
    }
}
