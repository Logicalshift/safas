use super::cell::*;

use crate::exec::*;

use std::sync::*;
use std::convert::*;

///
/// Represents the type of a monad cell
/// 
/// These are treated specially when binding functions (or macros) in that they turn their parent
/// function into a similar monad derived from this one.
///
pub struct MonadType {
    /// Represents the flat_map function. Should be `fn (fn y -> Monad) -> Monad`, extracting the value contained within this monad
    flat_map: CellRef,
}

impl MonadType {
    ///
    /// Creates a new monad from a cell representing a flat_map function
    ///
    pub fn new(flat_map: CellRef) -> MonadType {
        MonadType {
            flat_map
        }
    }

    ///
    /// Creates a string representation of this monad type
    ///
    pub fn to_string(&self) -> String {
        format!("(flat_map: {})", self.flat_map.to_string())
    }

    ///
    /// Performs the flat_map operation on a monad value
    ///
    pub fn flat_map(&self, monad: CellRef, map_fn: CellRef, frame: Frame) -> (Frame, RuntimeResult) {
        match &*self.flat_map {
            SafasCell::FrameMonad(action) => {
                // Call the flat map function with the monad and the supplied mapping function
                let mut frame   = frame;
                frame.cells[0]  = SafasCell::List(monad, map_fn).into();
                action.execute(frame)
            },
            _ => return (frame, Err(RuntimeError::NotAFunction(Arc::clone(&self.flat_map))))
        }
    }

    ///
    /// Performs the 'next' operation (flat_map where the value is discarded and the specified monad is returned)
    ///
    pub fn next(&self, monad: CellRef, next_monad: CellRef, frame: Frame) -> (Frame, RuntimeResult) {
        let map_fn = SafasCell::FrameMonad(Box::new(wrap_frame(Ok(next_monad)))).into();
        self.flat_map(monad, map_fn, frame)
    }
}

/// Represents the flat_map function for a wrapped cell
pub struct WrapFlatMap(pub CellRef);

impl FrameMonad for WrapFlatMap {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        format!("##wrap({})", self.0.to_string())
    }

    fn execute(&self, frame: Frame) -> (Frame, Self::Binding) {
        let args                    = FlatMapArgs::try_from(frame.cells[0].clone());
        let args                    = match args { Ok(args) => args, Err(err) => return (frame, Err(err)) };

        // Argument should be a frame monad
        if let SafasCell::FrameMonad(map_fn) = &*args.map_fn {
            // Store the value in cell 0
            let WrapFlatMap(value)  = self;
            let value               = value.clone();
            let mut frame           = frame;
            frame.cells[0]          = SafasCell::List(value, NIL.clone()).into();

            // Result is the result of calling this function
            map_fn.execute(frame)
        } else {
            // Not a monad
            return (frame, Err(RuntimeError::NotAFunction(args.map_fn)))
        }
    }
}
