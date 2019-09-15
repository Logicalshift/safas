use crate::meta::*;

use smallvec::*;

///
/// A SAFAS execution frame
///
pub struct Frame {
    /// The frame above this one on the 'stack'
    previous_frame: Option<Box<Frame>>,

    /// Cells allocated to this frame
    cells: SmallVec<[SafasCell; 8]>
}
