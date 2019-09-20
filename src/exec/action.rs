use super::frame::*;
use super::frame_monad::*;
use super::runtime_error::*;

use crate::meta::*;

use smallvec::*;
use std::sync::*;

///
/// An action that can be run against a frame
///
#[derive(Clone, Debug)]
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

impl FrameMonad for SmallVec<[Action; 8]> {
    type Binding = RuntimeResult;

    fn resolve(&self, frame: Frame) -> (Frame, RuntimeResult) {
        // We just convert to a normal vec and run the actions from there
        let actions = self.iter().cloned().collect::<Vec<_>>();
        actions.resolve(frame)
    }
}

impl FrameMonad for Vec<Action> {
    type Binding = RuntimeResult;

    fn resolve(&self, frame: Frame) -> (Frame, RuntimeResult) {
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
                Pop                         => { 
                    if let Some(value) = frame.stack.pop() {
                        result = value;
                    } else {
                        return (frame, Err(RuntimeError::StackIsEmpty));
                    }
                }

                PopList(num_cells)          => { 
                    result = Arc::new(SafasCell::Nil);
                    for _ in 0..*num_cells {
                        let val = if let Some(val) = frame.stack.pop() {
                            val
                        } else {
                            return (frame, Err(RuntimeError::StackIsEmpty));
                        };
                        result = Arc::new(SafasCell::List(val, result));
                    }
                }

                PopListWithCdr(num_cells)   => { 
                    if let Some(value) = frame.stack.pop() {
                        result = value;
                    } else {
                        return (frame, Err(RuntimeError::StackIsEmpty));
                    };

                    for _ in 0..*num_cells {
                        let val = if let Some(val) = frame.stack.pop() {
                            val
                        } else {
                            return (frame, Err(RuntimeError::StackIsEmpty));
                        };
                        result = Arc::new(SafasCell::List(val, result));
                    }
                }

                Call                        => { 
                    match &*result {
                        SafasCell::Monad(action)    => { 
                            let (new_frame, new_result) = action.resolve(frame);
                            if let Ok(new_result) = new_result {
                                frame                   = new_frame;
                                result                  = new_result;
                            } else {
                                return (new_frame, new_result);
                            }
                        },
                        _                           => return (frame, Err(RuntimeError::NotAFunction))
                    }
                }
            }
        }

        (frame, Ok(result))
    }
}
