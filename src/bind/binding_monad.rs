use super::bind_error::*;
use super::symbol_bindings::*;
use crate::exec::*;

use smallvec::*;
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
/// The flat_map function for a binding monad
///
pub fn flat_map_binding<InputMonad: BindingMonad, OutputMonad: BindingMonad, NextFn: Fn(InputMonad::Binding) -> OutputMonad+Send+Sync>(action: NextFn, monad: InputMonad) -> impl BindingMonad<Binding=OutputMonad::Binding> {
    FlatMapValue {
        input:  monad,
        next:   action,
        output: PhantomData
    }
}

///
/// As for flat_map but combines two monads that generate actions by concatenating the actions together
///
pub fn flat_map_binding_actions<InputMonad, OutputMonad, NextFn>(action: NextFn, monad: InputMonad) -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>>
where   InputMonad:     BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>>,
        OutputMonad:    BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>>,
        NextFn:         Fn() -> OutputMonad+Send+Sync {
    // Perform the input monad
    flat_map_binding(move |actions| {
        // Resolve the output
        let next = action();

        flat_map_binding(move |next_actions| {
            // Combine the actions from both monads
            match (actions.clone(), next_actions) {
                (Ok(actions), Ok(next_actions)) => { 
                    let mut actions = actions;
                    actions.extend(next_actions);

                    wrap_binding(Ok(actions))
                },
                
                (Err(err), _) => wrap_binding(Err(err.clone())),
                (_, Err(err)) => wrap_binding(Err(err))
            }
        }, next)
    }, monad)
}
