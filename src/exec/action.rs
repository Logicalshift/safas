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
    CellValue(usize),

    /// Stores a value in a particular cell
    StoreCell(usize),

    /// Pushes a value onto the frame's stack
    Push,

    /// Pops a value from the stack
    Pop,

    /// Pops a number of values from the stack and turns them into a list
    PopList(usize),

    /// Pops a value from the stack to use as the CDR for the end of the list, then pops a number of values to generate the full list
    PopListWithCdr(usize),

    /// Calls the current value
    Call
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
                Value(cell)                 => { result = Arc::clone(&cell); },
                CellValue(pos)              => { result = Arc::clone(&frame.cells[*pos]); },
                StoreCell(cell)             => { frame.cells[*cell] = Arc::clone(&result); },
                Push                        => { frame.stack.push(Arc::clone(&result)); },
                Pop                         => { result = frame.stack.pop().expect("Stack empty"); }

                PopList(num_cells)          => { 
                    result = Arc::new(SafasCell::Nil);
                    for _ in 0..*num_cells {
                        let val = frame.stack.pop().expect("Stack empty");
                        result = Arc::new(SafasCell::List(val, result));
                    }
                }

                PopListWithCdr(num_cells)   => { 
                    result = frame.stack.pop().expect("Stack empty");
                    for _ in 0..*num_cells {
                        let val = frame.stack.pop().expect("Stack empty");
                        result = Arc::new(SafasCell::List(val, result));
                    }
                }

                Call                        => { 
                    match &*result {
                        SafasCell::Monad(action)    => { 
                            let (new_frame, new_result) = action.resolve(frame);
                            frame                       = new_frame;
                            result                      = new_result;
                        },
                        _                           => panic!("Value is not a function")
                    }
                }
            }
        }

        (frame, result)
    }
}