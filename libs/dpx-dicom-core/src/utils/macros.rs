
#[macro_export]
macro_rules! const_tag_meta {
    (
        ( $group:literal, $element:literal $(, $creator:literal )? ) $( & $mask:literal )?,
        $vr:ident$(or $vr_alt:ident)?,
        $vm:literal $(- $( $vm2:literal )* $( $n:ident )? )?  ,
        $name:literal,
        $keyword:literal,
        $source:expr
    ) => {{
        use $crate::Vr::*;
        use $crate::tag::Source::*;
        Meta::<'static> {
            tag: Tag::new(TagKey::new($group, $element), $crate::const_tag_meta!(__creator_selector,$($creator)?)),
            mask: $crate::const_tag_meta!(__mask_selector, $( $mask )? ),
            vr: $vr,
            alt_vr: $crate::const_tag_meta!(__alt_vr_selector, $( $vr_alt )? ),
            vm: $crate::const_tag_meta!(__alt_vm_selector, $vm $(- $( $vm2 )* $( $n )? )? ),
            name: ::std::borrow::Cow::<'static, str>::Borrowed($name),
            keyword: ::std::borrow::Cow::<'static, str>::Borrowed($keyword),
            source: $source
        }
    }};
    (__creator_selector,) => { ::core::option::Option::None };
    (__creator_selector,$v:expr) => { ::core::option::Option::Some(::std::borrow::Cow::Borrowed($v)) };
    (__mask_selector,) => { 0xFFFFFFFFu32 };
    (__mask_selector,$v:expr) => { $v };
    (__alt_vr_selector,) => { $crate::Vr::Undefined };
    (__alt_vr_selector,$vr:ident) => { $vr };
    (__alt_vm_selector,$n1:tt) => { ($n1, $n1, 1) };
    (__alt_vm_selector,$n1:tt - n) => { ($n1, 0, 1) };
    (__alt_vm_selector,$n1:tt - $n2:tt n) => { ($n1, 0, $n1) };
    (__alt_vm_selector,$n1:tt - $n2:tt) => { ($n1, $n2, 1) };
}

#[macro_export]
macro_rules! const_tag_meta_list {
    (
        $(#![$outer:meta])*
        $(
            $(#[$inner:meta])*
            $pub:vis const $const_name:ident =
                ( $group:literal, $element:literal $(, $creator:literal )? ) $( & $mask:literal )?,
                $vr:ident$(or $vr_alt:ident)?,
                $vm:literal $(- $( $vm2:literal )* $( $n:ident )? )?  ,
                $name:literal,
                $keyword:literal,
                $source:expr;
        )*
    ) => {
        $(
            $(#[$inner])*
            $pub const $const_name : $crate::tag::Tag<'static> =
                $crate::tag::Tag::new(
                    $crate::tag::TagKey::new($group, $element),
                    $crate::const_tag_meta!(__creator_selector,$($creator)?));
        )*

        const META_ARRAY : &'static [$crate::tag::Meta<'static>] = &[ $(
            $crate::const_tag_meta!(( $group, $element $(, $creator )? ) $( & $mask )?,
                $vr$(or $vr_alt)?,
                $vm $(- $( $vm2 )* $( $n )? )?,
                $name,
                $keyword,
                $source)
            ),* ];

        $(#[$outer])*
        pub const CONST_META_LIST: &'static $crate::tag::StaticMetaList = &StaticMetaList( META_ARRAY );
    }
}
