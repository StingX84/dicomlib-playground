use std::fmt;

/// Factual classification of what caused the error.
///
/// Callers branch on this discriminant. The library makes no assumptions about
/// what the caller should do — retry, fail, log, surface to user — that is
/// entirely application policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// OS-level I/O failure (read, write, seek, flush, etc.)
    Io,
    /// Invalid configuration provided
    Configuration,
    /// Network socket or connection level failure
    Network,
    /// Remote peer violated the DICOM protocol
    Protocol,
    /// Data is structurally invalid or does not conform to the DICOM standard
    InvalidData,
    /// Valid DICOM feature not yet implemented by this library
    UnsupportedFeature,
    /// File, SOP instance, or named resource does not exist
    NotFound,
    /// Permission or authentication failure
    AccessDenied,
    /// Unexpected internal state — likely a library bug
    Internal,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io => write!(f, "I/O error"),
            Self::Configuration => write!(f, "Configuration"),
            Self::Network => write!(f, "network error"),
            Self::Protocol => write!(f, "DICOM protocol error"),
            Self::InvalidData => write!(f, "invalid DICOM data"),
            Self::UnsupportedFeature => write!(f, "unsupported DICOM feature"),
            Self::NotFound => write!(f, "resource not found"),
            Self::AccessDenied => write!(f, "access denied"),
            Self::Internal => write!(f, "internal error"),
        }
    }
}

/// Source location where the error was first created.
///
/// Captured automatically by [`dicom_err!`](crate::dicom_err) — equivalent to `__FILE__` /
/// `__LINE__` in C++. Not updated when context is added via [`dicom_ctx!`](crate::dicom_ctx).
#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// A knowledge-base entry carried by [`DicomError`].
///
/// Define one constant per KB entry and pass it to [`dicom_err!`](crate::dicom_err):
///
/// ```
/// use dpx_dicom_core::{KbEntry, dicom_err};
/// const CS_0001: KbEntry = KbEntry { id: "dpxkb_cs_0001", title: "Empty character set" };
/// let _err = dicom_err!(InvalidData, kb: CS_0001, "no terms in Specific Character Set");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct KbEntry {
    /// Anchor used in `docs/knowledge-base.md`, e.g. `"dpxkb_cs_0001"`.
    pub id: &'static str,
    /// Short title matching the KB section heading.
    pub title: &'static str,
}

/// Uniform error type used across all `dpx-dicom` crates.
///
/// Every error carries:
/// - a factual [`ErrorKind`] discriminant for programmatic branching
/// - an optional [`KbEntry`] linking to knowledge-base documentation
/// - an optional human-readable `message` with occurrence-specific detail;
///   when absent, [`ErrorKind`]'s `Display` is used as the default description
/// - the [`Location`] (file, line, column) where the error originated
/// - an optional chained `source` error from a lower layer
pub struct DicomError {
    pub kind: ErrorKind,
    pub kb: Option<KbEntry>,
    pub message: Option<String>,
    pub location: Location,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl DicomError {
    /// Attach a lower-level source error without changing the origin location.
    #[must_use]
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
}

impl fmt::Display for DicomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(entry) = self.kb {
            write!(f, "[{}] ", entry.id)?;
        }
        match &self.message {
            Some(msg) => write!(f, "{msg}")?,
            None => write!(f, "{}", self.kind)?,
        }
        write!(f, " (at {})", self.location)
    }
}

impl fmt::Debug for DicomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DicomError")
            .field("kind", &self.kind)
            .field("kb", &self.kb.map(|k| k.id))
            .field("message", &self.message)
            .field("location", &self.location.to_string())
            .field("source", &self.source.as_ref().map(|s| s.to_string()))
            .finish()
    }
}

impl std::error::Error for DicomError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|s| s as &(dyn std::error::Error + 'static))
    }
}

/// Convenience `Result` alias. The error type defaults to [`DicomError`].
pub type Result<T, E = DicomError> = std::result::Result<T, E>;

// ============================================================================
// ToErrorKind
// ============================================================================

/// Maps a foreign error type to the appropriate [`ErrorKind`].
///
/// Implement this for foreign error types that are converted into [`DicomError`]
/// via [`IntoDicomErr`]. The mapping is defined once per source type rather than
/// repeated at every call site.
///
/// # Example
///
/// ```
/// use dpx_dicom_core::error::{ToErrorKind, ErrorKind};
/// # #[derive(Debug)]
/// # struct MyError(bool);
/// # impl std::fmt::Display for MyError {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "err") }
/// # }
/// # impl std::error::Error for MyError {}
/// impl ToErrorKind for MyError {
///     fn to_error_kind(&self) -> ErrorKind {
///         if self.0 { ErrorKind::NotFound } else { ErrorKind::Io }
///     }
/// }
/// ```
pub trait ToErrorKind {
    fn to_error_kind(&self) -> ErrorKind;
}

