use super::bitcode::*;
use super::list::*;
use super::bits::*;

use crate::meta::*;
use crate::exec::*;
use crate::bind::*;
use crate::syntax::*;

use smallvec::*;
use std::result::{Result};

///
/// Defines a function to be a frame monad
///
pub fn define_function<Monad>(atom: &str, monad: Monad) -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>>
where Monad: 'static+FrameMonad<Binding=RuntimeResult> {
    let monad = Box::new(monad);
    let monad = SafasCell::FrameMonad(monad);

    define_symbol_value(AtomId::from(atom), monad)
}

///
/// Creates the standard function bindings for the SAFAS language
///
pub fn standard_functions() -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Define the standard functions
    let functions  = wrap_binding(Ok(smallvec![]));

    // Bitcode functions
    let functions   = flat_map_binding_actions(move || define_function("d", d_keyword()), functions);
    let functions   = flat_map_binding_actions(move || define_function("m", m_keyword()), functions);
    let functions   = flat_map_binding_actions(move || define_function("a", a_keyword()), functions);

    // List functions
    let functions   = flat_map_binding_actions(move || define_function("list",   list_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("cons",   cons_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("car",    car_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("cdr",    cdr_fn()), functions);

    let functions: Box<dyn BindingMonad<Binding=_>> = Box::new(functions);

    // Bit manipulation functions
    let functions   = flat_map_binding_actions(move || define_function("bits",   bits_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("sbits",  sbits_fn()), functions);

    functions
}
