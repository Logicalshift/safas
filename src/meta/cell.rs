use super::number::*;
use super::atom::*;
use super::monad_type::*;

use crate::bind::*;
use crate::exec::*;
use crate::bitcode::*;

use std::sync::*;
use std::fmt;
use std::fmt::{Debug, Formatter, Write};
use std::cmp::{Ordering};
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

impl Default for ReferenceType {
    fn default() -> Self {
        ReferenceType::Value
    }
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

    /// A boolean value
    Boolean(bool),

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

    /// A runtime error returned as a result
    Error(RuntimeError),

    /// A cell representing a node in a b-tree map (values are the key/value pairs and the child nodes)
    BTree(Vec<(CellRef, CellRef)>, Vec<CellRef>),

    /// A reference to a value on the frame
    FrameReference(usize, u32, ReferenceType),

    /// A monad with the specified type and type
    Monad(CellRef, MonadType),

    /// A monad that transforms the state of a frame (generally a lambda)
    FrameMonad(Box<dyn FrameMonad<Binding=RuntimeResult>>),

    // Syntax can describe a custom binding and compiler action
    Syntax(Box<dyn BindingMonad<Binding=SyntaxCompiler>>, CellRef),

    // Binding result for a syntax item
    BoundSyntax(SyntaxCompiler),

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
    /// Creates a cell containing an atom
    ///
    pub fn atom(name: &str) -> CellRef {
        SafasCell::Atom(get_id_for_atom_with_name(name)).into()
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
            SafasCell::BoundSyntax(syntax)                          => syntax.reference_type(),
            SafasCell::List(car, cdr)                               => {
                if let SafasCell::Syntax(syntax, _) = &**car {
                    syntax.reference_type(cdr.clone())
                } else {
                    match car.reference_type() {
                        ReferenceType::ReturnsMonad => ReferenceType::Monad,            // Calling something that returns a monad, so evaluates to a monad
                        ReferenceType::Monad        => ReferenceType::Monad,            // See compile_statement: 'calling' a monad returns a monad
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
            SafasCell::Nil              => Some(SafasNumber::Plain(0)),
            _                           => None
        }
    }

    ///
    /// If this is a boolean, the value of the boolean
    ///
    pub fn bool_value(&self) -> Option<bool> {
        match self {
            SafasCell::Boolean(val) => Some(*val),
            _                       => None
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
    /// Creates a 'a -> b' type string for displaying the contents of a B-Tree
    ///
    fn btree_to_string(&self) -> String {
        match self {
            SafasCell::BTree(key_values, child_nodes) => {
                // Start with the empty string
                let mut result = String::from("");

                if child_nodes.len() > 0 {
                    // Left values, median value, right values
                    for idx in 0..=key_values.len() {
                        write!(result, "{}", child_nodes[idx].btree_to_string()).ok();

                        if idx < key_values.len() {
                            write!(result, "\n  {} -> {}", key_values[idx].0.to_string(), key_values[idx].1.to_string()).ok();
                        }
                    }
                } else {
                    // Just the values from the leaf node
                    for (key, value) in key_values.iter() {
                        write!(result, "\n  {} -> {}", key.to_string(), value.to_string()).ok();
                    }
                }

                result
            },
            _ => "?? -> ??".to_string()
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
            Boolean(value)                                              => if *value { "=t" } else { "=f" }.to_string(),
            Number(number)                                              => number.to_string(),
            BitCode(bitcode)                                            => format!("{}", bitcode.iter().map(|bitcode| bitcode.to_string()).collect::<Vec<_>>().join("")),
            String(string_value)                                        => format!("\"{}\"", string_value),         // TODO: character quoting
            Char(chr_value)                                             => format!("'{}'", chr_value),              // TODO: character quoting,
            FrameReference(cell, frame, ReferenceType::Value)           => format!("cell#({},{})", cell, frame),
            FrameReference(cell, frame, ReferenceType::Monad)           => format!("monadcell#({},{})", cell, frame),
            FrameReference(cell, frame, ReferenceType::ReturnsMonad)    => format!("monadfncell#({},{})", cell, frame),
            Monad(cell, monad)                                          => format!("monad#{}#{}", cell.to_string(), monad.to_string()),
            FrameMonad(monad)                                           => monad.description(),
            Syntax(syntax, parameter)                                   => format!("compile#{}#{}", syntax.description(), parameter.to_string()),
            BoundSyntax(syntax)                                         => format!("bound_syntax#{:p}", syntax),
            Any(val)                                                    => format!("any#{:p}", val),
            Error(err)                                                  => format!("error#{:?}", err),
            BTree(_, _)                                                 => format!("btree#({}\n)", self.btree_to_string()),
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

impl PartialEq for SafasCell {
    fn eq(&self, rhs: &SafasCell) -> bool {
        use self::SafasCell::*;

        match (self, rhs) {
            (Nil, Nil)                                          => true,
            (Atom(a), Atom(b))                                  => a == b,
            (Boolean(a), Boolean(b))                            => a == b,
            (Number(a), Number(b))                              => a == b,
            (BitCode(a), BitCode(b))                            => a == b,
            (String(a), String(b))                              => a == b,
            (Char(a), Char(b))                                  => a == b,
            (List(a_car, a_cdr), List(b_car, b_cdr))            => a_car == b_car && a_cdr == b_cdr,
            (Error(_), Error(_))                                => false,
            (FrameReference(_, _, _), FrameReference(_, _, _))  => false,
            (Monad(_, _), Monad(_, _))                          => false,
            (FrameMonad(_), FrameMonad(_))                      => false,
            (Syntax(_, _), Syntax(_, _))                        => false,
            (BoundSyntax(_), BoundSyntax(_))                    => false,
            (Any(_), Any(_))                                    => false,

            (_, _)                                              => false
        }
    }
}

impl PartialOrd for SafasCell {
    fn partial_cmp(&self, rhs: &SafasCell) -> Option<Ordering> {
        use self::SafasCell::*;

        match (self, rhs) {
            (Nil, Nil)                                          => Some(Ordering::Equal),
            (Atom(a), Atom(b))                                  => a.partial_cmp(b),
            (Boolean(a), Boolean(b))                            => a.partial_cmp(b),
            (Number(a), Number(b))                              => a.partial_cmp(b),
            (Nil, Number(b))                                    => SafasNumber::Plain(0).partial_cmp(b),
            (Number(a), Nil)                                    => a.partial_cmp(&SafasNumber::Plain(0)),
            (BitCode(_), BitCode(_))                            => None,
            (String(a), String(b))                              => a.partial_cmp(b),
            (Char(a), Char(b))                                  => a.partial_cmp(b),
            (List(a_car, a_cdr), List(b_car, b_cdr))            => {
                match a_car.partial_cmp(b_car) {
                    None                    => None,
                    Some(Ordering::Equal)   => a_cdr.partial_cmp(b_cdr),
                    Some(order)             => Some(order)
                }
            },
            (Error(_), Error(_))                                => None,
            (FrameReference(_, _, _), FrameReference(_, _, _))  => None,
            (Monad(_, _), Monad(_, _))                          => None,
            (FrameMonad(_), FrameMonad(_))                      => None,
            (Syntax(_, _), Syntax(_, _))                        => None,
            (BoundSyntax(_), BoundSyntax(_))                    => None,
            (Any(_), Any(_))                                    => None,

            (_, _)                                              => None
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

impl From<RuntimeError> for SafasCell {
    fn from(error: RuntimeError) -> SafasCell {
        SafasCell::Error(error)
    }
}

impl Into<CellRef> for RuntimeError {
    fn into(self: RuntimeError) -> CellRef {
        SafasCell::from(self).into()
    }
}
