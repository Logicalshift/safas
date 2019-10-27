use super::symbol_bindings::*;

use crate::exec::*;

use std::convert::{Infallible};
use std::result::{Result};

///
/// Indicates an error that ocurred during binding
///
#[derive(Clone, PartialEq, Debug)]
pub enum BindError {
    /// Something that was meant to be infallible failed
    NotInfallible,

    /// A feature is not yet implemented
    NotImplemented,

    /// A symbol has no known value
    UnknownSymbol(String),

    /// A symbol has a value but is not bound to anything
    UnboundSymbol,

    /// Attempted to compile a FrameReference that points at a different frame (they should all be bound to the current frame)
    CannotLoadCellInOtherFrame,

    /// Macro monads can't be compiled directly: they should be expanded during the binding phase
    MacrosShouldBeBoundBeforeCompiling,

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
    SyntaxMatchFailed,

    /// We tried to evaluate some SAFAS code but it failed
    RuntimeError,

    /// A number couldn't be converted for this syntax
    NumberTooLarge,

    /// An attempt was made to use a symbol that was not yet defined
    ForwardReferencesNotAllowed,

    /// A file could not be found
    FileNotFound,

    /// An IO error occurred
    IOError
}

/// Result of a binding operation
pub type BindResult<T> = Result<(T, SymbolBindings), (BindError, SymbolBindings)>;

impl From<Infallible> for BindError {
    fn from(_: Infallible) -> BindError {
        BindError::NotInfallible
    }
}

impl From<RuntimeError> for BindError {
    fn from(err: RuntimeError) -> BindError {
        use self::RuntimeError::*;
        match err {
            NotInfallible                       => BindError::NotInfallible,
            NumberTooLarge                      => BindError::NumberTooLarge,
            NotImplemented                      => BindError::NotImplemented,
            FileNotFound                        => BindError::FileNotFound,
            IOError                             => BindError::IOError,
            BindingError(err)                   => err,
            ParseError(_)                       |
            StackIsEmpty                        |
            TypeMismatch(_)                     |
            NotAFunction(_)                     |
            TooManyArguments(_)                 |
            NotAMonad(_)                        |
            MismatchedMonad(_)                  |
            NotALabel(_)                        |
            CannotAllocateLabelsDuringAssembly  |
            NotEnoughArguments(_)               => BindError::RuntimeError
        }
    }
}
