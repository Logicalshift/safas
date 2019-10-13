use radix_fmt::*;

///
/// The output of the assembler is a vector of bitcode, which forms a series of instructions for generating the final result
/// 
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BitCode {
    /// Write out the specified set of bits and move the file offset forward that much
    Bits(u8, u128),

    /// Align to the specified number of bits
    Align(u8, u128, u32),

    /// Sets the file offset (in bits) for future instructions
    Move(u64)
}

impl BitCode {
    ///
    /// Works out the new bit position after a particular set of bitcode has been evaluated
    ///
    pub fn position_after<'a, TBitCode: IntoIterator<Item=&'a BitCode>>(initial_pos: u64, bitcode: TBitCode) -> u64 {
        use self::BitCode::*;
        let mut pos = initial_pos;

        for code_point in bitcode {
            match code_point {
                Bits(num_bits, _value)                  => pos += *num_bits as u64,
                Move(new_pos)                           => pos = *new_pos,
                Align(_bit_count, _pattern, alignment)  => {
                    let alignment   = *alignment as u64;
                    let offset      = pos % alignment;
                    if offset != 0 { pos += alignment-offset }
                }
            }
        }

        pos
    }

    ///
    /// Generates a string representation of this bitcode operation
    ///
    pub fn to_string(&self) -> String {
        match self {
            BitCode::Bits(num_bits, bits)                   => format!("d{}b{}", radix(*bits, 16), num_bits),
            BitCode::Align(num_bits, pattern, align_pos)    => format!("a{}({}b{})", align_pos, radix(*pattern, 16), num_bits),
            BitCode::Move(pos)                              => format!("m{}", radix(*pos, 16))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn position_after_bitcode() {
        assert!(BitCode::position_after(0, vec![BitCode::Bits(4, 4)].iter()) == 4);
        assert!(BitCode::position_after(0, vec![BitCode::Bits(4, 4), BitCode::Bits(4, 4)].iter()) == 8);
    }

    #[test]
    fn updates_from_initial_pos() {
        assert!(BitCode::position_after(32, vec![BitCode::Bits(4, 4)].iter()) == 36);
        assert!(BitCode::position_after(32, vec![BitCode::Bits(4, 4), BitCode::Bits(4, 4)].iter()) == 40);
    }

    #[test]
    fn position_after_move() {
        assert!(BitCode::position_after(0, vec![BitCode::Move(65536)].iter()) == 65536);
    }

    #[test]
    fn position_after_align() {
        assert!(BitCode::position_after(0, vec![BitCode::Align(8, 0, 32)].iter()) == 0);
        assert!(BitCode::position_after(0, vec![BitCode::Bits(4, 4), BitCode::Align(8, 0, 32)].iter()) == 32);
    }
}
