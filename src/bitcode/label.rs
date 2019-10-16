use crate::meta::*;

///
/// A label is a value that is derived from the output of a bitcode monad
///
#[derive(Clone)]
pub struct Label {
    /// The value of this label
    value: CellRef
}

impl Label {
    ///
    /// Creates a new label
    ///
    pub fn new() -> Label {
        Label {
            value: SafasCell::Nil.into()
        }
    }
}
