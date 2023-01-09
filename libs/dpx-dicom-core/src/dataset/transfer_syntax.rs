use super::*;

#[derive(Clone)]
pub struct TransferSyntax {
    is_little: bool,
    is_big: bool,
    is_deflate: bool,
    is_encapsulated: bool,
    oid: Cow<'static, str>,
}
