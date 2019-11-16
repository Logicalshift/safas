use super::def::*;
use super::def_syntax::*;
use super::extend_syntax::*;
use super::fun::*;
use super::quote::*;
use super::export::*;
use super::conditional::*;

use crate::io::*;
use crate::meta::*;
use crate::exec::*;
use crate::bind::*;
use crate::bitcode::*;

use smallvec::*;

///
/// Creates the standard syntax bindings for the SAFAS language
///
pub fn standard_syntax() -> impl BindingMonad<Binding=SmallVec<[Action; 8]>> {
    // Module syntax
    let syntax  = wrap_binding(smallvec![]);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("built_ins",     builtin_library()), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("import",        SafasCell::Syntax(Box::new(import_keyword()), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("export",        SafasCell::Syntax(Box::new(export_keyword()), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("re_export",     SafasCell::Syntax(Box::new(re_export_keyword()), NIL.clone())), syntax);

    let syntax: Box<dyn BindingMonad<Binding=_>> = Box::new(syntax);

    // Definition/declaration syntax
    let syntax  = flat_map_binding_actions(move || define_symbol_value("def",           SafasCell::Syntax(Box::new(def_keyword()), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("def_syntax",    SafasCell::Syntax(Box::new(def_syntax_keyword()), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("extend_syntax", SafasCell::Syntax(Box::new(extend_syntax_keyword()), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("fun",           SafasCell::Syntax(Box::new(fun_keyword()), NIL.clone())), syntax);
    let syntax  = flat_map_binding_actions(move || define_symbol_value("quote",         SafasCell::Syntax(Box::new(quote_keyword()), NIL.clone())), syntax);

    let syntax: Box<dyn BindingMonad<Binding=_>> = Box::new(syntax);

    // Conditional syntax
    let syntax  = flat_map_binding_actions(move || define_symbol_value("if",            SafasCell::Syntax(Box::new(if_keyword()), NIL.clone())), syntax);

    // Bitcode syntax
    let syntax  = flat_map_binding_actions(move || define_symbol_value("label",         SafasCell::Syntax(Box::new(label_keyword()), NIL.clone())), syntax);

    syntax
}
