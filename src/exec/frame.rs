use crate::meta::*;
use crate::bind::*;

use smallvec::*;

///
/// A SAFAS execution frame
///
pub struct Frame {
    /// The frame above this one on the 'stack'
    pub previous_frame: Option<Box<Frame>>,

    /// Cells allocated to this frame
    pub cells: SmallVec<[CellRef; 8]>,

    /// The stack for this frame
    pub stack: SmallVec<[CellRef; 8]>
}

impl Frame {
    ///
    /// Creates a new frame (with a previous frame, if appropriate)
    ///
    pub fn new(size: usize, previous_frame: Option<Frame>) -> Frame {
        Frame {
            previous_frame: previous_frame.map(|frame| Box::new(frame)),
            cells:          smallvec![NIL.clone(); size],
            stack:          smallvec![]
        }
    }

    ///
    /// Pops a frame, returning the parent frame, or none
    ///
    pub fn pop(self) -> Option<Frame> {
        self.previous_frame.map(|frame| *frame)
    }

    ///
    /// Allocates enough space for the specified bindings
    ///
    pub fn allocate_for_bindings(&mut self, bindings: &SymbolBindings) {
        while self.cells.len() < bindings.num_cells {
            self.cells.push(NIL.clone())
        }
    }
}
