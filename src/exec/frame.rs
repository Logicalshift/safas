use crate::meta::*;

use smallvec::*;

use std::cell::{RefCell};
use std::rc::{Rc};

///
/// A SAFAS execution frame
///
pub struct Frame {
    /// The frame above this one on the 'stack'
    previous_frame: Option<Box<Frame>>,

    /// Cells allocated to this frame
    cells: SmallVec<[SafasCell; 8]>,

    /// The bitcode output of the assembler (bitcode is typically passed between frames, so it's stored by reference)
    bitcode: Rc<RefCell<Vec<BitCode>>>
}
