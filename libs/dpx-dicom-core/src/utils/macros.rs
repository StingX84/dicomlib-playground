
/// Simplifies constant [Tag](crate::Tag)'s and a corresponding
/// [tag::StaticMetaList](crate::tag::StaticMetaList) declaration.
///
/// Expected syntax:
/// ```text
/// declare_tags! {
///     [[pub]] const LIST_NAME = [
///         const KEYWORD: { (group, element [[, creator]] ) [[& mask]], VR[[or VR]], VM, NAME, SOURCE },
///         ... may repeat ...
///     ];
///     ... may repeat ...
/// }
/// ```
/// Where:
/// - `[[` ... `]]` - Denotes optional parts, that may be omitted.
/// - `pub` - Optional visibility string. This visibility will be set to the
///   produced `LIST_NAME` as well as to each of the constants.
/// - `LIST_NAME` - The list of `static` variable containing all the Meta
///   information. This variable may be passed to
///   [tag::Dictionary::add_static_list](crate::tag::Dictionary::add_static_list)
///   or to [inventory::submit!] to automatically register this list.
/// - `KEYWORD` - The keyword, which becomes a name of the produced constant as
///   well as the [tag::Meta::keyword](crate::tag::Meta::keyword) member value.
/// - `group`, `element` - Numeric u16 literals of the resulting
///   [TagKey](crate::TagKey). Typically in hexadecimal form `0x1234`, `0x5678`.
/// - `creator` - Optional private creator string literal.
/// - `mask` - Optional mask u32 numeric literal. If absent, will be set to
///   0xFFFFFFFF.
/// - `VR` - One of [Vr](crate::Vr) enumeration values. You may supply up to
///   three value representations separated by " or " text.
/// - `VM` - Value multiplicity expression. One of the following forms expected:
///   `X`, `X - Y`, `X - n`, `X - Y n`, where `X` and `Y` numeric u8 literals.
///   See more on value multiplicity at [tag::Meta::vm](crate::tag::Meta::vm)
/// - `NAME` - String literal, that will become a
///   [tag::Meta::name](crate::tag::Meta::name) of the tag.
/// - `SOURCE` - Expression yielding [tag::Source](crate::tag::Source). All
///   enums variants from `tag::Source` and `tag::PrivateIdentificationAction`
///   already brought into scope in this macro, so you can simply write
///   `Vendored(D)` instead of
///   `tag::Source::Vendored(tag::PrivateIdentificationAction::D)`
///
/// Macro supports doc-comments and attributes for the `LIST_NAME` and `KEYWORD`.
/// Note: If you did not provide any doc-comment or attribute to the
/// `KEYWORD`, macro will automatically generate doc-comments for you.
///
/// After expansion, you will get:
/// - `const KEYWORD : dpx_dicom_core::Tag = ...;`
/// - `static LIST_NAME : dpx_dicom_core::uid::StaticMetaList = ...`
///
/// See example in [tag::Dictionary](crate::tag::Dictionary) struct
/// documentation.
#[macro_export]
macro_rules! declare_tags {
    ($(
        $(#[$outer:meta])*
        $pub:vis const $list_name:ident = [$(
            $(#[$inner:meta])*
            $keyword:ident :
            {
                ( $group:literal, $element:literal $(, $creator:literal )? ) $( & $mask:literal )?,
                $($vr:ident)or+,
                $vm:literal $( - $($vm2:literal)? $($n:ident)? )?,
                $name:literal,
                $source:expr
            }
        ),*$(,)?]
    );*;) => {
        $(
            $(
                $crate::declare_tags!{__doc_selector,
                    core::concat!(
                        ::core::stringify!( ( $group, $element $(, $creator )? ) $( & $mask )? ),
                        ", ", ::core::stringify!($($vr)or+),
                        ", ", ::core::stringify!($vm $(- $( $vm2 )? $( $n )? )?),
                        ", ", ::core::stringify!($name),
                    ),
                    $(#[$inner])*,
                    #[allow(non_upper_case_globals)]
                    $pub const $keyword : $crate::Tag =
                        $crate::Tag::new(
                            $crate::TagKey::new($group, $element),
                            $crate::declare_tags!(__creator_selector,$($creator)?));
                }
            )*
        )*
        mod _internals {
            #![allow(unused_imports)]
            use $crate::{ Vr::*, tag::Source::*, tag::PrivateIdentificationAction::* };
            use ::core::{ option::Option, stringify };
            use ::std::borrow::Cow;

            $(
                pub(super) static $list_name: &[$crate::tag::Meta] = &[ $(
                    $crate::declare_tags!(
                        __meta,
                        ( $group, $element $(, $creator )? ) $( & $mask )?,
                        $($vr)or+,
                        $vm $(- $( $vm2 )* $( $n )? )?,
                        $name,
                        stringify!($keyword),
                        $source)
                    ),* ];
            )*
        }

        $(
            $(#[$outer])*
            $pub static $list_name: $crate::tag::StaticMetaList = $crate::tag::StaticMetaList::new(
                _internals::$list_name
            );
        )*
    };

    // Internal selectors
    (
        __meta,
        ( $group:literal, $element:literal $(, $creator:literal )? ) $( & $mask:literal )?,
        $($vr:ident)or+,
        $vm:literal $(- $( $vm2:literal )* $( $n:ident )? )?  ,
        $name:expr,
        $keyword:expr,
        $source:expr
    ) => {{
        $crate::tag::Meta {
            tag: $crate::Tag::new(
                $crate::TagKey::new($group, $element),
                $crate::declare_tags!(__creator_selector,$($creator)?)),
            mask: $crate::declare_tags!(__mask_selector, $( $mask )? ),
            vr: $crate::declare_tags!(__vr_selector, $($vr),+ ),
            vm: $crate::declare_tags!(__alt_vm_selector, $vm $(- $( $vm2 )* $( $n )? )? ),
            name: Cow::<'static, str>::Borrowed($name),
            keyword: Cow::<'static, str>::Borrowed($keyword),
            source: $source
        }
    }};

    (__doc_selector,$def_doc:expr,$(#[$inner:meta])+,$($c:tt)+) => { $(#[$inner])+ $($c)+ };
    (__doc_selector,$def_doc:expr,,$($c:tt)+) => { #[doc=$def_doc] $($c)+ };
    (__creator_selector,) => { Option::None };
    (__creator_selector,$v:expr) => { Option::Some(::std::borrow::Cow::<'static, str>::Borrowed($v)) };
    (__mask_selector,) => { 0xFFFFFFFFu32 };
    (__mask_selector,$v:expr) => { $v };
    (__vr_selector,) => { (Undefined, Undefined, Undefined ) };
    (__vr_selector,$vr1:ident) => { ( $vr1, Undefined, Undefined ) };
    (__vr_selector,$vr1:ident,$vr2:ident) => { ( $vr1, $vr2, Undefined ) };
    (__vr_selector,$vr1:ident,$vr2:ident,$vr3:ident) => { ( $vr1, $vr2, $vr3 ) };
    (__alt_vm_selector,$n1:tt) => { ($n1, $n1, 1) };
    (__alt_vm_selector,$n1:tt - n) => { ($n1, 0, 1) };
    (__alt_vm_selector,$n1:tt - $n2:tt n) => { ($n1, 0, $n1) };
    (__alt_vm_selector,$n1:tt - $n2:tt) => { ($n1, $n2, 1) };
}

/// Simplifies constant [Uid](crate::Uid)'s and a corresponding
/// [uid::StaticMetaList](crate::uid::StaticMetaList) declaration.
///
/// Expected syntax:
/// ```text
/// declare_uids! {
///     [[pub]] const LIST_NAME = [
///         const KEYWORD: { UID, RETIRED, NAME, TYPE },
///         ... may repeat ...
///     ];
///     ... may repeat ...
/// }
/// ```
/// Where:
/// - `pub` - Optional visibility string. This visibility will be set to the
///   produced `LIST_NAME` as well as to each of the constants.
/// - `LIST_NAME` - The list of `static` variable containing all the Meta
///   information. This variable may be passed to
///   [uid::Dictionary::add_static_list](crate::uid::Dictionary::add_static_list)
///   or to [inventory::submit!] to automatically register this list.
/// - `KEYWORD` - The keyword, which becomes a name of the produced constant as
///   well as the [uid::Meta::keyword](crate::uid::Meta::keyword) member value.
/// - `UID` - OID string literal, which will become a produced constant value
///   and stored in [uid::Meta::uid](crate::uid::Meta::uid) field.
/// - `RETIRED` - Boolean literals settings
///   [uid::Meta::is_retired](crate::uid::Meta::is_retired) flag.
/// - `NAME` - String literal, that will become a
///   [uid::Meta::name](crate::uid::Meta::name) field value.
/// - `TYPE` - Expression yielding [uid::UidType](crate::uid::UidType). Enums
///   variants from `uid::UidType` and enum `uid::StorageKind` brought into
///   scope in this macro, so you can simply write
///   `SopClassPatientStorage{kind:: StorageKind::Image, ...}` instead of
///   `uid::UidType::SopClassPatientStorage(uid::StorageKind::Image, ...)`
///
/// Macro supports doc-comments and attributes for the `LIST_NAME` and `KEYWORD`.
/// It does not generate any automatically.
///
/// After expansion, you will get:
/// - `const KEYWORD : &str = UID;`
/// - `static LIST_NAME : dpx_dicom_core::uid::StaticMetaList = ...`
///
/// See example in [uid::Dictionary](crate::uid::Dictionary) struct
/// documentation.
#[macro_export]
macro_rules! declare_uids {
    ($(
        $(#[$outer:meta])*
        $pub:vis const $list_name:ident = [$(
            $(#[$inner:meta])*
            $keyword:ident :
            {
                $uid:literal,
                $is_retired:literal,
                $name:literal,
                $uid_type:expr
            }
        ),*$(,)?]
    );*;) => {
        $(
            $(
                $(#[$inner])*
                #[allow(non_upper_case_globals)]
                $pub const $keyword : &str = $uid;
            )*
        )*
        mod _internals {
            use core::stringify;
            use std::borrow::Cow;
            use $crate::{uid::UidType::*, uid::StorageKind, Uid};

            $(
                pub(super) static $list_name: &[$crate::uid::Meta] = &[ $(
                    $crate::uid::Meta {
                        uid: Uid::new(Cow::Borrowed($uid)),
                        is_retired: $is_retired,
                        name: Cow::Borrowed($name),
                        keyword: Cow::Borrowed(stringify!($keyword)),
                        uid_type: $uid_type,
                    }
                ),*];
            )*
        }

        $(
            $(#[$outer])*
            $pub static $list_name: $crate::uid::StaticMetaList = $crate::uid::StaticMetaList::new(
                _internals::$list_name
            );
        )*
    };
}
