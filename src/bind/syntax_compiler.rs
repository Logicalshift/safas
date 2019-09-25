use super::bind_error::*;
use super::binding_monad::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;

///
/// A syntax compiler describes the actions needed to compile a piece of syntax into a series of actions
/// 
/// There are two components. The binding monad binds all of the cells to their bound values. The action
/// takes the result of the binding and applies it to generate the actions required to execute the syntax
///
pub struct SyntaxCompiler {
    /// Generates the bound statement for this syntax
    pub binding_monad: Box<dyn BindingMonad<Binding=Result<CellRef, BindError>>>,

    /// Generates the actions for the bound syntax
    pub generate_actions: Box<dyn Fn(CellRef) -> Result<SmallVec<[Action; 8]>, BindError>+Send+Sync>
}
