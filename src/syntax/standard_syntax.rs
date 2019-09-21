use super::def::*;
use super::lambda::*;

use crate::exec::*;
use crate::bind::*;

use smallvec::*;
use std::sync::*;
use std::result::{Result};

///
/// Creates the standard syntax bindings for the SAFAS language
///
pub fn standard_syntax() -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Define the standard syntax
    let syntax  = wrap_binding(Ok(smallvec![]));
    let syntax  = flat_map_binding_actions(move || define_symbol_value("def", SymbolValue::ActionMonad(Arc::new(DefKeyword::new()))), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("lambda", SymbolValue::ActionMonad(Arc::new(LambdaKeyword::new()))), syntax);

    syntax
}
