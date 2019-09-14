use super::number::*;

use std::sync::*;

///
/// A SAFAS cell represents a single value: for example 'D' or '24'
/// 
/// The most complicated of these structures is the list, which just joins two cells
///
#[derive(Clone, Debug)]
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
