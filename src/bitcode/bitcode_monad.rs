use super::code::*;

use crate::meta::*;

use smallvec::*;
use std::sync::*;
use std::collections::{HashMap};

///
/// Represents the possible ways a value can be wrapped by a bitcode monad
///
pub enum BitCodeValue {
    /// An absolute value
    Value(CellRef),

    /// The value of the label with the specified ID
    LabelValue(u64),

    /// A monad with a label set to the specified value
    SetLabel(u64, CellRef),

    /// The ID of a new label
    AllocLabel,

    /// The current bit position (unsigned 64-bit number)
    CurrentBitPos
}

///
/// Represents the bitcode content of a bitcode monad
///
pub enum BitCodeContent {
    /// A string of bitcode
    Value(SmallVec<[BitCode; 8]>),

    /// A function that takes this monad and returns a new monad with its content
    ContentFromMapFn(CellRef),

    /// Contents of the first monad followed by the contents of the second one
    Combine(Box<BitCodeMonad>, Box<BitCodeMonad>)
}

///
/// The bitcode monad wraps a bitcode file in the process of being built up, along with 
/// any labels it might have. It's possible to reference labels whose values are not yet
/// known: a bitcode monad can be resolved to a final description of the contents of a
/// file by repeatedly resolving it until the labels values become stable.
///
pub struct BitCodeMonad {
    /// The value wrapped by this monad
    value: BitCodeValue,

    /// The bitcode contained by this monad
    bitcode: Arc<BitCodeContent>,

    /// Labels with known values
    known_labels: Arc<HashMap<u64, CellRef>>,

    /// Labels whose values were requested but had no value yet
    unknown_labels: Arc<Vec<u64>>,

    /// Label assignments generated so far (can be added to known_labels for the next pass)
    label_assignments: Arc<Vec<(u64, CellRef)>>,

    /// The bit position represented by this monad (this should always match the content of the bitcode)
    bit_pos: u64
}
