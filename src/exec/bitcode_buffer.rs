use crate::meta::*;

use std::rc::{Rc};
use std::cell::{RefCell};

///
/// A bitcode buffer forms a linked list of bitcode (this allows for portions of bitcode to be replaced without
/// copying the entire buffer). The list is in reverse, so the bitcode that should be compiled to the output
/// first is at the end.
///
pub struct BitCodeBuffer {
    /// The code in this buffer
    code: Rc<RefCell<Vec<BitCode>>>,

    /// Bitcode that precedes this code (or none if this is the first code block)
    preceding: Option<Rc<BitCodeBuffer>>
}
