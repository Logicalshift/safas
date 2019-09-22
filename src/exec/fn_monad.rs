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
///     let car = FnMonad::from(|SafasList(car, _cdr)| { Arc::clone(&car) })
/// ```
/// 
/// Takes advantage of the conversion from a cell to a `SafasList`, saving the effort of needing to extract
/// the car and cdr values.
///
pub struct FnMonad<Fun, Args> {
    action:     Fun,
    arguments:  PhantomData<Args>
}

impl<Fun, Args> From<Fun> for FnMonad<Fun, Args>
where   Fun:    Send+Sync+Fn(Args) -> Arc<SafasCell>,
        Args:   Send+Sync+FnArgs {
    fn from(fun: Fun) -> FnMonad<Fun, Args> {
        FnMonad {
            action:     fun,
            arguments:  PhantomData
        }
    }
}

impl<Fun, Args> FrameMonad for FnMonad<Fun, Args>
where   Fun:    Send+Sync+Fn(Args) -> Arc<SafasCell>,
        Args:   Send+Sync+FnArgs {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        format!("##fn#{:p}##", &self.action)
    }

    fn resolve(&self, frame: Frame) -> (Frame, Self::Binding) {
        let args    = Args::args_from_frame(&frame);
        let args    = match args { Ok(args) => args, Err(err) => return (frame, Err(err)) };
        let result  = (self.action)(args);

        (frame, Ok(result))
    }
}

impl FnArgs for Vec<Arc<SafasCell>> {
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

impl FnArgs for Arc<SafasCell> {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        if args.len() > 1 {
            Err(RuntimeError::TooManyArguments(Arc::clone(&frame.cells[0])))
        } else if args.len() < 1 {
            Err(RuntimeError::NotEnoughArguments(Arc::clone(&frame.cells[0])))
        } else {
            Ok(Arc::clone(&args[0]))
        }
    }
}

impl<'a, A1, A2> FnArgs for (A1, A2)
where   A1: TryFrom<Arc<SafasCell>>,
        A2: TryFrom<Arc<SafasCell>>,
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
where   A1: TryFrom<Arc<SafasCell>>,
        A2: TryFrom<Arc<SafasCell>>,
        A3: TryFrom<Arc<SafasCell>>,
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

impl FnArgs for SafasList {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        if args.len() > 1 {
            Err(RuntimeError::TooManyArguments(Arc::clone(&frame.cells[0])))
        } else if args.len() < 1 {
            Err(RuntimeError::NotEnoughArguments(Arc::clone(&frame.cells[0])))
        } else {
            Ok(SafasList::try_from(&args[0])?)
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
