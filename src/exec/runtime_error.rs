use crate::meta::*;

use std::sync::*;
use std::result::{Result};

///
/// Error that can occur during evaluating a frame
///
#[derive(Clone, Debug)]
pub enum RuntimeError {
    /// Expected to pop a value from the stack but couldn't
    StackIsEmpty,

    /// Value cannot be called as a function
    NotAFunction(Arc<SafasCell>)
}

/// The result of a runtime operation (most common binding type of a frame monad)
pub type RuntimeResult = Result<Arc<SafasCell>, RuntimeError>;
