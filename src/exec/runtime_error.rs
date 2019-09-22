use crate::meta::*;
use crate::bind::*;
use crate::parse::*;

use std::sync::*;
use std::result::{Result};

///
/// Error that can occur during evaluating a frame
///
#[derive(Clone, Debug)]
pub enum RuntimeError {
    /// An error occurred while parsing code
    ParseError(ParseError),

    /// A bind error occurred while generating code
    BindingError(BindError),

    /// Expected to pop a value from the stack but couldn't
    StackIsEmpty,

    /// Expected a particular type of cell, but a different type was encountered
    TypeMismatch(Arc<SafasCell>),

    /// Value cannot be called as a function
    NotAFunction(Arc<SafasCell>)
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
