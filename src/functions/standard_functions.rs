use super::list::*;
use super::bits::*;
use super::monad::*;

use crate::meta::*;
use crate::exec::*;
use crate::bind::*;
use crate::syntax::*;
use crate::bitcode::*;

use smallvec::*;

///
/// Defines a function to be a frame monad
///
pub fn define_function<Monad>(atom: &str, monad: Monad) -> impl BindingMonad<Binding=SmallVec<[Action; 8]>>
where Monad: 'static+FrameMonad<Binding=RuntimeResult> {
    let monad = Box::new(monad);
    let monad = SafasCell::FrameMonad(monad);

    define_symbol_value(AtomId::from(atom), monad)
}

///
/// Creates the standard function bindings for the SAFAS language
///
pub fn standard_functions() -> impl BindingMonad<Binding=SmallVec<[Action; 8]>> {
    // Define the standard functions
    let functions  = wrap_binding(smallvec![]);

    // Bitcode functions
    let functions   = flat_map_binding_actions(move || define_function("d",             d_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("m",             m_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("a",             a_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("set_bit_pos",   set_bit_pos_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("bit_pos",       bit_pos_fn()), functions);

    let functions: Box<dyn BindingMonad<Binding=_>> = Box::new(functions);

    // List functions
    let functions   = flat_map_binding_actions(move || define_function("list",          list_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("cons",          cons_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("car",           car_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("cdr",           cdr_fn()), functions);

    let functions: Box<dyn BindingMonad<Binding=_>> = Box::new(functions);

    // Bit manipulation functions
    let functions   = flat_map_binding_actions(move || define_function("bits",          bits_fn()), functions);
    let functions   = flat_map_binding_actions(move || define_function("sbits",         sbits_fn()), functions);

    // Monad functions
    let functions   = flat_map_binding_actions(move || define_function("wrap",          wrap_fn()), functions);

    functions
}
