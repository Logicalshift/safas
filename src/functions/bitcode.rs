use crate::meta::*;
use crate::exec::*;

///
/// Binding monad that implements a bitcode keyword
///
pub struct BitCodeKeyword<BitCodeFn> {
    /// Function that generates code for this function
    generate_code: BitCodeFn
}

impl<BitCodeFn> BitCodeKeyword<BitCodeFn>
where BitCodeFn: Send+Sync+Fn(Vec<SafasNumber>) -> Vec<BitCode> {
    ///
    /// Creates a new bitcode generation function that uses a function that takes one or more numbers
    ///
    pub fn new(generate_code: BitCodeFn) -> BitCodeKeyword<BitCodeFn> {
        BitCodeKeyword {
            generate_code: generate_code
        }
    }
}

impl<BitCodeFn> FrameMonad for BitCodeKeyword<BitCodeFn> 
where BitCodeFn: Send+Sync+Fn(Vec<SafasNumber>) -> Vec<BitCode> {
    type Binding=RuntimeResult;

    fn description(&self) -> String {
        format!("##bitcode#{:p}##", &self)
    }

    fn resolve(&self, frame: Frame) -> (Frame, Self::Binding) {
        // Arguments are the argument list and the statements
        let args = frame.cells[0].to_vec();
        let args = match args { Some(args) => args, None => vec![] };

        // Each argument must be a number
        let args = args.into_iter()
            .map(|arg| match &*arg {
                SafasCell::Number(num)  => Ok(num.clone()),
                _                       => Err(RuntimeError::TypeMismatch(arg))
            })
            .collect::<Result<Vec<_>, _>>();

        // Type error if an argument is not a number
        let args = match args { Ok(args) => args, Err(err) => { return (frame, Err(err)); } };

        // Generate the bitcode for these values
        let bitcode = (self.generate_code)(args);

        // Append to the current frame
        let mut frame = frame;
        frame.bitcode.extend(bitcode);

        // Finish up
        (frame, Ok(SafasCell::Nil.into()))
    }
}

/// The 'D' data output keyword
pub fn d_keyword() -> impl FrameMonad<Binding=RuntimeResult> {
    BitCodeKeyword::new(|values| values.into_iter().map(|value| {
        use self::SafasNumber::*;
        use self::BitCode::Bits;

        match value {
            Plain(val)                      => Bits(32, val),
            BitNumber(bit_count, val)       => Bits(bit_count, val),
            SignedBitNumber(bit_count, val) => Bits(bit_count, val as u128)
        }
    }).collect())
}

/// The 'M' move to address keyword
pub fn m_keyword() -> impl FrameMonad<Binding=RuntimeResult> {
    // TODO: when we have an instruction pointer value, this needs to update that
    BitCodeKeyword::new(|values| values.into_iter().map(|value| {
        use self::SafasNumber::*;
        use self::BitCode::Move;

        match value {
            Plain(val)                      => Move(val as u64),
            BitNumber(_, val)               => Move(val as u64),
            SignedBitNumber(_, val)         => Move(val as u64)
        }
    }).collect())
}


/// The 'A' align keyword
pub fn a_keyword() -> impl FrameMonad<Binding=RuntimeResult> {
    BitCodeKeyword::new(|values| {
        use self::SafasNumber::*;
        use self::BitCode::Align;

        // The first parameter is the bit pattern, and the second is the alignment (in bits)
        let pattern         = if values.len() > 0 { values[0] } else { SafasNumber::BitNumber(8, 0) };
        let alignment_bits  = if values.len() > 1 { values[1] } else { SafasNumber::Plain(32) };

        let alignment_bits  = match alignment_bits {
            Plain(val)              => val,
            BitNumber(_, val)       => val,
            SignedBitNumber(_, val) => (val.abs()) as u128
        } as u32;

        let instruction     = match pattern {
            Plain(val)                      => Align(32, val, alignment_bits),
            BitNumber(bit_count, val)       => Align(bit_count, val, alignment_bits),
            SignedBitNumber(bit_count, val) => Align(bit_count, val as u128, alignment_bits)
        };

        vec![instruction]
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::interactive::*;

    #[test]
    fn write_data_byte() {
        let (_, bitcode) = eval("(d $9fu8)").unwrap();

        assert!(&*bitcode.code.borrow() == &vec![BitCode::Bits(8, 0x9f)])
    }

    #[test]
    fn write_three_bytes() {
        let (_, bitcode) = eval("(d $9fu8) (d $1c42u16)").unwrap();

        assert!(&*bitcode.code.borrow() == &vec![BitCode::Bits(8, 0x9f), BitCode::Bits(16, 0x1c42)])
    }

    #[test]
    fn write_three_bytes_in_one_operation() {
        let (_, bitcode) = eval("(d $9fu8 $1c42u16)").unwrap();

        assert!(&*bitcode.code.borrow() == &vec![BitCode::Bits(8, 0x9f), BitCode::Bits(16, 0x1c42)])
    }

    #[test]
    fn write_move() {
        let (_, bitcode) = eval("(m $c001)").unwrap();

        assert!(&*bitcode.code.borrow() == &vec![BitCode::Move(0xc001)])
    }

    #[test]
    fn write_align() {
        let (_, bitcode) = eval("(a $beeff00du32 64)").unwrap();

        assert!(&*bitcode.code.borrow() == &vec![BitCode::Align(32, 0xbeeff00d, 64)])
    }
}
