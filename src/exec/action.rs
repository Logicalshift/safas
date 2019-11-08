use super::frame::*;
use super::frame_monad::*;
use super::runtime_error::*;

use crate::meta::*;

use smallvec::*;
use std::sync::*;
use std::iter::{FromIterator};

///
/// An action that can be run against a frame
///
#[derive(Clone, Debug)]
pub enum Action {
    /// Returns a value directly
    Value(CellRef),

    /// Reads a value from a cell
    CellValue(usize),

    /// Stores a value in a particular cell
    StoreCell(usize),

    /// Pushes a value onto the frame's stack
    Push,

    /// Pops a value from the stack
    Pop,

    /// Pushes a value onto the stack (and sets it as the current result)
    PushValue(CellRef),

    /// Pushes the content of a cell onto the stack (and sets it as the current result)
    PushCell(usize),

    /// Pops a number of values from the stack and turns them into a list
    PopList(usize),

    /// Pops a value from the stack to use as the CDR for the end of the list, then pops a number of values to generate the full list
    PopListWithCdr(usize),

    /// Calls the current value
    Call,

    /// Pops a number of values from the stack as a list, stores them in cell 0, then pops a further value and calls it
    PopCall(usize),

    /// Given a monad on the stack and a function as the current value, calls the monad's flat_map value
    FlatMap,

    /// Treats the current value as a monad, and calls the Wrap function
    Wrap,

    /// Calls flat_map on the monad on top of the stack with a function that returns the current value, pushing the result
    /// back onto the stack
    Next,

    /// Move ahead or behind x actions
    Jump(isize),

    /// If the value on top of the stack is not true (ie, any value other than =t), move ahead/behind x actions
    JumpIfFalse(isize)
}

impl Action {
    ///
    /// Performs simple peephole optimisation on a series of actions, combining operations that are easy to combine
    ///
    pub fn peephole_optimise<ActionIter: IntoIterator<Item=Action>, Target: FromIterator<Action>>(actions: ActionIter) -> Target {
        let mut actions = actions.into_iter().fuse();

        // The window represents the instructions we're inspecting
        let mut window = (None, None, None, actions.next());
        let mut result = vec![];
        
        // Actions are read in from the right of the window, and are 
        loop {
            // Stop once there are no more actions to process
            if let (None, None, None, None) = window {
                break;
            }

            match window {
                (action1, action2, Some(Action::Value(val)), Some(Action::Push)) => {
                    // Value, Push => PushValue
                    window = (None, action1, action2, Some(Action::PushValue(val)));
                }

                (action1, action2, Some(Action::CellValue(cell_id)), Some(Action::Push)) => {
                    // CellValue, Push => PushCell
                    window = (None, action1, action2, Some(Action::PushCell(cell_id)));
                }

                (Some(Action::PopList(arg_count)), Some(Action::StoreCell(0)), Some(Action::Pop), Some(Action::Call)) => {
                    window = (None, None, None, Some(Action::PopCall(arg_count)));
                }

                (action1, action2, Some(Action::Push), Some(Action::Pop)) => {
                    window = (None, None, action1, action2);
                },

                (action1, action2, Some(Action::Pop), Some(Action::Push)) => {
                    // Slight behaviour difference: the result is not the popped value after this
                    window = (None, None, action1, action2);
                },

                (action1, action2, action3, action4) => {
                    // Actions leaving the window are added to the result
                    if let Some(action) = action1 { 
                        result.push(action)
                    }

                    // Update the window
                    window = (action2, action3, action4, actions.next());
                }
            }
        }

        // Convert the result back into a series of actions
        result.into_iter().collect()
    }
}

impl FrameMonad for SmallVec<[Action; 8]> {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        format!("{:?}", self)
    }

    fn execute(&self, frame: Frame) -> (Frame, RuntimeResult) {
        // We just convert to a normal vec and run the actions from there
        let actions = self.iter().cloned().collect::<Vec<_>>();
        actions.execute(frame)
    }
}

