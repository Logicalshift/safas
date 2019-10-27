use crate::meta::*;
use crate::bind::*;
use crate::parse::*;

use std::num::{TryFromIntError};
use std::result::{Result};
use std::convert::{Infallible};

///
/// Error that can occur during evaluating a frame
///
#[derive(Clone, Debug)]
pub enum RuntimeError {
    /// An operation marked as infallible managed to fail
    NotInfallible,

    /// A function is not yet implemented
    NotImplemented,

    /// An error occurred while parsing code
    ParseError(ParseError),

    /// A bind error occurred while generating code
    BindingError(BindError),

    /// Expected to pop a value from the stack but couldn't
    StackIsEmpty,

    /// Expected a particular type of cell, but a different type was encountered
    TypeMismatch(CellRef),

    /// Value cannot be called as a function
    NotAFunction(CellRef),

    /// Value cannot be treated as a monad
    NotAMonad(CellRef),

    /// Value cannot be treated as a string
    NotAString(CellRef),

    /// FlatMap function returned the wrong monad type
    MismatchedMonad(CellRef),

    /// Not enough arguments were passed to a function
    TooManyArguments(CellRef),

    /// Too many arguments were passed to a function
    NotEnoughArguments(CellRef),

    /// A value that was expected to be a label wasn't
    NotALabel(CellRef),

    /// Labels should be allocated prior to assembly
    CannotAllocateLabelsDuringAssembly,

    /// The number is too large to fit into the correct format
    NumberTooLarge,

    /// A file could not be found
    FileNotFound(String),

    /// An IO error occurred (we would have known the io::Error at the time but it's not compatible with RuntimeError as it can't be compared or cloned)
    IOError
}

/// The result of a runtime operation (most common binding type of a frame monad)
pub type RuntimeResult = Result<CellRef, RuntimeError>;

impl From<ParseError> for RuntimeError {
    fn from(error: ParseError) -> RuntimeError { 
        RuntimeError::ParseError(error)
    }
}

impl From<BindError> for RuntimeError {
    fn from(error: BindError) -> RuntimeError { 
        RuntimeError::BindingError(error)
    }
}

impl From<Infallible> for RuntimeError {
    fn from(_error: Infallible) -> RuntimeError { 
        RuntimeError::NotInfallible
    }
}

impl From<TryFromIntError> for RuntimeError {
    fn from(_error: TryFromIntError) -> RuntimeError { 
        RuntimeError::NumberTooLarge
    }
}
