use super::symbol_bindings::*;
use crate::exec::*;

use std::marker::{PhantomData};

///
/// The binding monad describes how to bind a program against its symbols
///
pub trait BindingMonad {
    type Frame: FrameMonad;

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Frame);
}

///
/// Binding monad that returns a constant value
///
struct ReturnValue<Frame: FrameMonad+Clone> {
    value: Frame
}

impl<Frame: FrameMonad+Clone> BindingMonad for ReturnValue<Frame> {
    type Frame=Frame;

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Frame) {
        (bindings, self.value.clone())
    }
}

///
/// Wraps a value in a binding monad
///
pub fn wrap_binding<Frame: FrameMonad+Clone>(value: Frame) -> impl BindingMonad<Frame=Frame> {
    ReturnValue { value }
}

struct FlatMapValue<InputMonad, OutputMonad, NextFn> {
    input:  InputMonad,
    next:   NextFn,
    output: PhantomData<OutputMonad>
}

impl<InputMonad, OutputMonad, NextFn> BindingMonad for FlatMapValue<InputMonad, OutputMonad, NextFn>
where   InputMonad:     BindingMonad,
        OutputMonad:    BindingMonad,
        NextFn:         Fn(InputMonad::Frame) -> OutputMonad {
    type Frame = OutputMonad::Frame;

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, OutputMonad::Frame) {
        let (bindings, value)   = self.input.resolve(bindings);
        let next                = (self.next)(value);
        next.resolve(bindings)
    }
}

///
/// That flat_map function for a binding monad
///
pub fn flat_map_binding<InputMonad: BindingMonad, OutputMonad: BindingMonad, NextFn: Fn(InputMonad::Frame) -> OutputMonad>(action: NextFn, monad: InputMonad) -> impl BindingMonad {
    FlatMapValue {
        input:  monad,
        next:   action,
        output: PhantomData
    }
}
