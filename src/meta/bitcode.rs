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
