use super::bitcode_buffer::*;
use crate::meta::*;

use smallvec::*;

///
/// A SAFAS execution frame
///
pub struct Frame {
    /// The frame above this one on the 'stack'
    previous_frame: Option<Box<Frame>>,

    /// Cells allocated to this frame
    cells: SmallVec<[SafasCell; 8]>,

    /// The bitcode output of the assembler (bitcode is typically passed between frames, so it's stored by reference)
    bitcode: BitCodeBuffer
}
