// cspell:ignore rfield

/// Local macro, that declares "ValueMeta", "Value" and "build" module with builders for each
/// ValueMeta variant.
///
/// See syntax in `config/meta.rs` for details.
macro_rules! declare_value_meta {
    ($(
        $(#[doc = $doc:tt])*
        $(#[cfg($cfg:meta)])*
        $name:ident { $(required: { $($rfield:ident : $rty:ty),* } $(,)?)? $(flags: { $($flag:ident),* } $(,)?)? $(limits: { $($field:ident : $ty:ty),* })? } = $native:ty),* $(,)
    ?) => {::place_macro::place! {
        /// Describes the type, constraints and flags of a [`Value`].
        #[derive(Debug, Clone)]
        pub enum ValueMeta {$(
            #[doc = __str__("Native: `" $native "`")]
            $(#[doc = $doc])*
            $(#[cfg($cfg)])*
            $name {
                $($($rfield: $rty,)*)?
                $($($flag: bool,)*)?
                $($($field: Option<$ty>,)*)?
                optional: bool
            },
        )*}
        impl ValueMeta {
            pub const fn kind_name(&self) -> &'static str {
                match self {
                    $($(#[cfg($cfg)])* Self::__ident__($name) { .. } => stringify!($name),)*
                }
            }
            pub const fn is_optional(&self) -> bool {
                match self {
                    $($(#[cfg($cfg)])* Self::__ident__($name) { optional, .. } => *optional,)*
                }
            }
            pub const fn is_support_subst(&self) -> bool {
                declare_value_meta!(@subst_match (self) () $( $(#[cfg($cfg)])* __ident__($name) [ $($($flag)*)? ] )* )
            }
        }
        #[derive(Debug, Clone)]
        pub enum Value {
            Null,
            $(
                $(#[doc = $doc])*
                $(#[cfg($cfg)])*
                __ident__($name)($native),
            )*
        }
        impl Value {
            pub const fn kind_name(&self) -> &'static str {
                match self {
                    Self::Null => "Null",
                    $($(#[cfg($cfg)])* Self::__ident__($name) { .. } => stringify!($name),)*
                }
            }
        }
        pub mod build {
            #![allow(unused_imports)]
            use super::*;
            pub trait Native {
               type T;
            }
            $(
                declare_value_meta!{@doc_selector,
                    concat!("Builder for [`ValueMeta::", stringify!($name), "`]"),
                    $(#[cfg($cfg)])*,
                pub struct __ident__($name) {
                    $($($rfield: $rty,)*)?
                    $($($flag: bool,)*)?
                    $($($field: Option<$ty>,)*)?
                    optional: bool,
                }
            }
            $(#[cfg($cfg)])*
            impl __ident__($name) {
                pub const fn new($($($rfield : $rty),*)?) -> Self {
                    Self {
                        $($($rfield,)*)?
                        $($($flag: false,)*)?
                        $($($field: None,)*)?
                        optional: false
                    }
                }
                $($(pub const fn $field(mut self, value: $ty) -> Self {
                    self.$field = Some(value);
                    self
                })*)?
                $($(pub const fn $flag(mut self) -> Self {
                    self.$flag = true;
                    self
                })*)?
                pub const fn optional(mut self) -> Self {
                    self.optional = true;
                    self
                }
                pub const fn build(self) -> super::ValueMeta {
                    super::ValueMeta::$name {
                        $($($rfield: self.$rfield,)*)?
                        $($($flag: self.$flag,)*)?
                        $($($field: self.$field,)*)?
                        optional: self.optional,
                    }
                }
            }
            $(#[cfg($cfg)])*
            impl Native for __ident__($name) { type T = $native; }
        )*}
    }};
    // Build the `is_support_subst` match by recursing over variants and their
    // flags, emitting a real arm only for variants that carry a `subst` flag.
    (@subst_match ($s:expr) ($($arm:tt)*)) => {
        match $s { $($arm)* #[allow(unreachable_patterns)] _ => false }
    };
    (@subst_match ($s:expr) ($($arm:tt)*) $(#[cfg($cfg:meta)])* $name:ident [ ] $($rest:tt)*) => {
        declare_value_meta!(@subst_match ($s) ($($arm)*) $($rest)*)
    };
    (@subst_match ($s:expr) ($($arm:tt)*) $(#[cfg($cfg:meta)])* $name:ident [ subst $($f:ident)* ] $($rest:tt)*) => {
        declare_value_meta!(@subst_match ($s) ($($arm)* $(#[cfg($cfg)])* Self::$name { subst, .. } => *subst,) $($rest)*)
    };
    (@subst_match ($s:expr) ($($arm:tt)*) $(#[cfg($cfg:meta)])* $name:ident [ $other:ident $($f:ident)* ] $($rest:tt)*) => {
        declare_value_meta!(@subst_match ($s) ($($arm)*) $(#[cfg($cfg)])* $name [ $($f)* ] $($rest)*)
    };

    (@doc_selector,$def_doc:expr,$(#[$inner:meta])+,$($c:tt)+) => { $(#[$inner])+ $($c)+ };
    (@doc_selector,$def_doc:expr,,$($c:tt)+) => { #[doc=$def_doc] $($c)+ };
}

pub(crate) use declare_value_meta;

/// Crate local macro to declare a static `ObjectMeta` for a config object.
/// Used in test code only
#[cfg(test)]
macro_rules! config_object_meta {
    ($(#[$outer:meta])* $pub:vis fn $name:ident() = $items:expr ) => {
        ::place_macro::place! {
            static __ident__(__TO_CASE__($name) _OBJ_META): ::std::sync::OnceLock<$crate::config::meta::ObjectMeta> = ::std::sync::OnceLock::new();
            $($outer)*
            $pub fn $name() -> &'static $crate::config::meta::ObjectMeta {
                __ident__(__TO_CASE__($name) _OBJ_META).get_or_init(|| $crate::config::meta::ObjectMeta::new($items))
            }
        }
    };
}

#[cfg(test)]
pub(crate) use config_object_meta;

/// Declares one or more `#[repr(u32)]` config enums and wires them into the
/// config metadata system.
///
/// Write the enums as you normally would. For each one the macro emits the enum
/// plus `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display` (`Name(123)`),
/// `Into<config::Value>`, and a [`ConfigEnum`] impl.
///
/// Per-variant doc comments are repurposed: all lines becomes [`EnumVisual::help`](crate::config::meta::EnumVisual::help).
/// [`EnumVisual::display_name`](crate::config::meta::EnumVisual::display_name) is the
/// variant name verbatim, while the variant's string id (used by `name()` and
/// `Choices`) is that name in snake_case (e.g. `JpegLossless` -> `jpeg_lossless`).
///
/// Explicit discriminants (`= 42`), `#[cfg(...)]`, and `#[default]` are passed through.
///
/// ```
/// # use dpx_dicom_core::{declare_config_enums, config::ConfigEnum};
/// declare_config_enums! {
///     /// Transfer syntax preference
///     #[derive(Default)]
///     pub enum Compression {
///         None,
///         /// Lossless JPEG
///         #[default]
///         JpegLossless = 10,
///     }
///
///     /// Verbosity of the log
///     pub enum LogLevel {
///         /// Errors only
///         /// This is second line of help text.
///         Quiet = 1,
///         /// Everything
///         #[cfg(feature = "verbose")]
///         Verbose = 9,
///     }
/// }
/// ```
///
/// [`ConfigEnum`]: crate::config::ConfigEnum
#[macro_export]
macro_rules! declare_config_enums {
    // Entry point: *( [ DOCS ] ["pub"] enum NAME "{" *( [ DOCS ] VARIANT [ = VALUE ] ",") "}" ).
    // Declares rust enum NAME.
    ($(
        $(#[$outer:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[doc = $doc_help_first:literal] $(#[doc = $doc_help_rest:literal])*)?
                $(#[cfg($inner:meta)])*
                $(#[default $($_gate:tt)?])?
                $variant:ident $(= $value:expr)?
            ),* $(,)?
        }
    )*) => {$(
        ::place_macro::place! {
            #[repr(u32)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            $(#[$outer])*
            $vis enum $name {
                $(
                    #[doc = __str__(
                        "**Id**: `" __to_case__($variant) "`"
                        $("<br>**Help**: " $doc_help_first $("\n" $doc_help_rest)*)?
                    )]
                    $(#[cfg($inner)])*
                    $(#[default $($_gate)?])?
                    $variant $(= $value)?
                ,)*
            }

            impl $crate::config::ConfigEnum for __ident__($name) {
                const CHOICES: $crate::config::meta::Choices<(u32, &'static str, Option<$crate::config::meta::EnumVisual>)> =
                    $crate::config::meta::Choices::Static(
                        &[$(
                            $(#[cfg($inner)])*
                            (
                                Self::__ident__($variant) as u32,
                                __str__(__to_case__($variant)),
                                $crate::declare_config_enums!(@extract_enum_visual $variant $( $doc_help_first $( $doc_help_rest )* )?)
                            )
                        ),*]
                    );

                fn name(&self) -> &'static str {
                    match self {
                        $($(#[cfg($inner)])* Self::__ident__($variant) => __str__(__to_case__($variant)),)*
                    }
                }
                fn from_name(name: &str) -> Option<Self> {
                    match name {
                        $($(#[cfg($inner)])* __str__(__to_case__($variant)) => Some(Self::__ident__($variant)),)*

                        _ => None,
                    }
                }

                fn as_u32(&self) -> u32 {
                    *self as u32
                }

                fn from_u32(v: u32) -> Option<Self> {
                    $($(#[cfg($inner)])*
                    if v == Self::__ident__($variant) as u32 {
                        return Some(Self::__ident__($variant));
                    })*
                    None
                }
            }

            impl ::std::fmt::Display for __ident__($name) {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    use $crate::config::ConfigEnum;
                    write!(f, "{}({})", self.name(), self.as_u32())
                }
            }

            impl From<__ident__($name)> for u32 {
                fn from(value: __ident__($name)) -> u32 {
                    value as u32
                }
            }

            impl From<__ident__($name)> for $crate::config::Value {
                fn from(value: __ident__($name)) -> $crate::config::Value {
                    $crate::config::Value::Enum(value as u32)
                }
            }

            impl TryFrom<u32> for __ident__($name) {
                type Error = $crate::DicomError;
                fn try_from(value: u32) -> Result<Self, Self::Error> {
                    use $crate::config::ConfigEnum;
                    Self::from_u32(value)
                        .ok_or_else(|| $crate::dicom_err!(
                            Configuration,
                            "invalid enum value {value} for {}",
                            stringify!($name)))
                }
            }

            impl ::std::str::FromStr for __ident__($name) {
                type Err = $crate::DicomError;
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    use $crate::config::ConfigEnum;
                    Self::from_name(s)
                        .ok_or_else(|| $crate::dicom_err!(
                            Configuration,
                            "invalid enum name {s:?} for {}",
                            stringify!($name)))
                }
            }
        }
    )*};
    // Extracts the `EnumVisual` for a variant, if any doc comments were provided.
    ( @extract_enum_visual $variant:ident) => {
        ::place_macro::place! {
            Some($crate::config::meta::EnumVisual{
                display_name: __string__($variant),
                help: None,
            })
        }
    };
    ( @extract_enum_visual $variant:ident $($doc_help:literal)+) => {
        ::place_macro::place! {
            Some($crate::config::meta::EnumVisual{
                display_name: __string__($variant),
                help: Some(::dedent::dedent!(__string__($($doc_help "\n"),+))),
            })
        }
    };
}

/// Declares one or more configuration objects, each as a module of typed keys
/// plus its [`ObjectMeta`].
///
/// For every top-level object `NAME { ... }` the macro emits a `mod NAME`
/// containing:
/// - one `pub const` [`Key`] per leaf key, named after the key and typed
///   from its meta type and `optional` flag (`Option<T>` / `T`);
/// - a flat `KEY_META: &[KeyMeta]` describing every key (nested keys included);
/// - `pub fn object_meta() -> &'static ObjectMeta`.
///
/// Marking a top-level object `#[root]` additionally submits its `object_meta`
/// to the [`inventory`](crate::__inventory) so it is folded into
/// [`collected_global_meta`](crate::config::meta::collected_global_meta).
///
/// # Entry grammar
///
/// Inside the braces, comma-separated entries are either **leaf keys** or
/// nested **groups**:
///
/// ```text
/// name: MetaType [Construct] (attr = value, flag, ...) = default
/// group { ...nested entries... }
/// ```
///
/// - **`MetaType`** is a builder from [`config::meta::build`](crate::config::meta::build)
///   (`Bool`, `String`, `Int`, `Enum`, `Vec`, `Object`, `Custom`, ...).
/// - **`[Construct]`** supplies a required constructor argument: `Enum[MyEnum]`,
///   `Object[other_object]`, `Custom[&MY_TYPE]`, or an element meta for
///   containers, e.g. `Vec[String(min = 10)]`.
/// - **`(...)`** is mapped onto the builder: `flag` calls `.flag()`,
///   `name = value` calls `.name(value)`. The `optional` flag additionally
///   forces reads to return `Option<T>`.
/// - **`= default`** sets the key default: `Default`, `Null`, or any expression
///   (e.g. `"text"`, `MyEnum::Variant`, `IntRange { lo: 1, hi: 2 }`).
///
/// Nested groups become submodules and extend the dotted key path
/// (`nested { param: ... }` → key `nested.param`).
///
/// # Attributes
///
/// - Key doc comments map by line: first line → `display_name`, second →
///   `brief`, the rest → `help`.
/// - `#[section = "..."]` sets the UI section of a key.
/// - `#[config(read_only, is_advanced, conditional, runtime)]` sets
///   [`KeyMeta`] flags. On a group it propagates to **direct** children only.
/// - `#[cfg(...)]` is passed through and propagates to **all** descendants.
///
/// ```
/// # use dpx_dicom_core::{declare_config_objects, declare_config_enums, config::ConfigEnum};
/// # declare_config_enums! {
/// #   pub enum TestEnum {
/// #       SecondValue,
/// #   }
/// # }
/// declare_config_objects! {
///     #[root]
///     pub app_config {
///         /// Listen address
///         /// One-line brief.
///         /// Longer help text.
///         #[section = "Network"]
///         #[config(read_only)]
///         param: Vec[String(min = 10)] = Default,
///         enabled: Bool = true,
///         level: Enum[TestEnum] = TestEnum::SecondValue,
///         nested {
///             opt: String(optional),
///         },
///     }
/// }
/// # fn main() {}
/// ```
///
/// [`ObjectMeta`]: crate::config::meta::ObjectMeta
/// [`KeyMeta`]: crate::config::meta::KeyMeta
/// [`Key`]: crate::config::Key
#[macro_export]
macro_rules! declare_config_objects {
    // Main entry point
    ( $(
        $(#[doc = $docs:literal])*
        $(#[cfg($cfg:meta)])*
        $(#[root $($root_gate:tt)?])?
        $vis:vis $name:ident { $($tree:tt)* }
    )* ) => { $(
        $(#[doc = $docs])*
        $(#[cfg($cfg)])*
        $vis mod $name {
            #[allow(unused_imports)]
            use super::*;
            $crate::declare_config_objects!{ @declare_keys @m[] @p[] $($tree)* }
            // const KEY_META: &[$crate::config::meta::KeyMeta] = &[];
            $crate::declare_config_objects!{ @key_meta_list @acc[] @m[] @p[] @a[] $($tree)* }

            ::place_macro::place! {
                static OBJECT_META: ::std::sync::OnceLock<$crate::config::meta::ObjectMeta> = ::std::sync::OnceLock::new();
                #[doc=__str__("Provides an object meta information for `" $name "`")]
                pub fn object_meta() -> &'static $crate::config::meta::ObjectMeta {
                    OBJECT_META.get_or_init(|| $crate::config::meta::ObjectMeta::new(KEY_META))
                }
            }
            $crate::declare_config_objects! { @submit $name $(#[root $($root_gate)?])? }
        }
    )* };

    // ---- write recursively: const key = MetaKey; ------
    (@declare_keys @m[$($group_cfg_attr:ident)*] @p[$($parent_names:ident)*]
        $(#[doc = $doc_name:literal]
            $(#[doc = $doc_brief:literal]
                $(#[doc = $doc_help_first:literal] $(#[doc = $doc_help_rest:literal])*)?)?
        )?
        $(#[section = $doc_section:literal])?
        $(#[config($($key_cfg_attr:ident),* $(,)?)])?
        $(#[cfg($inner:meta)])*
        $name:ident: $meta_type:ident $([$($construct:tt)*])? $(($($attr:ident $(= $value:expr)?),* $(,)?))? $(= $default:expr)?
        $(, $($rest:tt)*)?
    ) => {
        ::place_macro::place! {
            #[doc = $crate::declare_config_objects!(@key_doc_string
                [__str__("**Id**: `" $($parent_names ".")* $name "`")]
                [$($doc_section)?]
                [$($doc_name)?]
                [$($($doc_brief)?)?]
                [$($($(__str__($doc_help_first $( "\n" $doc_help_rest )*))?)?)?]
                [$($($key_cfg_attr)*)? $($group_cfg_attr)*]
            )]
            $(#[cfg($inner)])*
            #[allow(non_upper_case_globals)]
            pub const $name: $crate::config::Key<
                    $crate::declare_config_objects!(@optional_selector [$meta_type] [$($($construct)*)?] $($($attr $(= $value)?),*)?)
                > = $crate::config::Key::new($crate::config::KeyId::new(__str__($($parent_names ".")* $name)));
        }
        $crate::declare_config_objects!{@declare_keys @m[$($group_cfg_attr)*] @p[$($parent_names)*] $($($rest)*)?}
    };
    (@declare_keys @m[$($group_cfg_attr:ident)*] @p[$($parent_names:ident)*]
        $(#[config($($this_group_config:ident),* $(,)?)])?
        $(#[cfg($inner:meta)])*
        $name:ident {
            $($ns_content:tt)*
        }
        $(, $($rest:tt)*)?
    ) => {
        $(#[cfg($inner)])*
        pub mod $name {
            #[allow(unused_imports)]
            use super::*;
            $crate::declare_config_objects!{@declare_keys @m[$($($this_group_config)*)?] @p[$($parent_names)* $name] $($ns_content)*}
        }
        $crate::declare_config_objects!{@declare_keys @m[$($group_cfg_attr)*] @p[$($parent_names)*] $($($rest)*)?}
    };

    (@declare_keys @m[$($group_cfg_attr:ident)*] @p[$($parent_names:ident)*] $(,)?) => {};

    // ---- Writes doc string if "Name" is not empty
    (@key_doc_string [$fl:literal] [$($doc_section:literal)?] [$doc_name:literal] [$($doc_brief:literal)?] [$($doc_help:literal)?] [$($config_attr:ident)*] ) => {
        ::place_macro::place! {__str__(
            $fl
            $("<br>**Section:** " $doc_section)?
            "<br>**Name:** `" $doc_name "`"
            $("<br>**Brief:**" $doc_brief )?
            $("<br>**Help:**" $doc_help )?
            "<br>**Config:** " $($config_attr " ")*
        )}
    };
    (@key_doc_string [$fl:literal] [$($doc_section:literal)?] [] [$($doc_brief:literal)?] [$($doc_help:literal)?] [$($config_attr:ident)*] ) => {
        ::place_macro::place! {__str__(
            $fl
            "<br>**Name:** &lt;no edit support&gt;"
            "<br>**Config:** " $($config_attr " ")*
        )}
    };

    // ---- Key type selector based on Enum or any other type ------
    (@key_type_selector [Enum] [$construct:path]) => {
        $construct
    };
    (@key_type_selector [$meta_type:ident] [$($construct:tt)*]) => {
        <$crate::config::meta::build::$meta_type as $crate::config::meta::build::Native>::T
    };

    // ---- Template arg `T` or `Option<T>` selector based on "optional" flag ------
    (@optional_selector [$meta_type:ident] [$($construct:tt)*]) => {
        $crate::declare_config_objects!(@key_type_selector [$meta_type] [$($construct)*])
    };
    (@optional_selector [$meta_type:ident] [$($construct:tt)*] optional $(,$($rest:tt)*)? ) => {
        Option<$crate::declare_config_objects!(@key_type_selector [$meta_type] [$($construct)*])>
     };
    (@optional_selector [$meta_type:ident] [$($construct:tt)*] $_flag:ident $( = $_value:expr)? $(,$($rest:tt)*)? ) => {
        $crate::declare_config_objects!(@optional_selector [$meta_type] [$($construct:)*] $($($rest)*)?)
    };

    // ---- MetaValue builder ------
    // Mulch attributes into a builder pattern call chain, then call `build()`.
    ( @mk_meta_value @acc[$($acc:tt)*] $meta_type:tt $([$($construct:tt)*])? ) => {
        ::place_macro::place!{
            $crate::config::meta::build::$meta_type::new(
                $($crate::declare_config_objects!(@construct $meta_type $($construct)*))?
            )
            $($acc)*
            .build()
        }
    };
    ( @mk_meta_value @acc[$($acc:tt)*] $meta_type:ident $([$($construct:tt)*])? ($attr_name:ident = $attr_value:expr) $($rest:tt)* ) => {
        $crate::declare_config_objects!(@mk_meta_value
            @acc[ $($acc)*.$attr_name($attr_value) ]
            $meta_type $([$($construct)*])? $($rest)*)
    };
    ( @mk_meta_value @acc[$($acc:tt)*] $meta_type:ident $([$($construct:tt)*])? ($flag_name:ident) $($rest:tt)* ) => {
        $crate::declare_config_objects!(@mk_meta_value
            @acc[ $($acc)*.$flag_name() ]
            $meta_type $([$($construct)*])? $($rest)*)
    };

    // ---- Convert constructor supporting sub-types
    ( @construct Enum $construct:path ) => {
        ::place_macro::place!( { use $crate::config::ConfigEnum; $construct::CHOICES } )
    };
    ( @construct Object $construct:path ) => {
        ::place_macro::place!( $construct::object_meta )
    };
    ( @construct Vec $meta_type:ident $([$($construct:tt)*])? $(($($attr:ident $(= $value:expr)?),* $(,)?))?) => {
        &$crate::declare_config_objects!(@mk_meta_value @acc[] $meta_type $([$($construct)*])? $($(($attr $(= $value)?))*)?)
    };
    ( @construct Map $meta_type:ident $([$($construct:tt)*])? $(($($attr:ident $(= $value:expr)?),* $(,)?))?) => {
        &$crate::declare_config_objects!(@mk_meta_value @acc[] $meta_type $([$($construct)*])? $($(($attr $(= $value)?))*)?)
    };
    ( @construct Custom $construct:expr) => {
        $construct
    };
    ( @construct $any_type:tt $($args:tt)*) => {
        THIS_TYPE_DOES_NOT_SUPPORT_CONSTRUCTORS
    };

    // ---- builds a constant "const KEY_META = [KeyMeta]" with a flat list of all keys ------
    // No more tokens to parse. Return the accumulated flat list.
    ( @key_meta_list @acc[ $($acc:tt)* ] @m[$($_dummy1:tt)*] @p[$($_dummy2:tt)*] @a[$($_dummy3:tt)*] $(,)? ) => {
        // Output the flat tokens inside your desired final expression wrapper
        pub const KEY_META: &[$crate::config::meta::KeyMeta] = &[$($acc)*];
    };

    // Restore previous state after subgroup leave
    ( @key_meta_list @acc[ $($acc:tt)* ] @m[$($group_cfg_attr:ident)*] @p[$($parents:ident)*] @a[$(#[cfg($section:meta)])*]
        $(,)? @restore{ [$($prev_cfg_attr:ident)*] [$($prev_parent:ident)*] [$(#[cfg($prev_section:meta)])*] }
        $(, $($tail:tt)*)?
    ) => {
        $crate::declare_config_objects! { @key_meta_list @acc[ $($acc)* ]
            @m[$($prev_cfg_attr)*] @p[$($prev_parent)*] @a[$(#[cfg($prev_section)])*]
            $($($tail)*)? }
    };

    // Recursive step: Sub-tree match. Found a nested bracket group `group {...}`.
    // It pops the group, flattens it first, and pushes remaining tokens to the tail.
    ( @key_meta_list @acc[ $($acc:tt)* ] @m[$($group_cfg_attr:ident)*] @p[$($parents:ident)*] @a[$(#[cfg($section:meta)])*]
        $(#[config($($this_group_cfg_attr:ident),* $(,)?)])?
        $(#[cfg($inner:meta)])*
        $name:ident {
            $($sub_tree:tt)*
        }
        $(, $($tail:tt)*)?
    ) => {
        $crate::declare_config_objects! { @key_meta_list @acc[ $($acc)* ]
            @m[$($($this_group_cfg_attr)*)?] @p[$($parents)* $name] @a[$(#[cfg($section)])* $(#[cfg($inner)])*]
            $($sub_tree)*
            ,@restore{ [$($group_cfg_attr)*] [$($parents)*] [$(#[cfg($section)])*]}
            $(, $($tail)*)? }
    };

    // Recursive step: Leaf node match. Found a single key description.
    // Pushes the item to the accumulator and processes the remaining tail.
    ( @key_meta_list @acc[ $($acc:tt)* ] @m[$($group_cfg_attr:ident)*] @p[$($parents:ident)*] @a[$(#[cfg($section:meta)])*]
        $(#[doc = $doc_name:literal]
            $(#[doc = $doc_brief:literal]
                $(#[doc = $doc_help_first:literal] $(#[doc = $doc_help_rest:literal])*)?)?
        )?
        $(#[section = $doc_section:literal])?
        $(#[config($($key_cfg_attr:ident),* $(,)?)])?
        $(#[cfg($inner:meta)])*
        $name:ident: $meta_type:ident $([$($construct:tt)*])? $(($($attr:ident $(= $value:expr)?),* $(,)?))? $(= $default:expr)?
        $(, $($tail:tt)*)?
    ) => {
        ::place_macro::place!{
            $crate::declare_config_objects! { @key_meta_list @acc[
                $($acc)*
                $(#[cfg($inner)])*
                $(#[cfg($section)])*
                $crate::declare_config_objects!(@config_attrs @acc[
                    $crate::config::meta::KeyMetaBuilder::new(
                        $($parents ::)* $name.id,
                        $crate::declare_config_objects!( @mk_meta_value @acc[] $meta_type $([$($construct)*])? $($(($attr $(= $value)?))*)?)
                    )
                    $( .display_name($doc_name)
                        $( .brief(::dedent::dedent!($doc_brief))
                            $(.help(::dedent::dedent!(__str__($doc_help_first $("\n" $doc_help_rest)*))) )?
                        )?
                    )?
                    $( .section($doc_section) )?
                ]
                @d[ [$($($construct)*)?] [$meta_type] [$($default)?]]
                $($([$key_cfg_attr])*)? $([$group_cfg_attr])* ).build(),
            ] @m[$($group_cfg_attr)*] @p[ $($parents)* ] @a[$(#[cfg($section)])*] $( $($tail)* )? }
        }
    };

    // ---- Select default value
    ( @default [$($construct:tt)*] [$meta_type:ident] [] ) => { };
    ( @default [$($construct:tt)*] [$meta_type:ident] [Null] ) => { $crate::config::Value::Null };
    ( @default [$($construct:tt)*] [Object] [Default] ) => { $crate::config::Value::Object($crate::config::Object::new_empty($($construct)* :: object_meta())) };
    ( @default [$($construct:tt)*] [$meta_type:ident] [Default] ) => { ::place_macro::place!{ $crate::config::Value::__ident__($meta_type)(Default::default()) } };
    ( @default [$($construct:tt)*] [Custom] [$raw:expr] ) => { ::place_macro::place!{ $crate::config::Value::Custom(std::sync::Arc::new($raw)) } };
    ( @default [$($construct:tt)*] [$meta_type:ident] [$raw:expr] ) => { ::place_macro::place!{ $crate::config::Value::__ident__($meta_type)(($raw).into()) } };

    // ---- Set KeyMetaBuilder config attributes
    ( @config_attrs @acc[$($acc:tt)*] @d[] ) => { $($acc)* };
    ( @config_attrs @acc[$($acc:tt)*] @d[ [$($construct:tt)*] [$meta_type:ident] []] ) => { $($acc)* };
    ( @config_attrs @acc[$($acc:tt)*] @d[ [$($construct:tt)*] [$meta_type:ident] [$($default:tt)+]] ) => {
        $crate::declare_config_objects!(@config_attrs @acc[
            $($acc)*
            .default(|| $crate::declare_config_objects!(@default [$($construct)*] [$meta_type] [$($default)+]))
        ]  @d[])
    };
    ( @config_attrs @acc[$($acc:tt)*] @d[ [$($construct:tt)*] [$meta_type:ident] [$($default:tt)*]] [$attr:ident] $([$rest:ident])* ) => {
        $crate::declare_config_objects!(@config_attrs @acc[$($acc)* .$attr()]  @d[ [$($construct)*] [$meta_type] [$($default)*]] $([$rest])*)
    };

    // ---- Submit to inventory if #[root] ------
    ( @submit $name:ident #[root $($root_gate:tt)?]) => {
        $crate::__inventory::submit! { $crate::config::meta::ObjectMetaProvider(object_meta) }
    };
    ( @submit $($rest:tt)* ) => {};
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use crate::Context;
    use crate::config::ConfigValues;
    use crate::config::subst::lock_global_for_test;

    crate::declare_config_enums! {
        /// Line1 *test*
        #[derive(Default)]
        pub enum TestEnum {
            NoComment,
            /// Line 1 with *test*
            #[cfg(feature = "uuid")]
            First,
            /// Line 1
            /// Line 2
            #[default]
            SecondValue =44,
            /// Line 1
            /// Line 2
            /// ``` example ```
            /// Line 3
            ThirdValue = 55,
            /// Line 1
            /// Line 2
            /// Line 3
            ///
            /// Line 4
            FourValue = 66,
        }
        pub enum Test2 {
            A,
            B,
        }
    }

    #[cfg(feature = "serde")]
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, Default)]
    struct IntRange {
        lo: i32,
        hi: i32,
    }
    #[cfg(feature = "serde")]
    static INT_RANGE: crate::config::custom::Serde<IntRange> = crate::config::custom::Serde::new("IntRange");

    declare_config_objects! {
        pub app_config {
            /// First line
            /// Second line
            /// Third line
            ///
            /// Last line
            #[section = "Section 1"]
            #[config(read_only, is_advanced, conditional, runtime)]
            param: Vec[String(min = 10)] = Default,

            /// First Line
            param2: Bool = false,

            /// First Line
            /// Second Line
            param3: String(min = 10, subst) = "test",

            param4: Enum[TestEnum](optional) = TestEnum::SecondValue,

            #[config(conditional)] // #[config()] propagated only on direct children. #[cfg()] propagates to all children.
            nested {
                param: String(optional),
                nested {
                    param: String(optional)
                }
            },
            #[cfg(feature = "uuid")]
            uuid_only {
                nested {
                    param: Uuid
                }
            },

            #[cfg(feature = "serde")]
            custom: Custom[&INT_RANGE] = IntRange { lo: 1, hi: 2 },
            #[cfg(feature = "serde")]
            custom2: Custom[&INT_RANGE] = IntRange::default(),
        }

        #[root]
        pub other {
            param: Vec[String(min = 10)] = Default,
            nested {
                param: Object[app_config] = Default,
            },
            nested2 {
                param: Int = 42,
            },
        }
    }

    #[test]
    fn value_reads_chained_nested_object_and_optional_enum() {
        let _guard = lock_global_for_test();
        Context::with_current(|ctx| {
            assert_eq!(
                ctx.value(other::nested::param).value(app_config::param4),
                Some(TestEnum::SecondValue)
            );
        });
    }

    #[test]
    #[cfg(feature = "serde")]
    fn value_reads_custom() {
        let _guard = lock_global_for_test();
        Context::with_current(|ctx| {
            let app = ctx.value(other::nested::param);
            let custom = app.value(app_config::custom);
            assert_eq!(custom.downcast_ref::<IntRange>(), Some(&IntRange { lo: 1, hi: 2 }));
        });
    }
}
