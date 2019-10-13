use super::code::*;
use super::bitcode_functions::*;

use crate::meta::*;

use smallvec::*;
use std::sync::*;

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
    Uncertain(CellRef)
}

///
/// Represents the possible ways a value can be wrapped by a bitcode monad
///
#[derive(Clone, Debug)]
pub enum BitCodeValue {
    /// An absolute value
    Value(CellRef),

    /// The ID of a new label
    AllocLabel,
}

///
/// Represents the bitcode content of a bitcode monad
///
pub enum BitCodeContent {
    /// No bitcode
    Empty,

    /// A string of bitcode
    Value(SmallVec<[BitCode; 8]>),
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
    value: BitCodeValue,

    /// The bitcode contained by this monad
    bitcode: Arc<BitCodeContent>,

    /// The bit position represented by this monad (this should always match the content of the bitcode)
    bit_pos: u64
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
            bitcode:    Arc::new(BitCodeContent::Empty),
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
            bitcode:    Arc::new(BitCodeContent::Value(bitcode)),
            bit_pos:    bit_pos
        }
    }

    ///
    /// Creates a new bitcode monad that means 'allocate a new label'
    ///
    pub fn alloc_label() -> BitCodeMonad {
        BitCodeMonad {
            value:      BitCodeValue::AllocLabel,
            bitcode:    Arc::new(BitCodeContent::Empty),
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
            AllocLabel                  => unimplemented!(),
        }
    }

    ///
    /// Maps this monad by applying a function to the value it contains
    ///
    pub fn flat_map<TErr, TFn: Fn(CellRef) -> Result<BitCodeMonad, TErr>>(self, fun: TFn) -> Result<BitCodeMonad, TErr> {
        // Read the next value
        let value = self.value();

        // Retrieve the next monad based on the value
        let next = match value {
            PossibleValue::Certain(value)   => fun(value),
            PossibleValue::Uncertain(value) => fun(value)
        };

        // Result is the next monad
        // TODO: need to combine bitcode values
        // TODO: need to return a monad that can be re-evaluated in the case where the value is uncertain
        next
    }
}