impl FrameMonad for Vec<Action> {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        format!("{:?}", self)
    }

    fn execute(&self, frame: Frame) -> (Frame, RuntimeResult) {
        // Initial state
        let mut frame   = frame;
        let mut result  = NIL.clone();
        let mut ip      = 0;

        // Perform each action in turn
        loop {
            use self::Action::*;

            // Stop when we get past the end
            if ip >= self.len() { break; }

            // Read the next action
            let action = &self[ip];

            match action {
                Push                        => { frame.stack.push(Arc::clone(&result)); },
                Value(cell)                 => { result = Arc::clone(&cell); },
                PushValue(cell)             => { result = Arc::clone(&cell); frame.stack.push(Arc::clone(&result)); }
                CellValue(pos)              => { result = Arc::clone(&frame.cells[*pos]); },
                PushCell(pos)               => { result = Arc::clone(&frame.cells[*pos]); frame.stack.push(Arc::clone(&result)); }
                StoreCell(cell)             => { frame.cells[*cell] = Arc::clone(&result); },
                Pop                         => { 
                    if let Some(value) = frame.stack.pop() {
                        result = value;
                    } else {
                        return (frame, Err(RuntimeError::StackIsEmpty));
                    }
                }

                PopList(num_cells)          => { 
                    result = NIL.clone();
                    for _ in 0..*num_cells {
                        let val = if let Some(val) = frame.stack.pop() {
                            val
                        } else {
                            return (frame, Err(RuntimeError::StackIsEmpty));
                        };
                        result = SafasCell::List(val, result).into();
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
                        result = SafasCell::List(val, result).into();
                    }
                }

                Call                        => { 
                    match &*result {
                        SafasCell::FrameMonad(action)    => { 
                            let (new_frame, new_result) = action.execute(frame);
                            if let Ok(new_result) = new_result {
                                frame                   = new_frame;
                                result                  = new_result;
                            } else {
                                return (new_frame, new_result);
                            }
                        },
                        _                           => return (frame, Err(RuntimeError::NotAFunction(Arc::clone(&result))))
                    }
                }

                PopCall(num_cells)          => {
                    // Pop cells
                    result = NIL.clone();
                    for _ in 0..*num_cells {
                        let val = if let Some(val) = frame.stack.pop() {
                            val
                        } else {
                            return (frame, Err(RuntimeError::StackIsEmpty));
                        };
                        result = SafasCell::List(val, result).into();
                    }

                    // Store in frame 0
                    frame.cells[0] = result;

                    // Pop a function
                    result = if let Some(function) = frame.stack.pop() {
                        function
                    } else {
                        return (frame, Err(RuntimeError::StackIsEmpty));
                    };

                    // Call it
                    match &*result {
                        SafasCell::FrameMonad(action)    => { 
                            let (new_frame, new_result) = action.execute(frame);
                            if let Ok(new_result) = new_result {
                                frame                   = new_frame;
                                result                  = new_result;
                            } else {
                                return (new_frame, new_result);
                            }
                        },
                        _                           => return (frame, Err(RuntimeError::NotAFunction(Arc::clone(&result))))
                    }
                }

                FlatMap                     => {
                    let monad = if let Some(monad) = frame.stack.pop() {
                        monad
                    } else {
                        return (frame, Err(RuntimeError::StackIsEmpty));
                    };

                    match &*monad {
                        // Result contains the map function
                        SafasCell::Monad(value, monad_type) => {
                            // Call the monad's flat_map function with this cell value
                            let (new_frame, new_result) = monad_type.flat_map(value.clone(), result, frame);
                            if let Ok(new_result) = new_result {
                                frame                   = new_frame;
                                result                  = new_result;
                            } else {
                                return (new_frame, new_result);
                            }
                        },

                        _ => return (frame, Err(RuntimeError::NotAMonad(Arc::clone(&result))))
                    }
                },

                Wrap => {
                    let wrap            = WrapFlatMap(result);
                    let wrap_flat_map   = SafasCell::FrameMonad(Box::new(wrap));
                    let monad_type      = MonadType::new(wrap_flat_map.into());

                    result              = SafasCell::Monad(NIL.clone(), monad_type).into();
                }

                Next => {
                    let monad = if let Some(monad) = frame.stack.pop() {
                        monad
                    } else {
                        return (frame, Err(RuntimeError::StackIsEmpty));
                    };
                    let map_fn = SafasCell::FrameMonad(Box::new(wrap_frame(Ok(result)))).into();

                    match &*monad {
                        // Result contains the map function
                        SafasCell::Monad(value, monad_type) => {
                            // Call the monad's flat_map function with this cell value
                            let (new_frame, new_result) = monad_type.flat_map(value.clone(), map_fn, frame);
                            if let Ok(new_result) = new_result {
                                frame                   = new_frame;
                                result                  = new_result;
                                frame.stack.push(result.clone());
                            } else {
                                return (new_frame, new_result);
                            }
                        },

                        _ => return (frame, Err(RuntimeError::NotAMonad(Arc::clone(&monad))))
                    }
                },

                Jump(offset) => {
                    ip = ((ip as isize) + offset) as usize;
                    continue;
                },

                JumpIfFalse(offset) => {
                    if let SafasCell::Boolean(true) = &*result {
                        ip = ((ip as isize) + offset) as usize;
                        continue;
                    }
                }
            }

            // Next instruction
            ip += 1;
        }

        (frame, Ok(result))
    }
}
