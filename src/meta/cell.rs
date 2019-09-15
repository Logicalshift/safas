use super::number::*;
use super::atom::*;

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
    List(Arc<SafasCell>, Arc<SafasCell>)
}

impl SafasCell {
    pub fn is_nil(&self) -> bool {
        match self {
            SafasCell::Nil  => true,
            _               => false
        }
    }

    ///
    /// Converts this cell to a string
    ///
    pub fn to_string(&self) -> String {
        use self::SafasCell::*;

        match self {
            Nil                     => "#nil".to_string(),
            Atom(atom_id)           => name_for_atom_with_id(*atom_id),
            Number(number)          => { number.to_string() },
            String(string_value)    => format!("\"{}\"", string_value),        // TODO: character quoting
            Char(chr_value)         => format!("'{}'", chr_value),                  // TODO: character quoting,
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
                            result.push_str(&format!(". {}", other.to_string()));
                            next = second;
                        }
                    }
                }

                result
            }
        }
    }
}

impl Debug for SafasCell {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.to_string())
    }
}
