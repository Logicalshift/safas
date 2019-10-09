use super::binding_monad::*;
use super::bind_error::*;

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
}

impl<T: 'static+BindingMonad> BindingMonadAndThen for T {
    fn and_then<OutputMonad: 'static+BindingMonad, NextFn: 'static+Fn(Self::Binding) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>> {
        let result = flat_map_binding(action, self);
        Box::new(result)
    }

    fn map<Out: 'static+Default, NextFn: 'static+Fn(Self::Binding) -> Out+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=Out>> {
        let result = BindingFn(move |bindings| {
            let (bindings, val) = self.bind(bindings);
            let map_val         = val.map(|val| action(val));
            (bindings, map_val)
        } /*,
        move |bindings| {
            let (bindings, val) = self.pre_bind(bindings);
            let map_val         = Out::default();
            (bindings, map_val)
        } */);
        Box::new(result)
    }
}

///
/// Adds the `and_then_ok()` operation to a binding monad
///
pub trait BindingMonadAndThenOk<Val: Default> : BindingMonad<Binding=Val> {
    ///
    /// `and_then_ok()` is syntactic sugar around the `flat_map_binding_error` operation, which adds continuations only on success,
    /// which makes for more easily read code
    ///
    fn and_then_ok<Out: 'static+Default+Clone+Send+Sync, OutputMonad: 'static+BindingMonad<Binding=Out>, NextFn: 'static+Fn(Val) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>>;
}

impl<T: 'static+BindingMonad<Binding=Val>, Val: 'static+Default> BindingMonadAndThenOk<Val> for T {
    fn and_then_ok<Out: 'static+Default+Clone+Send+Sync, OutputMonad: 'static+BindingMonad<Binding=Out>, NextFn: 'static+Fn(Val) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>> {
        let result = flat_map_binding(action, self);
        Box::new(result)
    }
}
