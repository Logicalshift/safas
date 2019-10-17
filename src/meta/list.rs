use super::cell::*;
use crate::exec::*;

use std::sync::*;
use std::convert::{TryFrom};
use std::result::{Result};

///
/// Data structure representing a list item (with the Car and Cdr)
///
pub struct SafasList(pub CellRef, pub CellRef);

impl TryFrom<CellRef> for SafasList {
    type Error = RuntimeError;

    fn try_from(cell: CellRef) -> Result<SafasList, RuntimeError> {
        match &*cell {
            SafasCell::List(car, cdr)   => Ok(SafasList(Arc::clone(car), Arc::clone(cdr))),
            _                           => Err(RuntimeError::TypeMismatch(cell))
        }
    }
}

impl TryFrom<&CellRef> for SafasList {
    type Error = RuntimeError;

    fn try_from(cell: &CellRef) -> Result<SafasList, RuntimeError> {
        match &**cell {
            SafasCell::List(car, cdr)   => Ok(SafasList(Arc::clone(car), Arc::clone(cdr))),
            _                           => Err(RuntimeError::TypeMismatch(Arc::clone(cell)))
        }
    }
}
