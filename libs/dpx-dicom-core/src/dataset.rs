use crate::*;

mod element;
mod container;
mod transfer_syntax;

pub use element::Element;
pub(crate) use element::ElementContext;
pub(crate) use element::ElementValue;
pub use container::Container;
pub use transfer_syntax::TransferSyntax;


pub trait Dataset<'a> {
    fn values() -> Container<'a>;
}
