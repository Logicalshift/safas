use super::binding_monad::*;
use super::bind_error::*;

use crate::meta::*;
use crate::bind::*;

use std::sync::*;

struct MapBinding<InputMonad, NextFn> {
    input:  InputMonad,
    next:   Arc<NextFn>
}

impl<InputMonad, OutputValue, NextFn> BindingMonad for MapBinding<InputMonad, NextFn>
where   InputMonad:     'static+BindingMonad,
        OutputValue:    'static+Default,
        NextFn:         'static+Fn(InputMonad::Binding) -> Result<OutputValue, BindError>+Send+Sync {
    type Binding = OutputValue;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<OutputValue, BindError>) {
        let (bindings, val) = self.input.bind(bindings);
        let map_val         = val.and_then(|val| (self.next)(val));
        (bindings, map_val)
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, OutputValue) { 
        let (bindings, _val)    = self.input.pre_bind(bindings);
        let map_val             = OutputValue::default();
        (bindings, map_val)
    }
    
    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, parameter: CellRef, frame_depth: u32) -> (SymbolBindings, Option<(Box<dyn BindingMonad<Binding=Self::Binding>>, CellRef)>) {
        let (bindings, rebound_input)   = self.input.rebind_from_outer_frame(bindings, parameter, frame_depth);
        let rebound_input               = rebound_input.map(|(rebound_input, parameter)| (MapBinding { input: rebound_input, next: self.next.clone() }, parameter));
        let rebound_input               = rebound_input.map(|(rebound_input, parameter)| -> (Box<dyn BindingMonad<Binding=Self::Binding>>, _) { (Box::new(rebound_input), parameter) });

        (bindings, rebound_input) 
    }

    fn reference_type(&self, bound_value: CellRef) -> ReferenceType { self.input.reference_type(bound_value) }
}

///
/// Adds the `and_then` operation to a binding monad
///
pub trait BindingMonadAndThen : BindingMonad {
    ///
    /// `and_then()` is syntactic sugar around the `flat_map_binding` operation, which allows for writing more easily understood
    /// code by chaining operations in the order that they occur
    ///
    fn and_then<OutputMonad: 'static+BindingMonad, NextFn: 'static+Fn(Self::Binding) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>>;

    ///
    /// Maps the value contained by this monad to another value
    ///
    fn map<Out: 'static+Default, NextFn: 'static+Fn(Self::Binding) -> Out+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=Out>>;

    ///
    /// As for 'map' but allows for returning an error from the mapping function
    ///
    fn map_result<Out: 'static+Default, NextFn: 'static+Fn(Self::Binding) -> Result<Out, BindError>+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=Out>>;
}

impl<T: 'static+BindingMonad> BindingMonadAndThen for T {
    fn and_then<OutputMonad: 'static+BindingMonad, NextFn: 'static+Fn(Self::Binding) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>> {
        let result = flat_map_binding(action, self);
        Box::new(result)
    }

    fn map<Out: 'static+Default, NextFn: 'static+Fn(Self::Binding) -> Out+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=Out>> {
        self.map_result(move |val| Ok(action(val)))
    }

    fn map_result<Out: 'static+Default, NextFn: 'static+Fn(Self::Binding) -> Result<Out, BindError>+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=Out>> {
        let result = MapBinding {
            input:  self,
            next:   Arc::new(action)
        };

        Box::new(result)
    }
}
