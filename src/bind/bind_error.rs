use super::symbol_bindings::*;

use std::result::{Result};

///
/// Indicates an error that ocurred during binding
///
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BindError {
    /// A symbol has no known value
    UnknownSymbol,

    /// A symbol has a value but is not bound to anything
    UnboundSymbol,

    /// A constant was used where a function was expected
    ConstantsCannotBeCalled
}

/// Result of a binding operation
/// 
/// (I'd prefer '(Result<T, BindError>, SymbolBindings)' but rust makes that super annoying to work with. This is pretty bad too because of how
/// much error mapping needs to be done to get the bindings in)
pub type BindResult<T> = Result<(T, SymbolBindings), (BindError, SymbolBindings)>;
