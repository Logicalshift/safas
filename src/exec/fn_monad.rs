use super::frame::*;
use super::frame_monad::*;
use super::runtime_error::*;

use crate::meta::*;

use std::sync::*;
use std::result::{Result};
use std::marker::{PhantomData};
use std::convert::*;

///
/// Trait implemented by things that can be used as arguments to a function monad
///
pub trait FnArgs : Sized {
    /// Retrieves the function arguments from a frame
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError>;
}

///
/// FnMonad is the main way to import Rust closures into SAFAS
/// 
/// Implement the `FnArgs` trait on a type in order to interpret the raw cell into the arguments accepted
/// by your function and produce errors if the type is incorrect. Several implemenations are provided already, 
/// allowing for creating monads out of a wide variety of types. For example:Arc
/// 
/// ```
///     let car = FnMonad::from(|(SafasList(car, _cdr), )| { Arc::clone(&car) })
/// ```
/// 
/// This takes advantage of the conversion from a cell to a `SafasList`, saving the effort of needing to extract
/// the car and cdr values. Note the use of the rare single-tuple syntax - `(foo,)` here: all the tuple cases convert
/// using `TryFrom<CellRef>`. Direct implementions of `FnArgs` don't require the tuple.
///
pub struct FnMonad<Fun, Args> {
    action:     Fun,
    arguments:  PhantomData<Args>
}

impl<Fun, Args> From<Fun> for FnMonad<Fun, Args>
where   Fun:    Send+Sync+Fn(Args) -> CellRef,
        Args:   Send+Sync+FnArgs {
    fn from(fun: Fun) -> FnMonad<Fun, Args> {
        FnMonad {
            action:     fun,
            arguments:  PhantomData
        }
    }
}

impl<Fun, Args> FrameMonad for FnMonad<Fun, Args>
where   Fun:    Send+Sync+Fn(Args) -> CellRef,
        Args:   Send+Sync+FnArgs {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        format!("##fn#{:p}##", &self)
    }

    fn resolve(&self, frame: Frame) -> (Frame, Self::Binding) {
        let args    = Args::args_from_frame(&frame);
        let args    = match args { Ok(args) => args, Err(err) => return (frame, Err(err)) };
        let result  = (self.action)(args);

        (frame, Ok(result))
    }
}

impl FnArgs for Vec<CellRef> {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        Ok(args)
    }
}

impl FnArgs for () {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        if args.len() != 0 {
            Err(RuntimeError::TooManyArguments(Arc::clone(&frame.cells[0])))
        } else {
            Ok(())
        }
    }
}

impl<T> FnArgs for (T,)
where   T: TryFrom<CellRef>,
        RuntimeError: From<T::Error> {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        if args.len() > 1 {
            Err(RuntimeError::TooManyArguments(Arc::clone(&frame.cells[0])))
        } else if args.len() < 1 {
            Err(RuntimeError::NotEnoughArguments(Arc::clone(&frame.cells[0])))
        } else {
            let mut args    = args;
            let args        = args.pop().unwrap();

            Ok((T::try_from(args)?, ))
        }
    }
}

impl<'a, A1, A2> FnArgs for (A1, A2)
where   A1: TryFrom<CellRef>,
        A2: TryFrom<CellRef>,
        RuntimeError: From<A1::Error>,
        RuntimeError: From<A2::Error> {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        if args.len() > 2 {
            Err(RuntimeError::TooManyArguments(Arc::clone(&frame.cells[0])))
        } else if args.len() < 2 {
            Err(RuntimeError::NotEnoughArguments(Arc::clone(&frame.cells[0])))
        } else {
            let mut args    = args;
            let a2          = args.pop().unwrap();
            let a1          = args.pop().unwrap();

            Ok((A1::try_from(a1)?, A2::try_from(a2)?))
        }
    }
}

impl<'a, A1, A2, A3> FnArgs for (A1, A2, A3)
where   A1: TryFrom<CellRef>,
        A2: TryFrom<CellRef>,
        A3: TryFrom<CellRef>,
        RuntimeError: From<A1::Error>,
        RuntimeError: From<A2::Error>,
        RuntimeError: From<A3::Error> {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        if args.len() > 3 {
            Err(RuntimeError::TooManyArguments(Arc::clone(&frame.cells[0])))
        } else if args.len() < 3 {
            Err(RuntimeError::NotEnoughArguments(Arc::clone(&frame.cells[0])))
        } else {
            let mut args    = args;
            let a3          = args.pop().unwrap();
            let a2          = args.pop().unwrap();
            let a1          = args.pop().unwrap();

            Ok((A1::try_from(a1)?, A2::try_from(a2)?, A3::try_from(a3)?))
        }
    }
}

impl FnArgs for VarArgs {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        Ok(VarArgs(Arc::clone(&frame.cells[0])))
    }
}

///
/// Represents the arguments to a monad flat_map function
///
pub struct FlatMapArgs {
    /// The value wrapped by the monad
    pub monad_value:    CellRef,

    /// The map function to apply to the value
    pub map_fn:         CellRef
}

impl FnArgs for FlatMapArgs {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        match &*frame.cells[0] {
            SafasCell::List(car, cdr)   => Ok(FlatMapArgs { monad_value: car.clone(), map_fn: cdr.clone() }),
            _                           => Err(RuntimeError::NotAMonad(frame.cells[0].clone()))
        }
    }
}

impl TryFrom<CellRef> for FlatMapArgs {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { 
        match &*cell {
            SafasCell::List(car, cdr)   => Ok(FlatMapArgs { monad_value: car.clone(), map_fn: cdr.clone() }),
            _                           => Err(RuntimeError::NotAMonad(cell))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn frame_with_args(args: SafasCell) -> Frame {
        let mut frame = Frame::new(1, None);
        frame.cells[0] = Arc::new(args);
        frame
    }

    #[test]
    fn args_from_nothing() {
        assert!(<()>::args_from_frame(&frame_with_args(SafasCell::Nil)).unwrap() == ());
    }
}
