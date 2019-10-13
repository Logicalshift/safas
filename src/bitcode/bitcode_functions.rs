use super::bitcode_monad::*;

use crate::meta::*;
use crate::exec::*;

use std::convert::*;

lazy_static! {
    /// The bitcode flat_map function
    static ref BITCODE_FLAT_MAP: CellRef = CellRef::new(SafasCell::FrameMonad(Box::new(bitcode_flat_map_fn())));
}

struct BitCodeFlatMap;

impl FrameMonad for BitCodeFlatMap {
    type Binding = RuntimeResult;

    /// Executes this monad against a frame
    fn execute(&self, frame: Frame) -> (Frame, Self::Binding) {
        // Arguments should be flat_map arguments
        let args = frame.cells[0].clone();
        let args = match FlatMapArgs::try_from(args) { Ok(args) => args, Err(err) => return (frame, Err(err)) };

        // The monad value should be a bitcode monad of some kind
        let monad = match &*args.monad_value {
            SafasCell::Any(any_val) => any_val.downcast_ref::<BitCodeMonad>().cloned(),
            SafasCell::Nil          => Some(BitCodeMonad::empty()),
            _                       => None
        };

        // Replace the empty bitcode monad with a 'full' bitcode monad
        let monad = monad.unwrap_or_else(|| BitCodeMonad::empty());

        // Fetch the map function
        let map_fn = match &*args.map_fn {
            SafasCell::FrameMonad(monad_fn) => monad_fn,
            _                               => return (frame, Err(RuntimeError::NotAFunction(args.map_fn)))
        };

        // Applying the map function should return the result
        /*
        let result = monad.flat_map(move |val| {
            let frame = Frame::new(0, None);

            monad_fn.execute(frame);
        });
        */
        let result = BitCodeMonad::empty();     // TODO!

        // Wrap the resulting monad to generate the return value
        let result = SafasCell::Any(Box::new(result)).into();
        (frame, Ok(SafasCell::Monad(result, MonadType::new(BITCODE_FLAT_MAP.clone())).into()))
    }

    /// Retrieves a description of this monad when we need to display it to the user
    fn description(&self) -> String { format!("##bitcode_flatmap##") }

    /// True if the return value of this function should be treated as a monad by the binder
    fn returns_monad(&self) -> bool { true }
}

///
/// Returns the flat map function to attach to the bitcode monad
///
pub fn bitcode_flat_map_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    BitCodeFlatMap
}
