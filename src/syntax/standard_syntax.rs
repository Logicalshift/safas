use super::def::*;

use crate::exec::*;
use crate::bind::*;

use smallvec::*;
use std::sync::*;
use std::result::{Result};

///
/// Creates the standard syntax bindings for the SAFAS language
///
pub fn standard_syntax() -> impl BindingMonad<Binding=Result<Arc<SmallVec<[Action; 8]>>, BindError>> {
    // Create the bindings
    let def     = define_symbol_value("def", SymbolValue::ActionMonad(Arc::new(DefMonad::new())));

    // Combine them into a single monad
    //let result  = ();
    //let result  = flat_map_binding(move |_| def, result);

    def
}
