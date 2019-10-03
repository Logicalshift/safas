use super::cell::*;
use super::number::*;

use crate::exec::*;
use crate::bind::*;
use crate::meta::*;

use std::result::{Result};
use std::convert::{TryFrom};

///
/// Provides cell conversions for external types
/// 
pub struct CellValue<T>(pub T);

impl TryFrom<CellRef> for SafasNumber {
    type Error=RuntimeError;

    fn try_from(cell: CellRef) -> Result<SafasNumber, RuntimeError> {
        match &*cell {
            SafasCell::Number(num)  => Ok(num.clone()),
            _                       => Err(RuntimeError::TypeMismatch(cell))
        }
    }
}

impl TryFrom<CellRef> for CellValue<u8> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(u8::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<u16> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(u16::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<u32> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(u32::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<u64> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(u64::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<u128> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(u128::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<i8> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(i8::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<i16> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(i16::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<i32> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(i32::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<i64> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(i64::try_from(SafasNumber::try_from(cell)?)?)) }
}

impl TryFrom<CellRef> for CellValue<i128> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, RuntimeError> { Ok(CellValue(i128::try_from(SafasNumber::try_from(cell)?)?)) }
}

///
/// Represents an atom ID, used for conversions
///
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct AtomId(pub u64);

impl TryFrom<CellRef> for AtomId {
    type Error=RuntimeError;

    fn try_from(cell: CellRef) -> Result<AtomId, RuntimeError> {
        match &*cell {
            SafasCell::Atom(val)    => Ok(AtomId(*val)),
            _                       => Err(RuntimeError::BindingError(BindError::SyntaxExpectingAtom))
        }
    }
}

impl Into<CellRef> for AtomId {
    fn into(self) -> CellRef {
        SafasCell::Atom(self.0).into()
    }
}

impl<'a> From<&'a str> for AtomId {
    fn from(name: &'a str) -> AtomId {
        AtomId(get_id_for_atom_with_name(name))
    }
}

impl From<u64> for AtomId {
    fn from(id: u64) -> AtomId {
        AtomId(id)
    }
}

///
/// Represents a frame reference, used for conversions
///
pub struct FrameReference(pub usize, pub u32, pub ReferenceType);

impl TryFrom<CellRef> for FrameReference {
    type Error=RuntimeError;

    fn try_from(cell: CellRef) -> Result<FrameReference, RuntimeError> {
        match &*cell {
            SafasCell::FrameReference(cell, frame, cell_type)   => Ok(FrameReference(*cell, *frame, *cell_type)),
            _                                                   => Err(RuntimeError::BindingError(BindError::SyntaxExpectingAtom))
        }
    }
}

///
/// Represents a tuple generated from a list, used for conversions
///
pub struct ListTuple<T>(pub T);

impl<A1> TryFrom<CellRef> for ListTuple<(A1, )>
where   A1:             TryFrom<CellRef>,
        RuntimeError:   From<A1::Error> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, Self::Error> {
        // Read the list values
        let (first, cdr) = cell.list_value().ok_or(RuntimeError::BindingError(BindError::SyntaxExpectingList))?;

        if !cdr.is_nil() { return Err(RuntimeError::BindingError(BindError::TooManyArguments)); }

        // Convert them and generate the tuple
        Ok(ListTuple((A1::try_from(first)?, )))
    }
}

impl<A1, A2> TryFrom<CellRef> for ListTuple<(A1, A2)>
where   A1:         TryFrom<CellRef>,
        A2:         TryFrom<CellRef>,
        RuntimeError:   From<A1::Error>,
        RuntimeError:   From<A2::Error> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, Self::Error> {
        // Read the list values
        let (first, cdr)    = cell.list_value().ok_or(RuntimeError::BindingError(BindError::SyntaxExpectingList))?;
        let (second, cdr)   = cdr.list_value().ok_or(RuntimeError::BindingError(BindError::MissingArgument))?;

        if !cdr.is_nil() { return Err(RuntimeError::BindingError(BindError::TooManyArguments)); }

        // Convert them and generate the tuple
        Ok(ListTuple((A1::try_from(first)?, A2::try_from(second)?)))
    }
}

///
/// Like ListTuple, except will try to parse the final CDR as the tail type (so the list can be as long as needed)
/// rather than producing a fixed-size list
///
pub struct ListWithTail<THead, TTail>(pub THead, pub TTail);

impl<A1, TTail> TryFrom<CellRef> for ListWithTail<(A1, ), TTail>
where   A1:             TryFrom<CellRef>,
        TTail:          TryFrom<CellRef>,
        RuntimeError:   From<A1::Error>,
        RuntimeError:   From<TTail::Error> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, Self::Error> {
        // Read the list values
        let (first, cdr) = cell.list_value().ok_or(RuntimeError::BindingError(BindError::SyntaxExpectingList))?;

        // Convert them and generate the tuple
        Ok(ListWithTail((A1::try_from(first)?, ), TTail::try_from(cdr)?))
    }
}

impl<A1, A2, TTail> TryFrom<CellRef> for ListWithTail<(A1, A2), TTail>
where   A1:         TryFrom<CellRef>,
        A2:         TryFrom<CellRef>,
        TTail:          TryFrom<CellRef>,
        RuntimeError:   From<A1::Error>,
        RuntimeError:   From<A2::Error>,
        RuntimeError:   From<TTail::Error> {
    type Error=RuntimeError;
    fn try_from(cell: CellRef) -> Result<Self, Self::Error> {
        // Read the list values
        let (first, cdr)    = cell.list_value().ok_or(RuntimeError::BindingError(BindError::SyntaxExpectingList))?;
        let (second, cdr)   = cdr.list_value().ok_or(RuntimeError::BindingError(BindError::MissingArgument))?;

        // Convert them and generate the tuple
        Ok(ListWithTail((A1::try_from(first)?, A2::try_from(second)?), TTail::try_from(cdr)?))
    }
}