impl ToErrorKind for std::io::Error {
    fn to_error_kind(&self) -> ErrorKind {
        match self.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied => ErrorKind::AccessDenied,
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::BrokenPipe => ErrorKind::Network,
            _ => ErrorKind::Io,
        }
    }
}

// ============================================================================
// ErrContext
// ============================================================================

/// Prepend context to an existing [`DicomError`] as it propagates up the call stack.
///
/// The origin [`Location`] is **never changed** — it always points to where
/// the error was first created, regardless of how many context layers are added.
///
/// # Example
///
/// ```
/// use dpx_dicom_core::{Result, error::ErrContext, dicom_err};
/// # fn parse_header(_: &[u8]) -> Result<()> {
/// #     Err(dicom_err!(InvalidData, "invalid byte at offset 42"))
/// # }
/// let path = std::path::Path::new("header.dcm");
/// let err = parse_header(b"")
///     .err_context_with(|| format!("parsing {}", path.display()))
///     .unwrap_err();
/// assert!(err.to_string().starts_with("parsing header.dcm:"));
/// ```
pub trait ErrContext<T> {
    /// Prepend `msg` to the error message. All other fields — including `location` — are unchanged.
    fn err_context(self, msg: impl Into<String>) -> Result<T>;

    /// Lazy variant — the closure is only evaluated when the result is an error.
    fn err_context_with(self, f: impl FnOnce() -> String) -> Result<T>;
}

impl<T> ErrContext<T> for Result<T> {
    fn err_context(self, msg: impl Into<String>) -> Result<T> {
        self.map_err(|mut e| {
            let prefix = msg.into();
            e.message = Some(match e.message.take() {
                Some(existing) => format!("{prefix}: {existing}"),
                None => format!("{prefix}: {}", e.kind),
            });
            e
        })
    }

    fn err_context_with(self, f: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|mut e| {
            let prefix = f();
            e.message = Some(match e.message.take() {
                Some(existing) => format!("{prefix}: {existing}"),
                None => format!("{prefix}: {}", e.kind),
            });
            e
        })
    }
}

// ============================================================================
// IntoDicomErr
// ============================================================================

/// Convert a foreign error into a [`DicomError`].
///
/// [`ErrorKind`] is derived automatically from the source error via [`ToErrorKind`],
/// so the correct kind (`NotFound`, `AccessDenied`, `Network`, etc.) is chosen at
/// runtime without the caller having to inspect the error.
///
/// The call-site location is captured via `#[track_caller]`.
///
/// # Example
///
/// ```
/// use dpx_dicom_core::error::{IntoDicomErr, ErrorKind};
/// let path = std::path::Path::new("/nonexistent");
/// let err = std::fs::File::open(path)
///     .to_dicom_err_with(|| format!("opening {}", path.display()))
///     .unwrap_err();
/// assert_eq!(err.kind, ErrorKind::NotFound);
/// ```
pub trait IntoDicomErr<T> {
    /// Convert into [`DicomError`], deriving kind via [`ToErrorKind`].
    ///
    /// Prefer [`to_dicom_err_with`](Self::to_dicom_err_with) when `msg` involves
    /// a `format!` call to avoid allocating on the success path.
    #[track_caller]
    fn to_dicom_err(self, msg: impl Into<String>) -> Result<T>;

    /// Lazy variant — the closure is only evaluated when the result is an error.
    #[track_caller]
    fn to_dicom_err_with(self, f: impl FnOnce() -> String) -> Result<T>;
}

impl<T, E> IntoDicomErr<T> for std::result::Result<T, E>
where
    E: std::error::Error + ToErrorKind + Send + Sync + 'static,
{
    #[track_caller]
    fn to_dicom_err(self, msg: impl Into<String>) -> Result<T> {
        // Capture caller location here, outside the closure, while #[track_caller]
        // is in effect. Inside the map_err closure Location::caller() would point
        // to this file rather than the user's call site.
        let loc = std::panic::Location::caller();
        self.map_err(|e| DicomError {
            kind: e.to_error_kind(),
            kb: None,
            message: Some(msg.into()),
            location: Location {
                file: loc.file(),
                line: loc.line(),
                column: loc.column(),
            },
            source: Some(Box::new(e)),
        })
    }

    #[track_caller]
    fn to_dicom_err_with(self, f: impl FnOnce() -> String) -> Result<T> {
        let loc = std::panic::Location::caller();
        self.map_err(|e| DicomError {
            kind: e.to_error_kind(),
            kb: None,
            message: Some(f()),
            location: Location {
                file: loc.file(),
                line: loc.line(),
                column: loc.column(),
            },
            source: Some(Box::new(e)),
        })
    }
}

