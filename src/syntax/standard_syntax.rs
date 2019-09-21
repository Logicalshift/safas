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
    // Define the standard syntax
    let syntax  = ();
    let syntax  = flat_map_binding(move |_| define_symbol_value("def", SymbolValue::ActionMonad(Arc::new(DefMonad::new()))), syntax);

    syntax
}
