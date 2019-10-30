use super::binding_monad::*;
use super::bind_error::*;

use std::sync::*;

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
        let binding1    = Arc::new(self);
        let binding2    = Arc::clone(&binding1);

        let result = BindingFn::from_functions(move |bindings| {
            let (bindings, val) = binding1.bind(bindings);
            let map_val         = val.and_then(|val| action(val));
            (bindings, map_val)
        },
        move |bindings| {
            let (bindings, _val)    = binding2.pre_bind(bindings);
            let map_val             = Out::default();
            (bindings, map_val)
        });
        Box::new(result)
    }
}
