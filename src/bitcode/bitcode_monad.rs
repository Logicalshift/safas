use super::code::*;
use super::label::*;
use super::bitcode_functions::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::mem;

///
///
/// Some value may need to be recomputed in multiple passes to get their final 
/// value (eg, label values or the current bit position)
///
#[derive(Clone)]
pub enum PossibleValue {
    /// Value was resolved to an absolute value
    Certain(CellRef),

    /// Value is not certain (could need to be revised in a future pass)
    Uncertain(CellRef),

    /// Value is invalid
    Invalid
}

///
/// Represents the possible ways a value can be wrapped by a bitcode monad
///
#[derive(Clone)]
pub enum BitCodeValue {
    /// An absolute value
    Value(CellRef),

    /// Allocates a label with the specified ID (replacing any previous label with this ID)
    AllocLabel(usize),

    /// Reads the label with the specified ID's value
    LabelValue(usize),

    /// Value is the result of a flat_map operation on a bitcode monad
    FlatMap(Arc<(BitCodeMonad, Box<dyn Fn(CellRef) -> Result<BitCodeMonad, RuntimeError>+Send+Sync>)>)
}

///
/// Represents the bitcode content of a bitcode monad
///
#[derive(Clone)]
pub enum BitCodeContent {
    /// No bitcode
    Empty,

    /// A string of bitcode
    Value(SmallVec<[BitCode; 8]>),
}

impl BitCodeContent {
    ///
    /// Takes the content and replaces it with 'empty'
    ///
    pub fn take(&mut self) -> BitCodeContent {
        let mut result = BitCodeContent::Empty;
        mem::swap(self, &mut result);
        result
    }
}

///
/// The bitcode monad wraps a bitcode file in the process of being built up, along with 
/// any labels it might have. It's possible to reference labels whose values are not yet
/// known: a bitcode monad can be resolved to a final description of the contents of a
/// file by repeatedly resolving it until the labels values become stable.
///
#[derive(Clone)]
pub struct BitCodeMonad {
    /// The value wrapped by this monad
    pub (super) value: BitCodeValue,

    /// The bitcode contained by this monad (this follows on from any bitcode generated due to the value)
    pub (super) bitcode: BitCodeContent,

    /// The bit position represented by this monad (this should always match the content of the bitcode)
    pub (super) bit_pos: u64,
}

impl BitCodeMonad {
    ///
    /// Attempts to retrieve a bitcode monad from a cell
    ///
    pub fn from_cell(cell: &CellRef) -> Option<BitCodeMonad> {
        match &**cell {
            SafasCell::Monad(cell, _)   => BitCodeMonad::from_cell(cell),
            SafasCell::Any(any_val)     => any_val.downcast_ref::<BitCodeMonad>().cloned(),
            SafasCell::Nil              => Some(BitCodeMonad::empty()),
            _                           => None
        }
    }

    ///
    /// Converts this bitcode monad to a cell reference
    ///
    pub fn to_cell(self) -> CellRef {
        // Create a bitcode monad cell
        let bitcode_monad   = SafasCell::Any(Box::new(self));
        let monad_type      = MonadType::new(BITCODE_FLAT_MAP.clone());
        let bitcode_monad   = SafasCell::Monad(bitcode_monad.into(), monad_type);

        bitcode_monad.into()
    }

    ///
    /// Creates an empty bitcode monad
    ///
    pub fn empty() -> BitCodeMonad {
        BitCodeMonad {
            value:      BitCodeValue::Value(SafasCell::Nil.into()),
            bitcode:    BitCodeContent::Empty,
            bit_pos:    0
        }
    }

    ///
    /// Creates a bitcode monad that wraps a value
    ///
    pub fn with_value(value: CellRef) -> BitCodeMonad {
        BitCodeMonad {
            value:      BitCodeValue::Value(value),
            bitcode:    BitCodeContent::Empty,
            bit_pos:    0
        }
    }

    ///
    /// Creates a new bitcode monad that just means 'write this bitcode'
    ///
    pub fn write_bitcode<TBitCode: IntoIterator<Item=BitCode>>(bitcode: TBitCode) -> BitCodeMonad {
        let bitcode = bitcode.into_iter().collect();
        let bit_pos = BitCode::position_after(0, &bitcode);

        BitCodeMonad {
            value:      BitCodeValue::Value(SafasCell::Nil.into()),
            bitcode:    BitCodeContent::Value(bitcode),
            bit_pos:    bit_pos
        }
    }

