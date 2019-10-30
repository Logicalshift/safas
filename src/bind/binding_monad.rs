use super::bind_error::*;
use super::symbol_bindings::*;

use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::marker::{PhantomData};
use std::sync::*;

///
/// The binding monad describes how to bind a program against its symbols
///
pub trait BindingMonad : Send+Sync {
    type Binding: Default;

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
    /// Performs pre-binding steps
    /// 
    /// When compiling a multi-statement expression, this will be called for all syntax elements to give them an opportunity to
    /// forward-declare any values they wish. No output value is generated during this stage: all that can be done is to
    /// update the bindings for a particular statement.
    /// 
    /// The return value here is passed on to the next monad in the chain.
    ///
    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding);

    ///
    /// Binds the content of this monad to some symbol bindings (returning the new symbol bindings and the bound value)
    /// 
    /// The bound value returned here is the value returned to the next monad in the chain, or the input to the
    /// compiler stage.
    ///
    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>);

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
    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<(), BindError>) { (bindings, Ok(())) }
    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, ()) { (bindings, ()) }
}

///
/// Binding monad generated from a resolve function
///
pub struct BindingFn<TFn: Fn(SymbolBindings) -> (SymbolBindings, Result<TBinding, BindError>), TPreBind: Fn(SymbolBindings) -> (SymbolBindings, TBinding), TBinding>(pub TFn, pub TPreBind);

impl<TFn, TBinding: Default> BindingFn<TFn, fn(SymbolBindings) -> (SymbolBindings, TBinding), TBinding>
where   TFn:        Fn(SymbolBindings) -> (SymbolBindings, Result<TBinding, BindError>)+Send+Sync {
    ///
    /// Creates a binding function with no pre-binding
    ///
    pub fn from_binding_fn(bind: TFn) -> BindingFn<TFn, fn(SymbolBindings) -> (SymbolBindings, TBinding), TBinding> {
        BindingFn(bind, prebind_no_op)
    }
}

impl<TFn, TPreBind, TBinding: Default> BindingFn<TFn, TPreBind, TBinding>
where   TFn:        Fn(SymbolBindings) -> (SymbolBindings, Result<TBinding, BindError>)+Send+Sync,
        TPreBind:   Fn(SymbolBindings) -> (SymbolBindings, TBinding)+Send+Sync {
    ///
    /// Creates a binding function with no pre-binding
    ///
    pub fn from_functions(bind: TFn, pre_bind: TPreBind) -> BindingFn<TFn, TPreBind, TBinding> {
        BindingFn(bind, pre_bind)
    }
}

///
/// A pre-binding function that performs no operation
///
fn prebind_no_op<TBinding: Default>(bindings: SymbolBindings) -> (SymbolBindings, TBinding) {
    (bindings, TBinding::default())
}

impl<TFn, TPreBind, TBinding: Default> BindingMonad for BindingFn<TFn, TPreBind, TBinding>
where   TFn:        Fn(SymbolBindings) -> (SymbolBindings, Result<TBinding, BindError>)+Send+Sync,
        TPreBind:   Fn(SymbolBindings) -> (SymbolBindings, TBinding)+Send+Sync {
    type Binding = TBinding;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<TBinding, BindError>) {
        let BindingFn(ref fun, ref _prebind) = self;
        fun(bindings)
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, TBinding) {
        let BindingFn(ref _fun, ref prebind) = self;
        prebind(bindings)
    }
}

///
/// Binding monad that returns a constant value
///
struct ReturnValue<Binding: Clone> {
    value: Binding
}

impl<Binding: Default+Send+Sync+Clone> BindingMonad for ReturnValue<Binding> {
    type Binding=Binding;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Binding, BindError>) {
        (bindings, Ok(self.value.clone()))
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Binding) {
        (bindings, self.value.clone())
    }
}

impl<Binding: Default> BindingMonad for Box<dyn BindingMonad<Binding=Binding>> {
    type Binding=Binding;

    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<Box<dyn BindingMonad<Binding=Self::Binding>>>) {
        (**self).rebind_from_outer_frame(bindings, frame_depth)
    }

    fn description(&self) -> String { (**self).description() }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Binding, BindError>) {
        (**self).bind(bindings)
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Binding) { 
        (**self).pre_bind(bindings)
    }

    fn reference_type(&self, bound_value: CellRef) -> ReferenceType { (**self).reference_type(bound_value) }
}

///
/// Wraps a value in a binding monad
///
pub fn wrap_binding<Binding: Default+Send+Sync+Clone>(value: Binding) -> impl BindingMonad<Binding=Binding> {
    ReturnValue { value }
}

struct FlatMapValue<InputMonad, OutputMonad, NextFn> {
    input:  InputMonad,
    next:   Arc<NextFn>,
    output: PhantomData<OutputMonad>
}

impl<InputMonad, OutputMonad, NextFn> BindingMonad for FlatMapValue<InputMonad, OutputMonad, NextFn>
where   InputMonad:     'static+BindingMonad,
        OutputMonad:    'static+BindingMonad,
        NextFn:         'static+Fn(InputMonad::Binding) -> OutputMonad+Send+Sync {
    type Binding = OutputMonad::Binding;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<OutputMonad::Binding, BindError>) {
        let (bindings, value)   = self.input.bind(bindings);
        let next                = value.map(|value| (self.next)(value));

        match next {
            Ok(next)    => next.bind(bindings),
            Err(err)    => (bindings, Err(err))
        }
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, OutputMonad::Binding) { 
        let (bindings, value)   = self.input.pre_bind(bindings);
        let next                = (self.next)(value);
        next.pre_bind(bindings)
    }
    
    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<Box<dyn BindingMonad<Binding=Self::Binding>>>) { 
        let (bindings, rebound_input)   = self.input.rebind_from_outer_frame(bindings, frame_depth);
        let rebound_input               = rebound_input.map(|rebound_input| FlatMapValue { input: rebound_input, next: self.next.clone(), output: PhantomData });
        let rebound_input               = rebound_input.map(|rebound_input| -> Box<dyn BindingMonad<Binding=Self::Binding>> { Box::new(rebound_input) });

        (bindings, rebound_input) 
    }

    fn reference_type(&self, bound_value: CellRef) -> ReferenceType { self.input.reference_type(bound_value) }
}

///
/// The flat_map function for a binding monad
///
pub fn flat_map_binding<InputMonad: 'static+BindingMonad, OutputMonad: 'static+BindingMonad, NextFn: 'static+Fn(InputMonad::Binding) -> OutputMonad+Send+Sync>(action: NextFn, monad: InputMonad) -> impl BindingMonad<Binding=OutputMonad::Binding> {
    FlatMapValue {
        input:  monad,
        next:   Arc::new(action),
        output: PhantomData
    }
}

///
/// As for flat_map but combines two monads that generate actions by concatenating the actions together
///
pub fn flat_map_binding_actions<InputMonad, OutputMonad, NextFn>(action: NextFn, monad: InputMonad) -> impl BindingMonad<Binding=SmallVec<[Action; 8]>>
where   InputMonad:     'static+BindingMonad<Binding=SmallVec<[Action; 8]>>,
        OutputMonad:    'static+BindingMonad<Binding=SmallVec<[Action; 8]>>,
        NextFn:         'static+Fn() -> OutputMonad+Send+Sync {
    // Perform the input monad
    flat_map_binding(move |actions| {
        // Resolve the output
        let next = action();

        flat_map_binding(move |next_actions| {
            // Combine the actions from both monads
            let mut actions = actions.clone();
            actions.extend(next_actions);

            wrap_binding(actions)
        }, next)
    }, monad)
}
