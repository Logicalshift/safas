use super::cell::*;
use crate::exec::*;

use std::sync::*;
use std::convert::{TryFrom};
use std::result::{Result};

///
/// Data structure representing a list item (with the Car and Cdr)
///
pub struct SafasList(pub Arc<SafasCell>, pub Arc<SafasCell>);

impl SafasList {
    ///
    /// A list containing just the 'nil' value
    ///
    pub fn nil() -> SafasList {
        SafasList(Arc::new(SafasCell::Nil), Arc::new(SafasCell::Nil))
    }
}

impl TryFrom<Arc<SafasCell>> for SafasList {
    type Error = RuntimeError;

    fn try_from(cell: Arc<SafasCell>) -> Result<SafasList, RuntimeError> {
        match &*cell {
            SafasCell::List(car, cdr)   => Ok(SafasList(Arc::clone(car), Arc::clone(cdr))),
            _                           => Err(RuntimeError::TypeMismatch(cell))
        }
    }
}

impl TryFrom<&Arc<SafasCell>> for SafasList {
    type Error = RuntimeError;

    fn try_from(cell: &Arc<SafasCell>) -> Result<SafasList, RuntimeError> {
        match &**cell {
            SafasCell::List(car, cdr)   => Ok(SafasList(Arc::clone(car), Arc::clone(cdr))),
            _                           => Err(RuntimeError::TypeMismatch(Arc::clone(cell)))
        }
    }
}
