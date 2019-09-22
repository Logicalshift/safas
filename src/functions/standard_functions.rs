use super::bitcode::*;

use crate::meta::*;
use crate::exec::*;
use crate::bind::*;
use crate::syntax::*;

use smallvec::*;
use std::sync::*;
use std::result::{Result};

///
/// Defines a function to be a frame monad
///
pub fn define_function<Monad>(atom: &str, monad: Monad) -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>>
where Monad: 'static+FrameMonad<Binding=RuntimeResult> {
    let monad = Arc::new(monad);
    let monad = SafasCell::Monad(monad);

    define_symbol_value(atom, SymbolValue::Constant(Arc::new(monad)))
}

///
/// Creates the standard function bindings for the SAFAS language
///
pub fn standard_functions() -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Define the standard functions
    let functions  = wrap_binding(Ok(smallvec![]));
    let functions  = flat_map_binding_actions(move || define_function("d", d_keyword()), functions);
    let functions  = flat_map_binding_actions(move || define_function("m", m_keyword()), functions);
    let functions  = flat_map_binding_actions(move || define_function("a", a_keyword()), functions);

    functions
}
