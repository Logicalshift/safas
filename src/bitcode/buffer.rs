use super::code::*;

use std::rc::{Rc};
use std::cell::{RefCell};

///
/// A bitcode buffer forms a linked list of bitcode (this allows for portions of bitcode to be replaced without
/// copying the entire buffer). The list is in reverse, so the bitcode that should be compiled to the output
/// first is at the end.
///
#[derive(Clone)]
pub struct BitCodeBuffer {
    /// The code in this buffer
    pub code: Rc<RefCell<Vec<BitCode>>>,

    /// Bitcode that precedes this code (or none if this is the first code block)
    pub preceding: Option<Rc<BitCodeBuffer>>
}

impl BitCodeBuffer {
    ///
    /// Creates a new bitcode buffer
    ///
    pub fn new() -> BitCodeBuffer {
        BitCodeBuffer {
            code:       Rc::new(RefCell::new(vec![])),
            preceding:  None
        }
    }

    ///
    /// Writes code to this buffer
    ///
    pub fn extend<Code: IntoIterator<Item=BitCode>>(&mut self, code: Code) {
        self.code.borrow_mut().extend(code)
    }
}
