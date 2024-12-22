use super::{Act, ActRef};
use std::borrow::Cow;
/// An act argument by name or value.
#[derive(Debug, Clone)]
pub enum ActArg {
    /// Reference by reference
    ActRef(ActRef),
    // /// Reference by value
    // Act(Act),
    /// Reference by name
    Name(Cow<'static, str>),
}

// impl From<Act> for ActArg {
//     fn from(act: Act) -> Self {
//         ActArg::Act(act)
//     }
// }

impl From<ActRef> for ActArg {
    fn from(act: ActRef) -> Self {
        ActArg::ActRef(act)
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for ActArg {
    fn from(x: T) -> Self {
        ActArg::Name(x.into())
    }
}
