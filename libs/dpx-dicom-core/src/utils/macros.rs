
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
                #[doc=::core::concat!(
                    ::core::stringify!( ( $group, $element $(, $creator )? ) $( & $mask )? ),
                    ", ", ::core::stringify!($($vr)or+),
                    ", ", ::core::stringify!($vm $(- $( $vm2 )? $( $n )? )?),
                    ", ", ::core::stringify!($name),
                )]
                $(#[$inner])*
                #[allow(non_upper_case_globals)]
                $pub const $keyword : $crate::Tag =
                    $crate::Tag::new(
                        $crate::TagKey::new($group, $element),
                        $crate::declare_tags!(__creator_selector,$($creator)?));
            )*
        )*
        mod _internals {
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


#[macro_export]
macro_rules! declare_uids {
    ($(
        $(#[$outer:meta])*
        $pub:vis const $list_name:ident = [$(
            $(#[$inner:meta])*
            $keyword:ident :
            {
                $uid:literal,
                $category:expr
            }
        ),*$(,)?]
    );*;) => {
        $(
            $(
                #[doc=::core::concat!(
                    $uid,
                    ", ", ::core::stringify!($category),
                )]
                $(#[$inner])*
                #[allow(non_upper_case_globals)]
                $pub const $keyword : $crate::Uid =
                    $crate::Uid::new($crate::Cow::Borrowed($uid));
            )*
        )*
        mod _internals {
            use core::stringify;
            use $crate::{uid::Category::*, Cow, Uid};

            $(
                pub(super) static $list_name: &[$crate::uid::Meta] = &[ $(
                    $crate::uid::Meta {
                        uid: Uid::new(Cow::Borrowed($uid)),
                        category: $category,
                        keyword: Cow::Borrowed(stringify!($keyword)),
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
