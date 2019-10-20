use super::code::*;
use super::label::*;
use super::bitcode_functions::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::mem;
use std::fmt;
use std::fmt::{Debug};

///
/// Represents the possible ways a value can be wrapped by a bitcode monad
///
#[derive(Clone)]
pub enum BitCodeValue {
    /// An absolute value
    Value(CellRef),

    /// Allocates a label with the specified ID (replacing any previous label with this ID)
    AllocLabel,

    /// Reads the label (cell returned by AllocLabel)
    LabelValue(CellRef),

    /// Sets a label value (cell returned by AllocLabel)
    SetLabelValue(CellRef, CellRef),

    /// Reads the current assembly position
    BitPos,

    /// Value is the result of a chain of flat_map operations on a bitcode monad
    FlatMap(Arc<BitCodeMonad>, Vec<Arc<dyn Fn(CellRef) -> Result<BitCodeMonad, RuntimeError>+Send+Sync>>)
}

impl Debug for BitCodeValue {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::BitCodeValue::*;

        match self {
            Value(value)                => write!(fmt, "Value({})", value.to_string()),
            AllocLabel                  => write!(fmt, "AllocLabel"),
            LabelValue(value)           => write!(fmt, "LabelValue({})", value.to_string()),
            SetLabelValue(label, value) => write!(fmt, "SetLabelValue({}, {})", label.to_string(), value.to_string()),
            BitPos                      => write!(fmt, "BitPos"),
            FlatMap(monad, flat_map)    => write!(fmt, "FlatMap({:?}, [{}])", monad, flat_map.len())
        }
    }
}

///
/// Represents the bitcode content of a bitcode monad
///
#[derive(Clone, Debug)]
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

    /// The bitcode contained by this monad (this precedes any bitcode generated as a result of this monad's value)
    pub (super) bitcode: BitCodeContent,

    /// Bitcode to generate after this monad (this follows any bitcode generated as a result of this monad's value)
    pub (super) following_bitcode: BitCodeContent,
}

impl Debug for BitCodeMonad {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "BitCodeMonad({:?}, {:?}, {:?})", self.value, self.bitcode, self.following_bitcode)
    }
}

impl BitCodeMonad {
    ///
    /// Given a cell that contains a bitcode monad (either a 'raw' one or one wrapped in a monad value), returns the
    /// bitcode monad.
    ///
    fn from_cell_no_flat_map(cell: &CellRef) -> Option<BitCodeMonad> {
        match &**cell {
            SafasCell::Any(any_val)             => any_val.downcast_ref::<BitCodeMonad>().cloned(),
            SafasCell::Monad(cell, _monad_type) => BitCodeMonad::from_cell_no_flat_map(cell),
            _                                   => None
        }
    }

    ///
    /// Attempts to retrieve a bitcode monad from a cell that contains either a monad or a 'raw' bitcode monad item.
    /// 
    /// For a monad cell, this will attempt to call flat_map to convert the monad into a bitcode monad (so this works with
    /// standard wrapped values)
    ///
    pub fn from_cell(cell: &CellRef) -> Option<BitCodeMonad> {
        match &**cell {
            SafasCell::Any(any_val)             => any_val.downcast_ref::<BitCodeMonad>().cloned(),
            SafasCell::Monad(cell, monad_type)  => {
                if let Some(result) = BitCodeMonad::from_cell(cell) { 
                    // Already a bitcode monad
                    Some(result) 
                } else {
                    // Call the map_fn to create a bitcode monad with the wrapped value
                    let create_monad    = FnMonad::from(|args: FlatMapArgs| SafasCell::Any(Box::new(BitCodeMonad::with_value(args.monad_value))).into());
                    let create_monad    = SafasCell::FrameMonad(Box::new(create_monad));
                    let (_frame, monad) = monad_type.flat_map(cell.clone(), create_monad.into(), Frame::new(1, None));

                    monad.ok()
                        .as_ref()
                        .and_then(|monad| Self::from_cell_no_flat_map(monad))
                }
            }
            _                                   => None
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
            value:              BitCodeValue::Value(NIL.clone()),
            bitcode:            BitCodeContent::Empty,
            following_bitcode:  BitCodeContent::Empty
        }
    }

