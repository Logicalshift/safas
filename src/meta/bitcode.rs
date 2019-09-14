///
/// The output of the assembler is a vector of bitcode, which forms a series of instructions for generating the final result
/// 
#[derive(Clone, Copy, Debug)]
pub enum BitCode {
    /// Write out the specified set of bits and move the file offset forward that much
    Bits(u8, u128),

    /// Align to the specified number of bits
    Align(u8, u128, u32),

    /// Sets the file offset (in bits) for future instructions
    Move(u32)
}
