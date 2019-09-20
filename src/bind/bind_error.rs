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
pub type BindResult<T> = Result<(T, SymbolBindings), (BindError, SymbolBindings)>;
