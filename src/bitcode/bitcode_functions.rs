use super::code::*;
use super::bitcode_monad::*;

use crate::meta::*;
use crate::exec::*;

use std::convert::*;

lazy_static! {
    /// The bitcode flat_map function
    pub (super) static ref BITCODE_FLAT_MAP: CellRef = CellRef::new(SafasCell::FrameMonad(Box::new(bitcode_flat_map_fn())));
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
        let bitcode_monad = BitCodeMonad::from_cell(&args.monad_value);

        // Replace the empty bitcode monad with a 'full' bitcode monad
        let bitcode_monad = bitcode_monad.unwrap_or_else(|| BitCodeMonad::empty());

        // Fetch the map function
        let map_fn = match &*args.map_fn {
            SafasCell::FrameMonad(monad_fn) => monad_fn,
            _                               => return (frame, Err(RuntimeError::NotAFunction(args.map_fn)))
        };

        // Applying the map function should return the updated monad
        let monad_value         = args.monad_value.clone();
        let next                = bitcode_monad.flat_map(move |val| {
            let mut frame           = Frame::new(1, None);
            frame.cells[0]          = SafasCell::List(monad_value.clone(), val).into();
            let (_frame, next)      = map_fn.execute(frame);

            let next                = match next { Ok(next) => next, Err(err) => return Err(err) };

            // Result of the map funciton should either be a bitcode monad or nil
            let next_monad          = BitCodeMonad::from_cell(&next);

            let next_monad          = match next_monad {
                Some(next_monad)    => next_monad,
                None                => return Err(RuntimeError::MismatchedMonad(next))
            };

            Ok(next_monad)
        });
        let next                = match next { Ok(next) => next, Err(err) => return (frame, Err(err)) };

        // Wrap the resulting monad to generate the return value
        let result = SafasCell::Any(Box::new(next)).into();
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

/// The 'D' data output function
pub fn d_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    ReturnsMonad(FnMonad::from(|numbers: Vec<SafasNumber>| {
        use self::SafasNumber::*;
        use self::BitCode::Bits;

        // Generate the bitcode
        let bitcode = numbers.into_iter().map(|num| match num {
            Plain(val)                      => Bits(32, val),
            BitNumber(bit_count, val)       => Bits(bit_count, val),
            SignedBitNumber(bit_count, val) => Bits(bit_count, val as u128)
        });

        // Create a bitcode monad cell
        let bitcode_monad   = BitCodeMonad::write_bitcode(bitcode);
        bitcode_monad.to_cell()
    }))
}
