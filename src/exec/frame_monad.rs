use super::frame::*;
use super::runtime_error::*;
use crate::meta::*;

use std::sync::*;
use std::result::{Result};
use std::marker::{PhantomData};

///
/// Trait implemented by things that represent a 'frame monad'
///
pub trait FrameMonad : Send+Sync {
    /// Resolves this monad against a frame
    fn resolve(&self, frame: Frame) -> (Frame, Result<Arc<SafasCell>, RuntimeError>);

    /// Retrieves a description of this monad when we need to display it to the user
    fn description(&self) -> String { format!("<frame_monad#{:p}>", self) }
}

///
/// Frame monad that returns a constant value
///
struct ReturnValue {
    value: Arc<SafasCell>
}

impl FrameMonad for ReturnValue {
    fn resolve(&self, frame: Frame) -> (Frame, Result<Arc<SafasCell>, RuntimeError>) {
        (frame, Ok(self.value.clone()))
    }
}

///
/// Wraps a value in a frame monad
///
pub fn wrap_frame(value: Arc<SafasCell>) -> impl FrameMonad {
    ReturnValue { value }
}

struct FlatMapValue<InputMonad, OutputMonad, NextFn> {
    input:  InputMonad,
    next:   NextFn,
    output: PhantomData<OutputMonad>
}

impl<InputMonad, OutputMonad, NextFn> FrameMonad for FlatMapValue<InputMonad, OutputMonad, NextFn>
where   InputMonad:     FrameMonad,
        OutputMonad:    FrameMonad,
        NextFn:         Send+Sync+Fn(Result<Arc<SafasCell>, RuntimeError>) -> OutputMonad {
    fn resolve(&self, frame: Frame) -> (Frame, Result<Arc<SafasCell>, RuntimeError>) {
        let (frame, value)  = self.input.resolve(frame);
        let next            = (self.next)(value);
        next.resolve(frame)
    }

    fn description(&self) -> String { format!("{} >>= <fn#{:p}>", self.input.description(), &self.next) }
}

///
/// That flat_map function for a frame monad (appends 'action' to the series of actions represented by 'monad')
///
pub fn flat_map_frame<InputMonad: FrameMonad, OutputMonad: FrameMonad, NextFn: Send+Sync+Fn(Result<Arc<SafasCell>, RuntimeError>) -> OutputMonad>(action: NextFn, monad: InputMonad) -> impl FrameMonad {
    FlatMapValue {
        input:  monad,
        next:   action,
        output: PhantomData
    }
}
