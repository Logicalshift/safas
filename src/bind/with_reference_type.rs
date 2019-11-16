use super::bind_error::*;
use super::binding_monad::*;
use super::symbol_bindings::*;

use crate::meta::*;

///
/// Same as the binding monad, but returning the specified reference type
///
pub struct WithReferenceType<TBindingMonad>(pub TBindingMonad, pub ReferenceType);

impl<TBindingMonad> BindingMonad for WithReferenceType<TBindingMonad>
where   TBindingMonad: BindingMonad,
        TBindingMonad::Binding: 'static {
    type Binding = TBindingMonad::Binding;

    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, parameter: CellRef, frame_depth: u32) -> (SymbolBindings, Option<(Box<dyn BindingMonad<Binding=Self::Binding>>, CellRef)>) {
        let (binding, rebound)  = self.0.rebind_from_outer_frame(bindings, parameter, frame_depth);
        let rebound             = rebound.map(|(binding, parameter)| -> (Box<dyn BindingMonad<Binding=Self::Binding>>, _) { (Box::new(WithReferenceType(binding, self.1)), parameter) });
        (binding, rebound)
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        self.0.pre_bind(bindings)
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        self.0.bind(bindings)
    }

    ///
    /// Called with the results of binding using this monad, returns the reference type that this will generate
    /// 
    fn reference_type(&self, _bound_value: CellRef) -> ReferenceType {
        self.1
    }

    ///
    /// Returns a string that describes what this monad does
    ///
    fn description(&self) -> String {
        self.0.description()
    }
}
