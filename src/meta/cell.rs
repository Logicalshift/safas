use super::number::*;
use super::atom::*;

use crate::bind::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::fmt;
use std::fmt::{Debug, Formatter};

///
/// A SAFAS cell represents a single value: for example 'D' or '24'
/// 
/// The most complicated of these structures is the list, which just joins two cells
///
#[derive(Clone)]
pub enum SafasCell {
    /// The 'nil' value
    Nil,

    /// An atom with a particular name
    Atom(u64),

    /// A numeric value
    Number(SafasNumber),

    /// A string value
    String(String),

    /// A character value
    Char(char),

    /// A list with a CAR and a CDR
    List(Arc<SafasCell>, Arc<SafasCell>),

    /// A monad that transforms the state of the current frame (generally a lambda)
    Monad(Arc<dyn FrameMonad<Binding=RuntimeResult>>),

    /// A macro expands to a statement, which is recursively compiled
    MacroMonad(Arc<dyn BindingMonad<Binding=Result<Arc<SafasCell>, BindError>>>),

    /// An action expands directly to a set of interpreter actions
    ActionMonad(Arc<dyn BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>>>)
}

impl SafasCell {
    ///
    /// Turns an iterator of cells into a list
    ///
    pub fn list_with_cells<Cells: IntoIterator<Item=Arc<SafasCell>>>(cells: Cells) -> Arc<SafasCell> 
    where Cells::IntoIter : DoubleEndedIterator {
        // The first cell requires special treatment
        let cells       = cells.into_iter().rev();

        // We build the list by adding to the end
        let mut cell    = Arc::new(SafasCell::Nil);
        for current_cell in cells {
            cell = Arc::new(SafasCell::List(current_cell, cell));
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
    pub fn list_value(&self) -> Option<(Arc<SafasCell>, Arc<SafasCell>)> {
        match self {
            SafasCell::List(car, cdr)   => Some((Arc::clone(car), Arc::clone(cdr))),
            _                           => None
        }
    }

    ///
    /// Converts this cell to a string
    ///
    pub fn to_string(&self) -> String {
        use self::SafasCell::*;

        match self {
            Nil                     => "()".to_string(),
            Atom(atom_id)           => name_for_atom_with_id(*atom_id),
            Number(number)          => number.to_string(),
            String(string_value)    => format!("\"{}\"", string_value),         // TODO: character quoting
            Char(chr_value)         => format!("'{}'", chr_value),              // TODO: character quoting,
            Monad(monad)            => monad.description(),
            MacroMonad(monad)       => format!("macro#{}", monad.description()),
            ActionMonad(monad)      => format!("compile#{}", monad.description()),
            List(first, second)     => {
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
    pub fn to_vec(&self) -> Option<Vec<Arc<SafasCell>>> {
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

impl Debug for SafasCell {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.to_string())
    }
}
