use super::bind_error::*;
use super::symbol_bindings::*;

use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::marker::{PhantomData};

///
/// The binding monad describes how to bind a program against its symbols
///
pub trait BindingMonad : Send+Sync {
    type Binding;

    ///
    /// Rebinds this monad to bind at a particular frame depth
    /// 
    /// This is used when this binding is first used from an 'outer' frame and might need to import its symbols
    /// to generate a closure (for example, a macro that depends on a value from an outer frame will need to
    /// import that symbol to access it)
    /// 
    /// Can return None if this monad is not changed by the rebinding.
    ///
    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, _frame_depth: u32) -> (SymbolBindings, Option<Box<dyn BindingMonad<Binding=Self::Binding>>>) { (bindings, None) }

    ///
    /// Binds the content of this monad to some symbol bindings (returning the new symbol bindings and the bound value)
    ///
    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding);

    ///
    /// Called with the results of binding using this monad, returns the reference type that this will generate
    /// 
    fn reference_type(&self, _bound_value: CellRef) -> ReferenceType { ReferenceType::Value }

    ///
    /// Returns a string that describes what this monad does
    ///
    fn description(&self) -> String { "##syntax##".to_string() }
}

impl BindingMonad for () {
    type Binding = ();

    fn description(&self) -> String { "##nop##".to_string() }
    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, ()) { (bindings, ()) }
}

///
/// Binding monad generated from a resolve function
///
pub struct BindingFn<TFn: Fn(SymbolBindings) -> (SymbolBindings, TBinding), TBinding>(pub TFn);

impl<TFn, TBinding> BindingMonad for BindingFn<TFn, TBinding> 
where TFn: Fn(SymbolBindings) -> (SymbolBindings, TBinding)+Send+Sync {
    type Binding = TBinding;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, TBinding) {
        let BindingFn(ref fun) = self;
        fun(bindings)
    }
}

///
/// Binding monad that returns a constant value
///
struct ReturnValue<Binding: Clone> {
    value: Binding
}

impl<Binding: Send+Sync+Clone> BindingMonad for ReturnValue<Binding> {
    type Binding=Binding;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Binding) {
        (bindings, self.value.clone())
    }
}

impl<Binding> BindingMonad for Box<dyn BindingMonad<Binding=Binding>> {
    type Binding=Binding;

    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<Box<dyn BindingMonad<Binding=Self::Binding>>>) {
        (**self).rebind_from_outer_frame(bindings, frame_depth)
    }

    fn description(&self) -> String { (**self).description() }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Binding) {
        (**self).bind(bindings)
    }

    fn reference_type(&self, bound_value: CellRef) -> ReferenceType { (**self).reference_type(bound_value) }
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

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, OutputMonad::Binding) {
        let (bindings, value)   = self.input.bind(bindings);
        let next                = (self.next)(value);
        next.bind(bindings)
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
/// A variant of flat_map that strips out errors from the result of the input monad
///
pub fn flat_map_binding_error<InputMonad, OutputMonad, Val, Out, NextFn: Fn(Val) -> OutputMonad+Send+Sync>(action: NextFn, monad: InputMonad) -> impl BindingMonad<Binding=Result<Out, BindError>> 
where  InputMonad:  BindingMonad<Binding=Result<Val, BindError>>,
       OutputMonad: 'static+BindingMonad<Binding=Result<Out, BindError>>,
       Out:         'static+Clone+Send+Sync {
    flat_map_binding(move |maybe_error| {
        let result: Box<dyn BindingMonad<Binding=Result<Out, BindError>>> = match maybe_error {
            Ok(val)     => Box::new(action(val)),
            Err(err)    => Box::new(wrap_binding(Err(err)))
        };

        result
    }, monad)
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
