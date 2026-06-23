#[macro_export]
macro_rules! config_object_meta {
    ($(#[$outer:meta])* $pub:vis fn $name:ident() = $items:expr ) => {
        paste::paste! {
            $pub static [<$name:upper _OBJ_META>]: std::sync::OnceLock<$crate::config::meta::ObjectMeta> = std::sync::OnceLock::new();
            $($outer)*
            $pub fn $name() -> &'static $crate::config::meta::ObjectMeta {
                [<$name:upper _OBJ_META>].get_or_init(|| $crate::config::meta::ObjectMeta::new($items))
            }
        }
    };
}