    ///
    /// Creates a new bitcode monad that means 'allocate a new label'
    ///
    pub fn alloc_label(id: usize) -> BitCodeMonad {
        BitCodeMonad {
            value:      BitCodeValue::AllocLabel(id),
            bitcode:    BitCodeContent::Empty,
            bit_pos:    0
        }
    }

    ///
    /// Creates a new bitcode monad that means 'read the value of the label passed in as the argument'
    ///
    pub fn read_label_value(label_id: usize) -> BitCodeMonad {
        BitCodeMonad {
            value:      BitCodeValue::LabelValue(label_id),
            bitcode:    BitCodeContent::Empty,
            bit_pos:    0
        }
    }

    ///
    /// Retrieves the value attached to this monad
    ///
    pub fn value(&self) -> PossibleValue {
        use self::BitCodeValue::*;
        use self::PossibleValue::*;

        match &self.value {
            Value(value)                => Certain(value.clone()),
            AllocLabel(label_id)        => unimplemented!("Labels should be tracked outside of the bitcode monad"),
            LabelValue(label_id)        => unimplemented!("Labels should be tracked outside of the bitcode monad"),
            FlatMap(flat_map)           => unimplemented!("value() won't work with a FlatMap bitcode monad (needs to be assembled)")
        }
    }

    ///
    /// Updates the bit_pos in this monad with a new starting position
    ///
    pub fn update_bit_pos_starting_at(&mut self, initial_bit_pos: u64) {
        match &self.bitcode {
            BitCodeContent::Empty           => { self.bit_pos = initial_bit_pos; }
            BitCodeContent::Value(bitcode)  => { self.bit_pos = BitCode::position_after(initial_bit_pos, bitcode); }
        }
    }

    ///
    /// Prepends bitcode from the specified monad onto this one (stores the bitcode from the specified monad at the start of this monad)
    ///
    pub fn prepend_bitcode(&mut self, from_bit_pos: u64, bitcode: BitCodeContent) {
        match bitcode {
            BitCodeContent::Empty           => { self.update_bit_pos_starting_at(from_bit_pos); }
            BitCodeContent::Value(bitcode)  => {
                self.update_bit_pos_starting_at(from_bit_pos);
                match self.bitcode.take() {
                    BitCodeContent::Empty               => { self.bitcode = BitCodeContent::Value(bitcode); }
                    BitCodeContent::Value(our_bitcode)  => {
                        let mut bitcode = bitcode;
                        bitcode.extend(our_bitcode);
                        self.bitcode = BitCodeContent::Value(bitcode);
                    }
                }
            }
        }
    }

    ///
    /// Maps this monad by applying a function to the value it contains
    ///
    pub fn flat_map<TFn: 'static+Fn(CellRef) -> Result<BitCodeMonad, RuntimeError>+Send+Sync>(self, fun: TFn) -> Result<BitCodeMonad, RuntimeError> {
        // Return a flatmapped bitcode monad
        Ok(BitCodeMonad {
            value:      BitCodeValue::FlatMap(Arc::new((self, Box::new(fun)))),
            bitcode:    BitCodeContent::Empty,
            bit_pos:    0
        })
    }

    /*
    ///
    /// Maps this monad by applying a function to the value it contains
    ///
    pub fn flat_map<TFn: Fn(CellRef) -> Result<BitCodeMonad, RuntimeError>>(self, fun: TFn) -> Result<BitCodeMonad, RuntimeError> {
        // Read the next value
        let value = self.value();

        // Retrieve the next monad based on the value
        let next = match value {
            PossibleValue::Certain(value)   => fun(value),
            PossibleValue::Uncertain(value) => fun(value),
            PossibleValue::Invalid          => fun(SafasCell::Nil.into())
        };

        // Prepend the bitcode from the previous monad to the new one
        let next = next.map(|mut next| { next.prepend_bitcode(&self); next });
        let next = next.map(|mut next| { next.prepend_labels(&self); next });

        // Result is the next monad
        // TODO: need to return a monad that can be re-evaluated in the case where the value is uncertain
        next
    }
    */
}
