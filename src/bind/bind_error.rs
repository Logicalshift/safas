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
    FunArgumentsNotSupplied,

    /// Tried to define a value to a symbol that was not an atom
    VariablesMustBeAtoms,

    /// Tried to bind to a syntax item that can't be matched against (eg a cell containing a function)
    NotValidInSyntax,

    /// Was expecting an atom in this position
    SyntaxExpectingAtom,

    /// Was expecting a list
    SyntaxExpectingList,

    /// A '>' or a '}' was missing when generating a syntax pattern
    SyntaxMissingBracket(char),

    /// All symbols in the syntax were matched, but there was still extra input
    SyntaxMatchedPrefix,

    /// A symbol that could not be matched was encountered in a syntax pattern
    SyntaxMatchFailed
}

/// Result of a binding operation
pub type BindResult<T> = Result<(T, SymbolBindings), (BindError, SymbolBindings)>;
