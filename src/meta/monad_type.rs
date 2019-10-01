use super::cell::*;
use super::cell_conversion::*;

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

///
/// Reference to a monad type
///
pub type MonadTypeRef = Arc<MonadType>;

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
    pub fn flat_map(&self, value: CellRef, frame: Frame) -> (Frame, RuntimeResult) {
        match &*self.flat_map {
            SafasCell::FrameMonad(action) => {
                let mut frame   = frame;
                frame.cells[0]  = value;
                action.resolve(frame)
            },
            _ => return (frame, Err(RuntimeError::NotAFunction(Arc::clone(&self.flat_map))))
        }
    }
}

/// Represents the flat_map function for a wrapped cell
pub struct WrapFlatMap(pub CellRef);

impl FrameMonad for WrapFlatMap {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        format!("##wrap({})", self.0.to_string())
    }

    fn resolve(&self, frame: Frame) -> (Frame, Self::Binding) {
        let args                    = ListTuple::<(CellRef, )>::try_from(frame.cells[0].clone());
        let args                    = match args { Ok(args) => args, Err(err) => return (frame, Err(err)) };
        let ListTuple((map_fn, ))   = args;

        // Argument should be a frame monad
        if let SafasCell::FrameMonad(map_fn) = &*map_fn {
            // Store the value in cell 0
            let WrapFlatMap(value)  = self;
            let value               = value.clone();
            let mut frame           = frame;
            frame.cells[0]          = value;

            // Result is the result of calling this function
            map_fn.resolve(frame)
        } else {
            // Not a monad
            return (frame, Err(RuntimeError::NotAFunction(map_fn)))
        }
    }
}
