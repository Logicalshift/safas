use super::symbol_bindings::*;
use crate::meta::*;

use std::marker::{PhantomData};

///
/// The binding monad describes how to bind a program against its symbols
///
pub trait BindingMonad {
    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, SafasCell);
}

///
/// Binding monad that returns a constant value
///
struct ReturnValue {
    value: SafasCell
}

impl BindingMonad for ReturnValue {
    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, SafasCell) {
        (bindings, self.value.clone())
    }
}

///
/// Wraps a value in a binding monad
///
pub fn wrap_binding(value: SafasCell) -> impl BindingMonad {
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
        NextFn:         Fn(SafasCell) -> OutputMonad {
    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, SafasCell) {
        let (bindings, value)   = self.input.resolve(bindings);
        let next                = (self.next)(value);
        next.resolve(bindings)
    }
}

///
/// That flat_map function for a binding monad
///
pub fn flat_map_binding<InputMonad: BindingMonad, OutputMonad: BindingMonad, NextFn: Fn(SafasCell) -> OutputMonad>(action: NextFn, monad: InputMonad) -> impl BindingMonad {
    FlatMapValue {
        input:  monad,
        next:   action,
        output: PhantomData
    }
}
