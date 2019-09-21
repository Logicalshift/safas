use super::bind_error::*;
use super::symbol_bindings::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::marker::{PhantomData};

///
/// The binding monad describes how to bind a program against its symbols
///
pub trait BindingMonad : Send+Sync {
    type Binding;

    ///
    /// Resolves this monad
    ///
    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding);

    ///
    /// Returns a string that describes what this monad does
    ///
    fn description(&self) -> String { "<syntax>".to_string() }
}

impl BindingMonad for () {
    type Binding = ();

    fn description(&self) -> String { "##nop##".to_string() }
    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, ()) { (bindings, ()) }
}

///
/// Binding monad that returns a constant value
///
struct ReturnValue<Binding: Clone> {
    value: Binding
}

impl<Binding: Send+Sync+Clone> BindingMonad for ReturnValue<Binding> {
    type Binding=Binding;

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Binding) {
        (bindings, self.value.clone())
    }
}

///
/// Wraps a value in a binding monad
///
pub fn wrap_binding<Binding: Send+Sync+Clone>(value: Binding) -> impl BindingMonad<Binding=Binding> {
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
        NextFn:         Fn(InputMonad::Binding) -> OutputMonad+Send+Sync {
    type Binding = OutputMonad::Binding;

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, OutputMonad::Binding) {
        let (bindings, value)   = self.input.resolve(bindings);
        let next                = (self.next)(value);
        next.resolve(bindings)
    }
}

///
/// That flat_map function for a binding monad
///
pub fn flat_map_binding<InputMonad: BindingMonad, OutputMonad: BindingMonad, NextFn: Fn(InputMonad::Binding) -> OutputMonad+Send+Sync>(action: NextFn, monad: InputMonad) -> impl BindingMonad<Binding=OutputMonad::Binding> {
    FlatMapValue {
        input:  monad,
        next:   action,
        output: PhantomData
    }
}
