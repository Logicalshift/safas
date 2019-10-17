use super::def::*;
use super::def_syntax::*;
use super::fun::*;
use super::quote::*;

use crate::meta::*;
use crate::exec::*;
use crate::bind::*;
use crate::bitcode::*;

use smallvec::*;

///
/// Creates the standard syntax bindings for the SAFAS language
///
pub fn standard_syntax() -> impl BindingMonad<Binding=SmallVec<[Action; 8]>> {
    // Define the standard syntax
    let syntax  = wrap_binding(smallvec![]);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("def",           SafasCell::ActionMonad(def_keyword(), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("def_syntax",    SafasCell::ActionMonad(def_syntax_keyword(), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("fun",           SafasCell::ActionMonad(fun_keyword(), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("quote",         SafasCell::ActionMonad(quote_keyword(), NIL.clone())), syntax);

    // Define the bitcode syntax
    let syntax  = flat_map_binding_actions(move || define_symbol_value("label",         SafasCell::ActionMonad(label_keyword(), NIL.clone())), syntax);

    syntax
}
