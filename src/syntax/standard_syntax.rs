use super::def::*;
use super::def_syntax::*;
use super::fun::*;
use super::quote::*;

use crate::meta::*;
use crate::exec::*;
use crate::bind::*;

use smallvec::*;
use std::result::{Result};

///
/// Creates the standard syntax bindings for the SAFAS language
///
pub fn standard_syntax() -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Define the standard syntax
    let syntax  = wrap_binding(Ok(smallvec![]));
    let syntax  = flat_map_binding_actions(move || define_symbol_value("def",           SafasCell::ActionMonad(def_keyword())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("def_syntax",    SafasCell::ActionMonad(def_syntax_keyword())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("fun",           SafasCell::ActionMonad(fun_keyword())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("quote",         SafasCell::ActionMonad(quote_keyword())), syntax);

    syntax
}
