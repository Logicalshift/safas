use super::bitcode_buffer::*;
use crate::meta::*;

use smallvec::*;

use std::rc::*;
use std::sync::*;

///
/// A SAFAS execution frame
///
pub struct Frame {
    /// The frame above this one on the 'stack'
    pub previous_frame: Option<Box<Frame>>,

    /// Cells allocated to this frame
    pub cells: SmallVec<[Arc<SafasCell>; 8]>,

    /// The bitcode output of the assembler (bitcode is typically passed between frames, so it's stored by reference)
    pub bitcode: BitCodeBuffer,
}

impl Frame {
    ///
    /// Creates a new frame (with a previous frame, if appropriate)
    ///
    pub fn new(size: usize, previous_frame: Option<Frame>) -> Frame {
        Frame {
            previous_frame: previous_frame.map(|frame| Box::new(frame)),
            cells:          smallvec![Arc::new(SafasCell::Nil); size],
            bitcode:        BitCodeBuffer::new()
        }
    }

    ///
    /// Pops a frame, returning the parent frame, or none
    ///
    pub fn pop(self) -> Option<Frame> {
        let mut bitcode = self.bitcode;

        self.previous_frame.map(move |frame| {
            let mut frame = *frame;

            if bitcode.code.borrow().len() > 0 {
                if frame.bitcode.code.borrow().len() == 0 {
                    // No bitcode in the new frame, so just replace with the bitcode from the frame we're leaving
                    frame.bitcode = bitcode;
                } else {
                    // Insert our bitcode into the frame (frames generate new bitcode blocks)
                    let mut new_bitcode     = BitCodeBuffer::new();
                    bitcode.preceding       = Some(Rc::new(frame.bitcode));
                    new_bitcode.preceding   = Some(Rc::new(bitcode));
                    frame.bitcode           = new_bitcode; 
                }
            }

            frame
        })
    }
}