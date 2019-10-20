use super::number::*;
use super::atom::*;
use super::monad_type::*;

use crate::bind::*;
use crate::exec::*;
use crate::bitcode::*;

use std::sync::*;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::any::*;

lazy_static! {
    /// A cellref representing the general 'nil' value
    pub static ref NIL: CellRef = SafasCell::Nil.into();
}

///
/// The type of value stored in a frame reference
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReferenceType {
    /// Normal value
    Value,

    /// Value that should be treated as a monad
    Monad,

    /// Value that returns a monad if called
    ReturnsMonad
}

///
/// A SAFAS cell represents a single value: for example 'D' or '24'
/// 
/// The most complicated of these structures is the list, which just joins two cells
///
pub enum SafasCell {
    /// The 'nil' value
    Nil,

    /// An atom with a particular name
    Atom(u64),

    /// A numeric value
    Number(SafasNumber),

    /// Bitcode generated by the assembler
    BitCode(Vec<BitCode>),

    /// A string value
    String(String),

    /// A character value
    Char(char),

    /// A list with a CAR and a CDR
    List(CellRef, CellRef),

    /// A reference to a value on the frame
    FrameReference(usize, u32, ReferenceType),

    /// A monad with the specified type and type
    Monad(CellRef, MonadType),

    /// A monad that transforms the state of a frame (generally a lambda)
    FrameMonad(Box<dyn FrameMonad<Binding=RuntimeResult>>),

    /// An action will transform the binding state of the compiler and generate a binding, and will compile that binding to a set of interpreter actions
    /// The parameter can be used to pass values between the pre-binding and the binding stage if needed
    ActionMonad(SyntaxCompiler, CellRef),

    /// An arbitrary Rust type
    Any(Box<dyn Any+Send+Sync>)
}

pub type CellRef = Arc<SafasCell>;

impl SafasCell {
    ///
    /// Turns an iterator of cells into a list
    ///
    pub fn list_with_cells<Cells: IntoIterator<Item=CellRef>>(cells: Cells) -> CellRef 
    where Cells::IntoIter : DoubleEndedIterator {
        // The first cell requires special treatment
        let cells       = cells.into_iter().rev();

        // We build the list by adding to the end
        let mut cell    = NIL.clone();
        for current_cell in cells {
            cell = SafasCell::List(current_cell, cell).into();
        }

        // Final result
        cell
    }

    ///
    /// Turns an iterator of cells into a list
    ///
    pub fn list_with_cells_and_cdr<Cells: IntoIterator<Item=CellRef>>(cells: Cells, cdr: CellRef) -> CellRef 
    where Cells::IntoIter : DoubleEndedIterator {
        // The first cell requires special treatment
        let cells       = cells.into_iter().rev();

        // We build the list by adding to the end
        let mut cell    = cdr;
        for current_cell in cells {
            cell = SafasCell::List(current_cell, cell).into();
        }

        // Final result
        cell
    }

    ///
    /// Returns true if this cell is nil
    ///
    pub fn is_nil(&self) -> bool {
        match self {
            SafasCell::Nil  => true,
            _               => false
        }
    }

    ///
    /// Returns the reference type for this cell
    ///
    pub fn reference_type(&self) -> ReferenceType {
        match self {
            SafasCell::Monad(_, _)                                  => ReferenceType::Monad,
            SafasCell::FrameReference(_, _, ref_type)               => *ref_type,
            SafasCell::FrameMonad(frame_monad)                      => if frame_monad.returns_monad() { ReferenceType::ReturnsMonad } else { ReferenceType::Value },
            SafasCell::List(car, cdr)                               => {
                if let SafasCell::ActionMonad(syntax, _) = &**car {
                    syntax.binding_monad.reference_type(cdr.clone())
                } else {
                    match car.reference_type() {
                        ReferenceType::ReturnsMonad => ReferenceType::Monad,            // Calling something that returns a monad, so evaluates to a monad
                        ReferenceType::Monad        => ReferenceType::ReturnsMonad,     // See compile_statement: 'calling' a monad returns a monad
                        _                           => ReferenceType::Value
                    }
                }
            },
            _                                                       => ReferenceType::Value
        }
    }

