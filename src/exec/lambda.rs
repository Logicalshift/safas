use super::frame::*;
use super::frame_monad::*;

use crate::meta::*;

use std::sync::*;

///
/// Represents a function (and a closure)
/// 
/// Functions are called by putting their arguments in cell 0 (as a list) before resolving the monad
///
pub struct Lambda<Action: FrameMonad> {
    /// Monad describing the actions to perform in this function
    action: Action,

    /// The number of cells to allocate on the frame for this function
    num_cells: usize,

    /// The number of cells to fill with arguments for this function (loaded in to cells 1-args)
    arg_count: usize,
}

impl<Action: FrameMonad> Lambda<Action> {
    ///
    /// Creates a new lambda from a frame monad
    ///
    pub fn new(action: Action, num_cells: usize, arg_count: usize) -> Lambda<Action> {
        Lambda {
            action,
            num_cells,
            arg_count
        }
    }
}

impl<Action: FrameMonad> FrameMonad for Lambda<Action> {
    fn description(&self) -> String {
        let args = (0..self.arg_count).into_iter().map(|_| "_").collect::<Vec<_>>().join(" ");

        format!("(lambda ({}) {})", args, self.action.description())
    }

    fn resolve(&self, frame: Frame) -> (Frame, Arc<SafasCell>) {
        // Args in cell 0 from the calling frame
        let mut args        = Arc::clone(&frame.cells[0]);

        // Create the frame for the function call
        let mut frame       = Frame::new(self.num_cells, Some(frame));

        // Read the arguments
        let mut arg_pos     = 0;

        loop {
            // Stop if we run out of arguments
            if arg_pos > self.arg_count { 
                break; 
            }

            // Read the next argument from the list
            let (cell_value, next_arg) = if let SafasCell::List(car, cdr) = &*args {
                (Arc::clone(&car), Arc::clone(&cdr))
            } else {
                break;
            };

            // Store in the cell
            frame.cells[1+arg_pos]  = cell_value;

            // Move to the next argument
            args                    = next_arg;
            arg_pos                 += 1;
        }

        // Resolve the action (actually calling the function)
        let (frame, result) = self.action.resolve(frame);

        // Pop the frame we pushed for the action
        let frame = frame.pop().expect("Calling frame");

        (frame, result)
    }
}