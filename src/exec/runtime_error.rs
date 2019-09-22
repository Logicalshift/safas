use crate::meta::*;
use crate::bind::*;
use crate::parse::*;

use std::num::{TryFromIntError};
use std::sync::*;
use std::result::{Result};
use std::convert::{Infallible};

///
/// Error that can occur during evaluating a frame
///
#[derive(Clone, Debug)]
pub enum RuntimeError {
    /// An operation marked as infallible managed to fail
    NotInfallible,

    /// An error occurred while parsing code
    ParseError(ParseError),

    /// A bind error occurred while generating code
    BindingError(BindError),

    /// Expected to pop a value from the stack but couldn't
    StackIsEmpty,

    /// Expected a particular type of cell, but a different type was encountered
    TypeMismatch(Arc<SafasCell>),

    /// Value cannot be called as a function
    NotAFunction(Arc<SafasCell>),

    /// Not enough arguments were passed to a function
    TooManyArguments(Arc<SafasCell>),

    /// Too many arguments were passed to a function
    NotEnoughArguments(Arc<SafasCell>),

    /// The number is too large to fit into the correct format
    NumberTooLarge
}

/// The result of a runtime operation (most common binding type of a frame monad)
pub type RuntimeResult = Result<Arc<SafasCell>, RuntimeError>;

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