    ///
    /// If this is an atom, returns the atom ID (or None if it is not)
    ///
    pub fn to_atom_id(&self) -> Option<u64> {
        match self {
            SafasCell::Atom(atom_id)    => Some(*atom_id),
            _                           => None
        }
    }

    ///
    /// If this is a string, the value of the string
    ///
    pub fn string_value(&self) -> Option<String> {
        match self {
            SafasCell::String(string)   => Some(string.clone()),
            _                           => None
        }
    }

    ///
    /// If this is a character, the value of the character
    ///
    pub fn char_value(&self) -> Option<char> {
        match self {
            SafasCell::Char(chr)    => Some(*chr),
            _                       => None
        }
    }

    ///
    /// If this is a number, the value of the number
    ///
    pub fn number_value(&self) -> Option<SafasNumber> {
        match self {
            SafasCell::Number(number)   => Some(*number),
            _                           => None
        }
    }

    ///
    /// If this is a list, returns the car and cdr cells
    ///
    pub fn list_value(&self) -> Option<(CellRef, CellRef)> {
        match self {
            SafasCell::List(car, cdr)   => Some((Arc::clone(car), Arc::clone(cdr))),
            _                           => None
        }
    }

    ///
    /// If this is a frame reference, returns the cell ID and the frame number
    ///
    pub fn frame_reference(&self) -> Option<(usize, u32, ReferenceType)> {
        match self {
            SafasCell::FrameReference(cell, frame, ref_type)    => Some((*cell, *frame, *ref_type)),
            _                                                   => None
        }
    }

    ///
    /// Converts this cell to a string
    ///
    pub fn to_string(&self) -> String {
        use self::SafasCell::*;

        match self {
            Nil                                                         => "()".to_string(),
            Atom(atom_id)                                               => name_for_atom_with_id(*atom_id),
            Number(number)                                              => number.to_string(),
            BitCode(bitcode)                                            => format!("{}", bitcode.iter().map(|bitcode| bitcode.to_string()).collect::<Vec<_>>().join("")),
            String(string_value)                                        => format!("\"{}\"", string_value),         // TODO: character quoting
            Char(chr_value)                                             => format!("'{}'", chr_value),              // TODO: character quoting,
            FrameReference(cell, frame, ReferenceType::Value)           => format!("cell#({},{})", cell, frame),
            FrameReference(cell, frame, ReferenceType::Monad)           => format!("monadcell#({},{})", cell, frame),
            FrameReference(cell, frame, ReferenceType::ReturnsMonad)    => format!("monadfncell#({},{})", cell, frame),
            Monad(cell, monad)                                          => format!("monad#{}#{}", cell.to_string(), monad.to_string()),
            FrameMonad(monad)                                           => monad.description(),
            ActionMonad(syntax, parameter)                              => format!("compile#{}#{}", syntax.binding_monad.description(), parameter.to_string()),
            Any(val)                                                    => format!("any#{:p}", val),
            List(first, second)                                         => {
                let mut result  = format!("({}", first.to_string());
                let mut next    = second;

                loop {
                    match &**next {
                        Nil                 => { break; },
                        List(first, second) => {
                            result.push_str(&format!(" {}", first.to_string()));
                            next = second;
                        },
                        other               => {
                            result.push_str(&format!(" . {}", other.to_string()));
                            break;
                        }
                    }
                }

                result.push(')');

                result
            }
        }
    }

    ///
    /// If this cell represents a list, returns a vec of the items in the list
    /// 
    /// Returns None if the cell is not a list
    ///
    pub fn to_vec(&self) -> Option<Vec<CellRef>> {
        if let SafasCell::List(car, cdr) = self {
            let mut result = vec![];
            result.push(Arc::clone(car));

            let mut pos = cdr;
            while let SafasCell::List(car, cdr) = &**pos {
                result.push(Arc::clone(car));
                pos = cdr;
            }

            Some(result)
        } else if self.is_nil() {
            // Nil is the same as the empty list
            Some(vec![])
        } else {
            // Not a list
            None
        }
    }
}

impl Default for SafasCell {
    fn default() -> SafasCell {
        SafasCell::Nil
    }
}

impl Debug for SafasCell {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.to_string())
    }
}