    ///
    /// Creates a bitcode monad that wraps a value
    ///
    pub fn with_value(value: CellRef) -> BitCodeMonad {
        BitCodeMonad {
            value:              BitCodeValue::Value(value),
            bitcode:            BitCodeContent::Empty,
            following_bitcode:  BitCodeContent::Empty
        }
    }

    ///
    /// Creates a new bitcode monad that just means 'write this bitcode'
    ///
    pub fn write_bitcode<TBitCode: IntoIterator<Item=BitCode>>(bitcode: TBitCode) -> BitCodeMonad {
        let bitcode = bitcode.into_iter().collect();

        BitCodeMonad {
            value:              BitCodeValue::Value(NIL.clone()),
            bitcode:            BitCodeContent::Value(bitcode),
            following_bitcode:  BitCodeContent::Empty
        }
    }

    ///
    /// Creates a new bitcode monad that means 'allocate a new label'
    ///
    pub fn alloc_label() -> BitCodeMonad {
        BitCodeMonad {
            value:              BitCodeValue::AllocLabel,
            bitcode:            BitCodeContent::Empty,
            following_bitcode:  BitCodeContent::Empty
        }
    }

    ///
    /// Creates a new bitcode monad that means 'read the value of the label passed in as the argument'
    ///
    pub fn read_label_value(label: CellRef) -> BitCodeMonad {
        BitCodeMonad {
            value:              BitCodeValue::LabelValue(label),
            bitcode:            BitCodeContent::Empty,
            following_bitcode:  BitCodeContent::Empty
        }
    }

    ///
    /// Creates a new bitcode monad that means 'get the current bit position'
    ///
    pub fn read_bit_pos() -> BitCodeMonad {
        BitCodeMonad {
            value:              BitCodeValue::BitPos,
            bitcode:            BitCodeContent::Empty,
            following_bitcode:  BitCodeContent::Empty
        }
    }

    ///
    /// Creates a new bitcode monad that means 'set the value of the specified label to the value of the argument'
    ///
    pub fn set_label_value(label: CellRef, value: CellRef) -> BitCodeMonad {
        BitCodeMonad {
            value:              BitCodeValue::SetLabelValue(label, value),
            bitcode:            BitCodeContent::Empty,
            following_bitcode:  BitCodeContent::Empty
        }
    }

    ///
    /// Prepends bitcode from the specified monad onto this one (stores the bitcode from the specified monad at the start of this monad)
    ///
    pub fn prepend_bitcode(&mut self, bitcode: BitCodeContent) {
        match bitcode {
            BitCodeContent::Empty           => { }
            BitCodeContent::Value(bitcode)  => {
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
    pub fn flat_map<TFn: 'static+Fn(CellRef) -> Result<BitCodeMonad, RuntimeError>+Send+Sync>(mut self, fun: TFn) -> Result<BitCodeMonad, RuntimeError> {
        match self.value {
            BitCodeValue::AllocLabel => {
                // Labels are given a fixed value as soon as flat_map is called
                let label       = SafasCell::Any(Box::new(Label::new()));
                let mut next    = fun(label.into())?;
                next.prepend_bitcode(self.bitcode);

                Ok(next)
            },

            BitCodeValue::Value(const_value) => {
                // If this just has a constant value, we can map the bitcode immediately and combine the bitcode to get the result
                let mut next = fun(const_value)?;
                next.prepend_bitcode(self.bitcode);

                Ok(next)
            },

            BitCodeValue::FlatMap(initial, mut mappings) => {
                mappings.push(Arc::new(fun));
                self.value = BitCodeValue::FlatMap(initial, mappings);
                Ok(self)
            },

            _ => {
                // Return a flatmapped bitcode monad
                Ok(BitCodeMonad {
                    value:              BitCodeValue::FlatMap(Arc::new(self), vec![Arc::new(fun)]),
                    bitcode:            BitCodeContent::Empty,
                    following_bitcode:  BitCodeContent::Empty
                })
            }
        }
    }
}
