use crate::bind::*;
use crate::exec::*;

use smallvec::*;

///
/// The (def_syntax) keyword, expressed as a binding monad
/// 
/// Syntax is defined using:
/// 
/// ```(def_syntax <name> (<pattern> <macro> ...))```
/// 
/// <name> becomes a syntax item in the binding. We can use the new syntax like this:
/// 
/// ```(<name> <statements>)```
///
pub struct DefSyntaxKeyword {
}

impl DefSyntaxKeyword {
    ///
    /// Creates the def_syntax keyword
    ///
    pub fn new() -> DefSyntaxKeyword {
        DefSyntaxKeyword { }
    }
}

impl BindingMonad for DefSyntaxKeyword {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##def##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        unimplemented!()
    }
}