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
    fn map<Out: 'static, NextFn: 'static+Fn(Self::Binding) -> Out+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=Out>>;
}

impl<T: 'static+BindingMonad> BindingMonadAndThen for T {
    fn and_then<OutputMonad: 'static+BindingMonad, NextFn: 'static+Fn(Self::Binding) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>> {
        let result = flat_map_binding(action, self);
        Box::new(result)
    }

    fn map<Out: 'static, NextFn: 'static+Fn(Self::Binding) -> Out+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=Out>> {
        let result = BindingFn(move |bindings| {
            let (bindings, val) = self.resolve(bindings);
            let map_val         = action(val);
            (bindings, map_val)
        });
        Box::new(result)
    }
}

///
/// Adds the `and_then_ok()` operation to a binding monad
///
pub trait BindingMonadAndThenOk<Val> : BindingMonad<Binding=Result<Val, BindError>> {
    ///
    /// `and_then_ok()` is syntactic sugar around the `flat_map_binding_error` operation, which adds continuations only on success,
    /// which makes for more easily read code
    ///
    fn and_then_ok<Out: 'static+Clone+Send+Sync, OutputMonad: 'static+BindingMonad<Binding=Result<Out, BindError>>, NextFn: 'static+Fn(Val) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>>;
}

impl<T: 'static+BindingMonad<Binding=Result<Val, BindError>>, Val: 'static> BindingMonadAndThenOk<Val> for T {
    fn and_then_ok<Out: 'static+Clone+Send+Sync, OutputMonad: 'static+BindingMonad<Binding=Result<Out, BindError>>, NextFn: 'static+Fn(Val) -> OutputMonad+Send+Sync>(self, action: NextFn) -> Box<dyn BindingMonad<Binding=OutputMonad::Binding>> {
        let result = flat_map_binding_error(action, self);
        Box::new(result)
    }
}