/// Construct a [`DicomError`] capturing the call-site location automatically.
///
/// The location (file, line, column) is fixed at the macro call site and is
/// not updated when context is later added via [`dicom_ctx!`](crate::dicom_ctx).
///
/// # Forms
///
/// No message — `Display` falls back to the `ErrorKind` default description:
/// ```
/// use dpx_dicom_core::{KbEntry, dicom_err};
/// # const MY_KB: KbEntry = KbEntry { id: "dpxkb_ex_0001", title: "Example" };
/// let _e = dicom_err!(Io);
/// let _e = dicom_err!(InvalidData, kb: MY_KB);
/// ```
///
/// With a formatted message:
/// ```
/// use dpx_dicom_core::{KbEntry, dicom_err};
/// # const MY_KB: KbEntry = KbEntry { id: "dpxkb_ex_0001", title: "Example" };
/// let _e = dicom_err!(InvalidData, "unexpected tag {:#010x}", 0x00080010u32);
/// let _e = dicom_err!(InvalidData, kb: MY_KB, "unexpected tag {:#010x}", 0x00080010u32);
/// ```
///
/// With a chained source error:
/// ```
/// use dpx_dicom_core::dicom_err;
/// let _e = std::fs::read("/nonexistent")
///     .map_err(|e| dicom_err!(Io, "cannot read file").with_source(e))
///     .unwrap_err();
/// ```
#[macro_export]
macro_rules! dicom_err {
    // No message, with kb
    ($kind:ident, kb: $kb:expr $(,)?) => {
        $crate::error::DicomError {
            kind:     $crate::error::ErrorKind::$kind,
            kb:       Some($kb),
            message:  None,
            location: $crate::error::Location { file: file!(), line: line!(), column: column!() },
            source:   None,
        }
    };
    // No message, no kb
    ($kind:ident $(,)?) => {
        $crate::error::DicomError {
            kind:     $crate::error::ErrorKind::$kind,
            kb:       None,
            message:  None,
            location: $crate::error::Location { file: file!(), line: line!(), column: column!() },
            source:   None,
        }
    };
    // With message and kb
    ($kind:ident, kb: $kb:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::error::DicomError {
            kind:     $crate::error::ErrorKind::$kind,
            kb:       Some($kb),
            message:  Some(format!($fmt $(, $arg)*)),
            location: $crate::error::Location { file: file!(), line: line!(), column: column!() },
            source:   None,
        }
    };
    // With message, no kb
    ($kind:ident, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::error::DicomError {
            kind:     $crate::error::ErrorKind::$kind,
            kb:       None,
            message:  Some(format!($fmt $(, $arg)*)),
            location: $crate::error::Location { file: file!(), line: line!(), column: column!() },
            source:   None,
        }
    };
}

/// Add or replace context on an existing [`DicomError`] without changing its origin location.
///
/// Unlike [`dicom_err!`](crate::dicom_err), this macro does not capture a new location — the
/// error's `location` field stays pointing to where the error was first created.
///
/// # Forms
///
/// Replace or set `kb` only:
/// ```
/// use dpx_dicom_core::{KbEntry, dicom_err, dicom_ctx};
/// # const MY_KB: KbEntry = KbEntry { id: "dpxkb_ex_0001", title: "Example" };
/// let e = dicom_ctx!(dicom_err!(InvalidData), kb: MY_KB);
/// assert!(e.kb.is_some());
/// ```
///
/// Replace or set `message` only:
/// ```
/// use dpx_dicom_core::{dicom_err, dicom_ctx};
/// let e = dicom_ctx!(dicom_err!(InvalidData), "while processing tag {:#010x}", 0x00080010u32);
/// assert!(e.message.as_deref() == Some("while processing tag 0x00080010"));
/// ```
///
/// Replace or set both:
/// ```
/// use dpx_dicom_core::{KbEntry, dicom_err, dicom_ctx};
/// # const MY_KB: KbEntry = KbEntry { id: "dpxkb_ex_0001", title: "Example" };
/// let e = dicom_ctx!(dicom_err!(InvalidData), kb: MY_KB, "while processing tag {:#010x}", 0x00080010u32);
/// assert!(e.kb.is_some() && e.message.is_some());
/// ```
#[macro_export]
macro_rules! dicom_ctx {
    // Set kb only
    ($err:expr, kb: $kb:expr $(,)?) => {{
        let mut e = $err;
        e.kb = Some($kb);
        e
    }};
    // Set message only
    ($err:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {{
        let mut e = $err;
        e.message = Some(format!($fmt $(, $arg)*));
        e
    }};
    // Set both kb and message
    ($err:expr, kb: $kb:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {{
        let mut e = $err;
        e.kb = Some($kb);
        e.message = Some(format!($fmt $(, $arg)*));
        e
    }};
}
