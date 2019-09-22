use super::cell::*;
use super::number::*;

use crate::exec::*;

use std::sync::*;
use std::result::{Result};
use std::convert::{TryFrom};

///
/// Provides cell conversions for external types
/// 
pub struct CellValue<T>(pub T);

impl TryFrom<Arc<SafasCell>> for SafasNumber {
    type Error=RuntimeError;

    fn try_from(cell: Arc<SafasCell>) -> Result<SafasNumber, RuntimeError> {
        match &*cell {
            SafasCell::Number(num)  => Ok(num.clone()),
            _                       => Err(RuntimeError::TypeMismatch(cell))
        }
    }
}

impl TryFrom<Arc<SafasCell>> for CellValue<u8> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(u8::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<u16> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(u16::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<u32> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(u32::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<u64> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(u64::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<u128> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(u128::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<i8> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(i8::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<i16> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(i16::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<i32> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(i32::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<i64> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(i64::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<Arc<SafasCell>> for CellValue<i128> {
    type Error=RuntimeError;
    fn try_from(cell: Arc<SafasCell>) -> Result<Self, RuntimeError> { Ok(CellValue(i128::try_from(SafasNumber::try_from(cell)?)?)) }
}
