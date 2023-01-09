use super::*;

#[derive(Clone)]
pub struct Container<'a> {
    context: ElementContext,
    tags: Vec<Value<'a>>,
}

// impl<'a> Container<'a> {
//     fn get_raw(tag: TagKey)
// }
