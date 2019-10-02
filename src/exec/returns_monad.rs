use super::frame::*;
use super::frame_monad::*;

///
/// Decorates a frame monad to indicate it returns a monad
///
pub struct ReturnsMonad<T: FrameMonad>(pub T);

impl<T: FrameMonad> FrameMonad for ReturnsMonad<T> {
    type Binding = T::Binding;

    fn description(&self) -> String                             { self.0.description() }
    fn execute(&self, frame: Frame) -> (Frame, Self::Binding)   { self.0.execute(frame) }
    fn returns_monad(&self) -> bool                             { true }
}
