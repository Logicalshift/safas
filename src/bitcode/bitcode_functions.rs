use super::code::*;
use super::bitcode_monad::*;

use crate::meta::*;
use crate::exec::*;

use std::iter;
use std::convert::*;

lazy_static! {
    /// The bitcode flat_map function
    pub (super) static ref BITCODE_FLAT_MAP: CellRef = CellRef::new(SafasCell::FrameMonad(Box::new(bitcode_flat_map_fn())));
}

///
/// Arguments passed in to the function called via the bitcode flat_map function 
///
pub struct BitCodeFlatMapArgs<TValue> {
    /// The bitcode monad itself
    pub monad_value: CellRef,

    /// The value returned by flat_map
    pub value: TValue
}

impl<TValue> FnArgs for BitCodeFlatMapArgs<TValue> 
where   TValue:         TryFrom<CellRef>,
        RuntimeError:   From<<TValue as TryFrom<CellRef>>::Error> {
    fn args_from_frame(frame: &Frame) -> Result<Self, RuntimeError> {
        match &*frame.cells[0] {
            SafasCell::List(car, cdr)   => Ok(BitCodeFlatMapArgs { monad_value: car.clone(), value: TValue::try_from(cdr.clone())? }),
            _                           => Err(RuntimeError::NotAMonad(frame.cells[0].clone()))
        }
    }
}

///
/// A frame monad that performs the flat_map operation on a bitcode monad
///
struct BitCodeFlatMap;

impl FrameMonad for BitCodeFlatMap {
    type Binding = RuntimeResult;

    /// Executes this monad against a frame
    fn execute(&self, frame: Frame) -> (Frame, Self::Binding) {
        // Arguments should be flat_map arguments
        let args = frame.cells[0].clone();
        let args = match FlatMapArgs::try_from(args) { Ok(args) => args, Err(err) => return (frame, Err(err)) };

        // The monad value should be a bitcode monad of some kind
        let bitcode_monad = BitCodeMonad::from_cell(&args.monad_value);

        // Replace the empty bitcode monad with a 'full' bitcode monad
        let bitcode_monad = bitcode_monad.unwrap_or_else(|| BitCodeMonad::empty());

        let map_fn = args.map_fn;

        // Applying the map function should return the updated monad
        let monad_value         = args.monad_value.clone();
        let next                = bitcode_monad.flat_map(move |val| {
            // Fetch the map function
            let map_fn = match &*map_fn {
                SafasCell::FrameMonad(monad_fn) => monad_fn,
                _                               => return Err(RuntimeError::NotAFunction(map_fn.clone()))
            };

            // Create a new frame to execute the map function on and execute it
            let mut frame           = Frame::new(1, None);
            frame.cells[0]          = SafasCell::List(monad_value.clone(), val).into();
            let (_frame, next)      = map_fn.execute(frame);

            let next                = match next { Ok(next) => next, Err(err) => return Err(err) };

            // Result of the map function should either be a bitcode monad or nil
            let next_monad          = BitCodeMonad::from_cell(&next);

            let next_monad          = match next_monad {
                Some(next_monad)    => next_monad,
                None                => return Err(RuntimeError::MismatchedMonad(next))
            };

            Ok(next_monad)
        });
        let next                = match next { Ok(next) => next, Err(err) => return (frame, Err(err)) };

        // Wrap the resulting monad to generate the return value
        let result = SafasCell::Any(Box::new(next)).into();
        (frame, Ok(SafasCell::Monad(result, MonadType::new(BITCODE_FLAT_MAP.clone())).into()))
    }

    /// Retrieves a description of this monad when we need to display it to the user
    fn description(&self) -> String { format!("##bitcode_flatmap##") }

    /// True if the return value of this function should be treated as a monad by the binder
    fn returns_monad(&self) -> bool { true }
}

///
/// Returns the flat map function to attach to the bitcode monad
///
pub fn bitcode_flat_map_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    BitCodeFlatMap
}

///
/// The 'd' data output function
/// 
/// `(d $ffu8)` generates a bitcode monad that writes out a single byte
/// 
pub fn d_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    ReturnsMonad(FnMonad::from(|numbers: Vec<SafasNumber>| {
        use self::SafasNumber::*;
        use self::BitCode::Bits;

        // Generate the bitcode
        let bitcode = numbers.into_iter().map(|num| match num {
            Plain(val)                      => Bits(32, val),
            BitNumber(bit_count, val)       => Bits(bit_count, val),
            SignedBitNumber(bit_count, val) => Bits(bit_count, val as u128)
        });

        // Create a bitcode monad cell
        let bitcode_monad   = BitCodeMonad::write_bitcode(bitcode);
        bitcode_monad.to_cell()
    }))
}

