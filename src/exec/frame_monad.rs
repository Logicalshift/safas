use super::frame::*;

use std::marker::{PhantomData};

///
/// Trait implemented by things that represent a 'frame monad'
///
pub trait FrameMonad : Send+Sync {
    type Binding;

    /// Resolves this monad against a frame
    fn resolve(&self, frame: Frame) -> (Frame, Self::Binding);

    /// Retrieves a description of this monad when we need to display it to the user
    fn description(&self) -> String { format!("<frame_monad#{:p}>", self) }
}

impl FrameMonad for () {
    type Binding = ();

    fn description(&self) -> String { "##nop##".to_string() }
    fn resolve(&self, frame: Frame) -> (Frame, ()) { (frame, ()) }
}

///
/// Frame monad that returns a constant value
///
struct ReturnValue<Binding: Clone> {
    value: Binding
}

impl<Binding: Clone+Send+Sync> FrameMonad for ReturnValue<Binding> {
    type Binding=Binding;

    fn resolve(&self, frame: Frame) -> (Frame, Binding) {
        (frame, self.value.clone())
    }
}

///
/// Wraps a value in a frame monad
///
pub fn wrap_frame<Binding: Clone+Send+Sync>(value: Binding) -> impl FrameMonad<Binding=Binding> {
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
        NextFn:         Send+Sync+Fn(InputMonad::Binding) -> OutputMonad {
    type Binding = OutputMonad::Binding;

    fn resolve(&self, frame: Frame) -> (Frame, OutputMonad::Binding) {
        let (frame, value)  = self.input.resolve(frame);
        let next            = (self.next)(value);
        next.resolve(frame)
    }

    fn description(&self) -> String { format!("{} >>= <fn#{:p}>", self.input.description(), &self.next) }
}

///
/// That flat_map function for a frame monad (appends 'action' to the series of actions represented by 'monad')
///
pub fn flat_map_frame<InputMonad: FrameMonad, OutputMonad: FrameMonad, NextFn: Send+Sync+Fn(InputMonad::Binding) -> OutputMonad>(action: NextFn, monad: InputMonad) -> impl FrameMonad<Binding=OutputMonad::Binding> {
    FlatMapValue {
        input:  monad,
        next:   action,
        output: PhantomData
    }
}
