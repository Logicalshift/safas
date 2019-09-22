use super::frame::*;
use super::frame_monad::*;
use super::runtime_error::*;

use crate::meta::*;

use std::sync::*;
use std::result::{Result};
use std::marker::{PhantomData};

///
/// Trait implemented by things that can be used as arguments to a function monad
///
pub trait FnArgs : Sized {
    /// Retrieves the function arguments from a frame
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError>;
}

///
/// Represents a monad created from a Rust function that operates on cells
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

impl FnArgs for Arc<SafasCell> {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        Ok(Arc::clone(&frame.cells[0]))
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

impl FnArgs for (Arc<SafasCell>, Arc<SafasCell>) {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        let args = frame.cells[0].to_vec().unwrap_or_else(|| vec![]);
        if args.len() > 2 {
            Err(RuntimeError::TooManyArguments(Arc::clone(&frame.cells[0])))
        } else if args.len() < 2 {
            Err(RuntimeError::NotEnoughArguments(Arc::clone(&frame.cells[0])))
        } else {
            Ok((Arc::clone(&args[0]), Arc::clone(&args[1])))
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
