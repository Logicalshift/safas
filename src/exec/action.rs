use super::frame::*;
use super::frame_monad::*;

use crate::meta::*;

use std::sync::*;

///
/// An action that can be run against a frame
///
pub enum Action {
    /// Returns a value directly
    Value(Arc<SafasCell>),

    /// Reads a value from a cell
    CellValue(usize)
}

impl FrameMonad for Vec<Action> {
    fn resolve(&self, frame: Frame) -> (Frame, Arc<SafasCell>) {
        // Initial state
        let mut frame   = frame;
        let mut result  = Arc::new(SafasCell::Nil);

        // Perform each action in turn
        for action in self.iter() {
            use self::Action::*;

            match action {
                Value(cell)     => { result = Arc::clone(&cell); },
                CellValue(pos)  => { result = Arc::clone(&frame.cells[*pos]); }
            }
        }

        (frame, result)
    }
}