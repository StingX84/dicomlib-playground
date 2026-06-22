//! The `config!` declaration macro.
//!
//! `config!` turns an ergonomic, nested description of a configuration surface
//! into three coordinated outputs, from a single source of truth:
//!
//! - **typed handles** — nested modules of [`TypedKey`](crate::config::TypedKey)
//!   constants whose path mirrors the dotted key path
//!   (`dicom::association::artim_timeout`);
//! - **runtime metadata** — a single constant `&[KeyMeta]` array registered with
//!   one [`StaticRegistry`](crate::config::StaticRegistry) submission;
//! - **generated types** — a Rust `enum` per `Enum` key.
//!
//! Internally the body is walked twice: an `@handles` pass emits the nested
//! module tree and any generated `enum`s, and an `@metas` pass accumulates every
//! key's [`KeyMeta`](crate::config::meta::KeyMeta) into one flat array (threading
//! the dotted-path prefix through nested groups with a `@pop` continuation
//! marker) and registers the whole array in a single call.
//!
//! ```ignore
//! use dpx_dicom_core::config;
//! use dpx_dicom_core::config::secs;
//!
//! config! {
//!     dicom {
//!         /// ARTIM timeout
//!         [conditional] artim_timeout: Duration = secs(10);
//!         local_aet: String = "DPX";
//!     }
//!     mode: Enum Mode { deny = 0 => "Deny", fix = 1 => "Fix" } = fix;
//! }
//! ```
//!
//! Grammar: nested `group { … }` blocks; leaves of the form
//! `[<attrs>] <name>: <Type> [?] = <default>;`. The leading `[…]` is optional and
//! holds storage modifiers (`conditional`, `runtime`) — an ordinary persisted,
//! unconditional key needs no bracket. `<Type>` is one of `Bool`, `Int`,
//! `Duration`, `String`, or `Enum <Name> { … }`; a trailing `?` marks a nullable
//! key. The default is a value expression (wrapped automatically into a `Value`)
//! or `fn(<expr>)` for a `fn() -> Value` the caller supplies. Doc lines map
//! positionally to `display_name` / `brief` / `help`.

