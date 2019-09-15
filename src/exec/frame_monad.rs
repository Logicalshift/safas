use super::frame::*;
use crate::meta::*;

use std::marker::{PhantomData};

///
/// Trait implemented by things that represent a 'frame monad'
///
pub trait FrameMonad {
    /// Resolves this monad against a frame
    fn resolve(&self, frame: Frame) -> (Frame, SafasCell);
}

///
/// Frame monad that returns a constant value
///
struct ReturnValue {
    value: SafasCell
}

impl FrameMonad for ReturnValue {
    fn resolve(&self, frame: Frame) -> (Frame, SafasCell) {
        (frame, self.value.clone())
    }
}

///
/// Wraps a value in a frame monad
///
pub fn wrap_frame(value: SafasCell) -> impl FrameMonad {
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
        NextFn:         Fn(SafasCell) -> OutputMonad {
    fn resolve(&self, frame: Frame) -> (Frame, SafasCell) {
        let (frame, value)  = self.input.resolve(frame);
        let next            = (self.next)(value);
        next.resolve(frame)
    }
}

///
/// That flat_map function for a frame monad (appends 'action' to the series of actions represented by 'monad')
///
pub fn flat_map_frame<InputMonad: FrameMonad, OutputMonad: FrameMonad, NextFn: Fn(SafasCell) -> OutputMonad>(action: NextFn, monad: InputMonad) -> impl FrameMonad {
    FlatMapValue {
        input:  monad,
        next:   action,
        output: PhantomData
    }
}
