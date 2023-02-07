use crate::{Codec, Context};
use std::borrow::Cow;

pub fn decode<'a>(bytes: &'a [u8], codec: &Codec, _: &Context) -> Cow<'a, str> {
    let custom = codec
        .external
        .expect("App bug: codecs_rs should be initialized");
    let (rv, _) = custom.decode_without_bom_handling(bytes);
    rv
}

pub fn encode<'a>(string: &'a str, codec: &Codec, _: &Context) -> Cow<'a, [u8]> {
    let custom = codec
        .external
        .expect("App bug: codecs_rs should be initialized");
    let (rv, _, _) = custom.encode(string);
    rv
}