///
/// The 'm' move to address function
/// 
/// `(m $10000)` restarts assembly at address 10000
/// 
pub fn m_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    ReturnsMonad(FnMonad::from(|(address, ): (SafasNumber, )| {
        use self::SafasNumber::*;
        use self::BitCode::Move;

        // Generate the bitcode
        let bitcode = match address {
            Plain(val)                          => Move(val as u64),
            BitNumber(_bit_count, val)          => Move(val as u64),
            SignedBitNumber(_bit_count, val)    => Move(val as u64)
        };

        // Create a bitcode monad cell
        let bitcode_monad   = BitCodeMonad::write_bitcode(iter::once(bitcode));
        bitcode_monad.to_cell()
    }))
}

///
/// The 'a' align function
/// 
/// `(a $AEAEu16 32)` aligns to the next 32-bit boundary, filling the space with the 16-bit pattern $aeae
/// 
pub fn a_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    ReturnsMonad(FnMonad::from(|(pattern, alignment_bits): (SafasNumber, SafasNumber)| {
        use self::SafasNumber::*;
        use self::BitCode::Align;

        // Generate the bitcode
        let alignment_bits  = match alignment_bits {
            Plain(val)              => val,
            BitNumber(_, val)       => val,
            SignedBitNumber(_, val) => (val.abs()) as u128
        } as u32;

        let bitcode         = match pattern {
            Plain(val)                      => Align(32, val, alignment_bits),
            BitNumber(bit_count, val)       => Align(bit_count, val, alignment_bits),
            SignedBitNumber(bit_count, val) => Align(bit_count, val as u128, alignment_bits)
        };

        // Create a bitcode monad cell
        let bitcode_monad   = BitCodeMonad::write_bitcode(iter::once(bitcode));
        bitcode_monad.to_cell()
    }))
}

#[cfg(test)]
mod test {
    use crate::bitcode::*;
    use crate::interactive::*;

    #[test]
    fn write_data_byte() {
        let result          = eval("((fun () (d $9fu8)))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode == &vec![BitCode::Bits(8, 0x9f)])
    }

    #[test]
    fn d_value_is_nil() {
        let result          = eval("((fun () (d $9fu8)))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (value, _)      = assemble(&monad).unwrap();

        assert!(value.to_string() == "()".to_string());
    }

    #[test]
    fn write_data_byte_from_monad() {
        let result          = eval("((fun () (d (wrap $9fu8))))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode == &vec![BitCode::Bits(8, 0x9f)])
    }

    #[test]
    fn write_data_byte_from_def_monad() {
        let result          = eval("(def x (wrap $9fu8)) ((fun () (d x)))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode == &vec![BitCode::Bits(8, 0x9f)])
    }

    #[test]
    fn write_data_byte_from_monad_value_is_nil() {
        let result          = eval("((fun () (d (wrap $9fu8))))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (value, _)      = assemble(&monad).unwrap();

        assert!(value.to_string() == "()".to_string());
    }

    #[test]
    fn write_three_bytes() {
        let result          = eval("((fun () (d $9fu8) (d $1c42u16)))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode == &vec![BitCode::Bits(8, 0x9f), BitCode::Bits(16, 0x1c42)])
    }

    #[test]
    fn write_three_bytes_from_monad() {
        let result          = eval("((fun () (d (wrap $9fu8)) (d (wrap $1c42u16))))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode == &vec![BitCode::Bits(8, 0x9f), BitCode::Bits(16, 0x1c42)])
    }

    #[test]
    fn write_three_bytes_in_one_operation() {
        let result          = eval("((fun () (d $9fu8 $1c42u16)))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode == &vec![BitCode::Bits(8, 0x9f), BitCode::Bits(16, 0x1c42)])
    }

    #[test]
    fn write_move() {
        let result          = eval("((fun () (m $c001)))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode ==  &vec![BitCode::Move(0xc001)])
    }

    #[test]
    fn write_align() {
        let result          = eval("((fun () (a $beeff00du32 64)))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (_, bitcode)    = assemble(&monad).unwrap();

        assert!(&bitcode ==  &vec![BitCode::Align(32, 0xbeeff00d, 64)])
    }
}
