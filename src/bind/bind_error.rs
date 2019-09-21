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
    ConstantsCannotBeCalled,

    /// Macro was called without arguments (if arguments were present but one was missing, use the call below)
    ArgumentsWereNotSupplied,

    /// An expected argument was missing
    MissingArgument,

    /// Too many arguments were supplied to a macro
    TooManyArguments,

    /// Arguments were not supplied to a function declaration
    LambdaArgumentsNotSupplied,

    /// Tried to define a value to a symbol that was not an atom
    VariablesMustBeAtoms
}

/// Result of a binding operation
pub type BindResult<T> = Result<(T, SymbolBindings), (BindError, SymbolBindings)>;