/// Declares a configuration surface. See the [module docs](self).
#[macro_export]
macro_rules! config {
    // ═══ Pass 1: handles — nested module tree of TypedKey consts and enum types ═══

    (@handles [$($seg:ident)*]) => {};

    (@handles [$($seg:ident)*] $gname:ident { $($inner:tt)* } $($rest:tt)*) => {
        #[allow(non_upper_case_globals)]
        pub mod $gname {
            $crate::config!(@handles [$($seg)* $gname] $($inner)*);
        }
        $crate::config!(@handles [$($seg)*] $($rest)*);
    };

    // Scalars (4 arms each: nullable × fn/value). The default is consumed but
    // unused — only name, type and storage matter for the handle.
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (bool) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (bool) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (bool) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (bool) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };

    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (i64) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (i64) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (i64) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (i64) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };

    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (::core::time::Duration) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (::core::time::Duration) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (::core::time::Duration) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (::core::time::Duration) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };

    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (::std::string::String) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Opt (::std::string::String) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (::std::string::String) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String = $v:expr ; $($rest:tt)*) => { $crate::config!(@hdl [$($seg)*] $name Req (::std::string::String) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };

    // Enum (generate the type, then emit the handle).
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@enumdef $ty { $($vn = $code => $disp),+ }); $crate::config!(@hdl [$($seg)*] $name Opt ($ty) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } ? = $variant:ident ; $($rest:tt)*) => { $crate::config!(@enumdef $ty { $($vn = $code => $disp),+ }); $crate::config!(@hdl [$($seg)*] $name Opt ($ty) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@enumdef $ty { $($vn = $code => $disp),+ }); $crate::config!(@hdl [$($seg)*] $name Req ($ty) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };
    (@handles [$($seg:ident)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } = $variant:ident ; $($rest:tt)*) => { $crate::config!(@enumdef $ty { $($vn = $code => $disp),+ }); $crate::config!(@hdl [$($seg)*] $name Req ($ty) ($crate::config!(@cond $($($m)*)?))); $crate::config!(@handles [$($seg)*] $($rest)*); };

    (@hdl [$($seg:ident)*] $name:ident $nmark:ident ($t:ty) ($cond:expr)) => {
        #[allow(non_upper_case_globals)]
        pub const $name: $crate::config::typed::TypedKey<$t, $crate::config::typed::$nmark> =
            $crate::config::typed::TypedKey::new(concat!($(stringify!($seg), ".",)* stringify!($name)), $cond);
    };

    // ═══ Pass 2: metas — accumulate every KeyMeta into one flat array ═══════════════

    (@metas [$($seg:ident)*] [$($acc:tt)*]) => {
        $crate::__inventory::submit! {
            $crate::config::registry::StaticRegistry(&[ $($acc)* ])
        }
    };

    (@metas [$($seg:ident)*] [$($acc:tt)*] $gname:ident { $($inner:tt)* } $($rest:tt)*) => {
        $crate::config!(@metas [$($seg)* $gname] [$($acc)*] $($inner)* @pop [$($seg)*] $($rest)*);
    };
    (@metas [$($seg:ident)*] [$($acc:tt)*] @pop [$($outer:ident)*] $($rest:tt)*) => {
        $crate::config!(@metas [$($outer)*] [$($acc)*] $($rest)*);
    };

    // Scalars.
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Bool) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Bool) (|| $crate::config::value::Value::Bool($v))),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Bool) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Bool = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Bool) (|| $crate::config::value::Value::Bool($v))),] $($rest)*); };

    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Int { min: None, max: None }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Int { min: None, max: None }) (|| $crate::config::value::Value::Int($v))),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Int { min: None, max: None }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Int = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Int { min: None, max: None }) (|| $crate::config::value::Value::Int($v))),] $($rest)*); };

    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Duration { min: None, max: None }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Duration { min: None, max: None }) (|| $crate::config::value::Value::Duration($v))),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Duration { min: None, max: None }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Duration = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Duration { min: None, max: None }) (|| $crate::config::value::Value::Duration($v))),] $($rest)*); };

    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::String { regexp: None, min_length: None, max_length: None, support_subst: false }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String ? = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::String { regexp: None, min_length: None, max_length: None, support_subst: false }) (|| $crate::config::value::Value::String(($v).into()))),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::String { regexp: None, min_length: None, max_length: None, support_subst: false }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : String = $v:expr ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::String { regexp: None, min_length: None, max_length: None, support_subst: false }) (|| $crate::config::value::Value::String(($v).into()))),] $($rest)*); };

    // Enum (reference the generated type's CHOICES, qualified by module path).
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } ? = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Enum { one_of: $crate::config::meta::MaybeGenerated::Static(&$($seg::)* $ty::CHOICES) }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } ? = $variant:ident ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (true) $name ($crate::config::meta::ValueMeta::Enum { one_of: $crate::config::meta::MaybeGenerated::Static(&$($seg::)* $ty::CHOICES) }) (|| $crate::config::value::Value::Enum($($seg::)* $ty::$variant as u32))),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } = fn($f:expr) ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Enum { one_of: $crate::config::meta::MaybeGenerated::Static(&$($seg::)* $ty::CHOICES) }) ($f)),] $($rest)*); };
    (@metas [$($seg:ident)*] [$($acc:tt)*] $(#[doc = $d:literal])* $([$($m:ident),* $(,)?])? $name:ident : Enum $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ $(,)? } = $variant:ident ; $($rest:tt)*) => { $crate::config!(@metas [$($seg)*] [$($acc)* $crate::config!(@km [$($seg)*] [$($d)*] ($crate::config!(@cond $($($m)*)?)) ($crate::config!(@runtime $($($m)*)?)) (false) $name ($crate::config::meta::ValueMeta::Enum { one_of: $crate::config::meta::MaybeGenerated::Static(&$($seg::)* $ty::CHOICES) }) (|| $crate::config::value::Value::Enum($($seg::)* $ty::$variant as u32))),] $($rest)*); };

    // One KeyMeta value.
    (@km [$($seg:ident)*] [$($d:literal)*] ($cond:expr) ($run:expr) ($nullable:literal) $name:ident ($vm:expr) ($default:expr)) => {
        $crate::config::meta::KeyMeta {
            key: $crate::config::Key::new(concat!($(stringify!($seg), ".",)* stringify!($name))),
            edit: $crate::config!(@edit $($d)*),
            conditional: $cond,
            runtime: $run,
            nullable: $nullable,
            default: $default,
            value_meta: $vm,
        }
    };

    // ═══ Enum type generation ═══════════════════════════════════════════════════════
    (@enumdef $ty:ident { $($vn:ident = $code:literal => $disp:literal),+ }) => {
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        #[repr(u32)]
        pub enum $ty { $($vn = $code),+ }

        impl $ty {
            pub const CHOICES: [(u32, &'static str, $crate::config::meta::EditName); $crate::config!(@count $($vn)+)] = [
                $(($code, stringify!($vn), $crate::config::meta::EditName { display_name: $disp, brief: ::core::option::Option::None, help: ::core::option::Option::None }),)+
            ];
            pub fn from_code(code: u32) -> ::core::option::Option<$ty> {
                match code { $($code => ::core::option::Option::Some($ty::$vn),)+ _ => ::core::option::Option::None }
            }
        }

        impl $crate::config::typed::ValueRef for $ty {
            type Ref<'a> = $ty;
            fn project(v: &$crate::config::value::Value) -> ::core::option::Option<$ty> {
                if let $crate::config::value::Value::Enum(code) = v { $ty::from_code(*code) } else { ::core::option::Option::None }
            }
        }
    };

    // ═══ Shared helpers ═════════════════════════════════════════════════════════════
    (@cond) => { false };
    (@cond conditional $($rest:ident)*) => { true };
    (@cond $other:ident $($rest:ident)*) => { $crate::config!(@cond $($rest)*) };

    (@runtime) => { false };
    (@runtime runtime $($rest:ident)*) => { true };
    (@runtime $other:ident $($rest:ident)*) => { $crate::config!(@runtime $($rest)*) };

    (@edit) => { ::core::option::Option::None };
    (@edit $($d:literal)+) => {
        ::core::option::Option::Some($crate::config::meta::EditConcept {
            section: "",
            is_advanced: false,
            read_only: false,
            name: $crate::config!(@editname $($d)+),
        })
    };
    (@editname $dn:literal) => {
        $crate::config::meta::EditName { display_name: $dn, brief: ::core::option::Option::None, help: ::core::option::Option::None }
    };
    (@editname $dn:literal $br:literal) => {
        $crate::config::meta::EditName { display_name: $dn, brief: ::core::option::Option::Some($br), help: ::core::option::Option::None }
    };
    (@editname $dn:literal $br:literal $($h:literal)+) => {
        $crate::config::meta::EditName { display_name: $dn, brief: ::core::option::Option::Some($br), help: ::core::option::Option::Some(concat!($($h, "\n"),+)) }
    };

    (@count) => { 0usize };
    (@count $head:ident $($tail:ident)*) => { 1usize + $crate::config!(@count $($tail)*) };

    // ═══ Public entry (must stay last: matches any token stream) ════════════════════
    ($($body:tt)*) => {
        $crate::config!(@handles [] $($body)*);
        $crate::config!(@metas [] [] $($body)*);
    };
}

#[cfg(test)]
mod tests {
    use crate::Arc;
    use crate::config::secs;
    use crate::config::map::Condition;
    use crate::config::{Config, Registry};
    use core::time::Duration;

    crate::config! {
        dicom {
            /// ARTIM timeout
            [conditional] artim_timeout: Duration = secs(10);
            local_aet: String = "DPX";
            max_pdu: Int = 16384;
            [conditional, runtime] computed_max: Int = 7;
        }
        mode: Enum Mode { deny = 0 => "Deny", fix = 1 => "Fix" } = fix;
        verbose: Bool = false;
    }

    fn empty() -> Config {
        Config::builder(Arc::new(Registry::new_from_static())).build()
    }

    #[test]
    fn macro_generates_handles_and_defaults() {
        let cfg = empty();
        assert_eq!(
            dicom::artim_timeout.get_for(&cfg, Condition::default()),
            Duration::from_secs(10)
        );
        assert_eq!(dicom::local_aet.get(&cfg), "DPX");
        assert_eq!(dicom::max_pdu.get(&cfg), 16384);
        assert_eq!(mode.get(&cfg), Mode::fix);
        assert!(!verbose.get(&cfg));
    }

    #[test]
    fn storage_modifiers_set_flags() {
        assert!(!dicom::local_aet.conditional());
        assert!(dicom::artim_timeout.conditional());
        assert!(dicom::computed_max.conditional());
    }

    #[test]
    fn dotted_path_mirrors_module_path() {
        assert_eq!(dicom::artim_timeout.key().as_str(), "dicom.artim_timeout");
        assert_eq!(mode.key().as_str(), "mode");
    }

    #[test]
    fn all_keys_registered_in_one_array() {
        let reg = Registry::new_from_static();
        for path in [
            "dicom.artim_timeout",
            "dicom.local_aet",
            "dicom.max_pdu",
            "dicom.computed_max",
            "mode",
            "verbose",
        ] {
            assert!(reg.get(&crate::config::Key::new(path)).is_some(), "missing {path}");
        }
    }
}
