#[macro_export]
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
/// ```ignore
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
/// [`ConfigEnum`]: crate::config::meta::ConfigEnum
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

            impl $crate::config::meta::ConfigEnum for __ident__($name) {
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
                    use $crate::config::meta::ConfigEnum;
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
                    use $crate::config::meta::ConfigEnum;
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
                    use $crate::config::meta::ConfigEnum;
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

#[macro_export]
macro_rules! declare_config_objects {
    // Main entry point
    ( $(
        $(#[doc = $docs:meta])*
        $(#[cfg($cfg:meta)])*
        $(#[root $($root_gate:tt)?])?
        $vis:vis $name:ident = { $($tree:tt)* }
    )* ) => { $(
        $(#[doc = $docs])*
        $(#[cfg($cfg)])*
        $vis mod $name {
            #[allow(unused_imports)]
            use super::*;
            // const typed_key = $crate::config::typed::TypedKey;
            $crate::declare_config_objects!{ @declare_keys @p[] $($tree)* }
            // const KEY_META: &[$crate::config::meta::KeyMeta] = &[];
            $crate::declare_config_objects!{ @key_meta_list @acc[] @p[] @a[] $($tree)* }

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
    (@declare_keys @p[$($parent_names:ident)*]
        $(#[doc = $doc_name:literal]
            $(#[doc = $doc_brief:literal]
                $(#[doc = $doc_help_first:literal] $(#[doc = $doc_help_rest:literal])*)?)?
        )?
        $(#[section = $doc_section:literal])?
        $(#[config($($config_attr:ident),* $(,)?)])?
        $(#[cfg($inner:meta)])*
        $name:ident: $meta_type:ident $([$($construct:tt)*])? $(($($attr:ident $(= $value:expr)?),* $(,)?))? $(= $default:expr)?
        $(, $($rest:tt)*)?
    ) => {
        ::place_macro::place! {
            #[doc =
                $crate::declare_config_objects!(@key_doc_string [__str__("**Id**: `" $($parent_names ".")* $name "`")] [$($doc_section)?] [$($doc_name)?] [$($($doc_brief)?)?] [$($($(__str__($doc_help_first $( "\n" $doc_help_rest )*))?)?)?])
            ]
            $(#[cfg($inner)])*
            #[allow(non_upper_case_globals)]
            pub const $name: $crate::config::typed::TypedKey<
                    <$crate::config::meta::build::$meta_type as $crate::config::meta::build::Native>::T,
                    $crate::declare_config_objects!(@optional_selector $($($attr $(= $value)?),*)?)
                > = $crate::config::typed::TypedKey::new(__str__($($parent_names ".")* $name));
        }
        $crate::declare_config_objects!{@declare_keys @p[$($parent_names)*] $($($rest)*)?}
    };
    (@declare_keys @p[$($parent_names:ident)*] $(#[cfg($inner:meta)])* $name:ident { $($ns_content:tt)* } $(, $($rest:tt)*)?) => {
        $(#[cfg($inner)])*
        pub mod $name {
            #[allow(unused_imports)]
            use super::*;
            $crate::declare_config_objects!{@declare_keys @p[$($parent_names)* $name] $($ns_content)*}
        }
        $crate::declare_config_objects!{@declare_keys @p[$($parent_names)*] $($($rest)*)?}
    };

    (@declare_keys @p[$($parent_names:ident)*] $(,)?) => {};

    // ---- Writes doc string if "Name" is not empty
    (@key_doc_string [$fl:literal] [$($doc_section:literal)?] [$doc_name:literal] [$($doc_brief:literal)?] [$($doc_help:literal)?] ) => {
        ::place_macro::place! {__str__(
            $fl
            $("<br>**Section:** " $doc_section)?
            "<br>**Name:** `" $doc_name "`"
            $("<br>**Brief:**" $doc_brief )?
            $("<br>**Help:**" $doc_help )?
        )}
    };
    (@key_doc_string [$fl:literal] [$($doc_section:literal)?] [] [$($doc_brief:literal)?] [$($doc_help:literal)?] ) => {
        concat!($fl, "<br>**Name:** &lt;no edit support&gt;")
    };

    // ---- Template arg `Req` or `Opt` selector based on "optional" flag ------
    (@optional_selector) => { $crate::config::typed::Req };
    (@optional_selector optional $(,$($rest:tt)*)? ) => { $crate::config::typed::Opt };
    (@optional_selector $_flag:ident $( = $_value:expr)? $(,$($rest:tt)*)? ) => {
        $crate::declare_config_objects!(@optional_selector $($($rest)*)?)
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
        ::place_macro::place!( { use $crate::config::meta::ConfigEnum; $construct::CHOICES } )
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
    ( @construct $any_type:tt $($args:tt)*) => {
        THIS_TYPE_DOES_NOT_SUPPORT_CONSTRUCTORS
    };

    // ---- builds a constant "const KEY_META = [KeyMeta]" with a flat list of all keys ------
    // No more tokens to parse. Return the accumulated flat list.
    ( @key_meta_list @acc[ $($acc:tt)* ] @p[$($_dummy1:tt)*] @a[$($_dummy2:tt)*] $(,)? ) => {
        // Output the flat tokens inside your desired final expression wrapper
        pub const KEY_META: &[$crate::config::meta::KeyMeta] = &[$($acc)*];
    };

    // Recursive step: Sub-tree match. Found a nested bracket group `group {...}`.
    // It pops the group, flattens it first, and pushes remaining tokens to the tail.
    ( @key_meta_list @acc[ $($acc:tt)* ] @p[ $($parents:ident)* ] @a[$(#[cfg($section:meta)])*] $(#[cfg($inner:meta)])* $name:ident { $($sub_tree:tt)* } $(, $($tail:tt)*)? ) => {
        $crate::declare_config_objects! { @key_meta_list @acc[ $($acc)* ] @p[ $($parents)* $name ] @a[$(#[cfg($section)])* $(#[cfg($inner)])*] $($sub_tree)* $(, $($tail)*)? }
    };

    // Recursive step: Leaf node match. Found a single key description.
    // Pushes the item to the accumulator and processes the remaining tail.
    ( @key_meta_list @acc[ $($acc:tt)* ] @p[ $($parents:ident)* ] @a[$(#[cfg($section:meta)])*]
        $(#[doc = $doc_name:literal]
            $(#[doc = $doc_brief:literal]
                $(#[doc = $doc_help_first:literal] $(#[doc = $doc_help_rest:literal])*)?)?
        )?
        $(#[section = $doc_section:literal])?
        $(#[config($($config_attr:ident),* $(,)?)])?
        $(#[cfg($inner:meta)])*
        $name:ident: $meta_type:ident $([$($construct:tt)*])? $(($($attr:ident $(= $value:expr)?),* $(,)?))? $(= $default:expr)?
        $(, $($tail:tt)*)?
    ) => {
        ::place_macro::place!{
            $crate::declare_config_objects! { @key_meta_list @acc[ $($acc)*
                $(#[cfg($inner)])*
                $(#[cfg($section)])*
                $crate::declare_config_objects!(@config_attrs @acc[
                    $crate::config::meta::KeyMetaBuilder::new(
                            $($parents ::)* $name.key(),
                            $crate::declare_config_objects!( @mk_meta_value @acc[] $meta_type $([$($construct)*])? $($(($attr $(= $value)?))*)?)
                        )
                        $( .default(|| $crate::declare_config_objects!(@default __ident__($meta_type) __id__($default))) )?
                        $( .display_name($doc_name)
                            $( .brief(::dedent::dedent!($doc_brief))
                                $(.help(::dedent::dedent!(__str__($doc_help_first $("\n" $doc_help_rest)*))) )?
                            )?
                        )?
                        $( .section($doc_section) )?
                ] $($([$config_attr])*)?)
                    .build(),
            ] @p[ $($parents)* ] @a[$(#[cfg($section)])*] $( $($tail)* )? }
        }
    };

    // ---- Select default value
    ( @default $meta_type:ident Null ) => { $crate::config::Value::Null };
    ( @default Object Default ) => { $crate::config::Value::Object($crate::config::Object::new_empty(object_meta())) };
    ( @default $meta_type:ident Default ) => { ::place_macro::place!{ $crate::config::Value::__ident__($meta_type)(Default::default()) } };
    ( @default $meta_type:ident $raw:expr ) => { ::place_macro::place!{ $crate::config::Value::__ident__($meta_type)(($raw).into()) } };

    // ---- Set KeyMetaBuilder config attributes
    ( @config_attrs @acc[$($acc:tt)*] ) => { $($acc)* };
    ( @config_attrs @acc[$($acc:tt)*] [$attr:ident] $([$rest:ident])* ) => { $crate::declare_config_objects!(@config_attrs @acc[$($acc)* .$attr()] $([$rest])*) };

    // ---- Submit to inventory if #[root] ------
    ( @submit $name:ident #[root $($root_gate:tt)?]) => {
        $crate::__inventory::submit! { $crate::config::meta::ObjectMetaProvider(object_meta) }
    };
    ( @submit $($rest:tt)* ) => {};
}

declare_config_objects! {
    pub app_config = {
        /// First line
        /// Second line
        /// Third line
        ///
        /// Last line
        #[section = "Section 1"]
        #[config(read_only, is_advanced, conditional, runtime)]
        param: Vec[String(min = 10)] = Default,

        /// First Line
        param2: Bool = false && true,

        /// First Line
        /// Second Line
        param3: String(min = 10, subst) = "test",
        param4: Enum[TestEnum] = TestEnum::SecondValue,

        nested {
            param: String(optional),
            nested {
                param: String(optional)
            }
        }
    }

    #[root]
    pub other = {
        param: Vec[String(min = 10)] = Default,
        nested {
            param: Object[app_config] = Null,
        }
    }
}
