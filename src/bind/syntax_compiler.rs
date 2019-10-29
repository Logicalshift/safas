use super::bind_error::*;

use crate::exec::*;

use std::sync::*;

///
/// A syntax compiler describes the actions needed to compile a piece of syntax into a series of actions
/// 
/// There are two components. The binding monad binds all of the cells to their bound values. The action
/// takes the result of the binding and applies it to generate the actions required to execute the syntax
///
pub struct SyntaxCompiler {
    /// Generates the actions for the bound syntax
    pub generate_actions: Arc<dyn Fn() -> Result<CompiledActions, BindError>+Send+Sync>
}

impl Default for SyntaxCompiler {
    fn default() -> SyntaxCompiler {
        // Default syntax compiler compiles nothing
        SyntaxCompiler {
            generate_actions: Arc::new(|| Ok(CompiledActions::empty()))
        }
    }
}
