use super::code::*;

///
/// Converts a bitcode sequence into a set of bytes
///
pub fn bitcode_to_bytes<BitCodeIterator: IntoIterator<Item=BitCode>>(bitcode: BitCodeIterator) -> Vec<u8> {
    // Allocate space for the result (start with 1024 bytes so we don't reallocate every time)
    let mut result      = vec![0u8; 1024];

    // Current and highest bit position (highest bit position sets the size of the file)
    let mut cur_bit_pos = 0usize;
    let mut max_bit_pos = 0usize;

    // Iterate through the code
    for code in bitcode {
        use self::BitCode::*;

        match code {
            Bits(len, pattern)              => { 
                // Resize so that the pattern will fit in to the result
                let new_bit_pos = cur_bit_pos + len as usize;
                while (new_bit_pos/8) > result.len() {
                    println!("Resize: {} {}", new_bit_pos/8, result.len());
                    result.resize(result.len() * 2, 0u8);
                }

                // Write out the pattern
                let mut bits_remaining      = len as usize;
                let mut pattern_remaining   = pattern;

                while bits_remaining > 0 {
                    // Work out how many bits to write
                    let mut to_write    = bits_remaining;
                    let cur_byte        = (cur_bit_pos/8)*8;
                    let next_byte       = cur_byte+8;
                    let pos             = cur_byte/8;

                    if cur_bit_pos + to_write > next_byte {
                        to_write        = next_byte - cur_bit_pos;
                    }

                    // Create the mask
                    let mask            = ((1u16 << to_write) - 1u16) as u8;
                    let shift           = cur_bit_pos - cur_byte;

                    // Mask out the bits
                    result[pos]         &= !(mask<<shift);

                    // Store the bits from the pattern
                    let pattern         =   pattern_remaining&(mask as u128);
                    result[pos]         |=  (pattern as u8)<<shift;

                    // Next part of the pattern
                    pattern_remaining   >>= to_write;
                    bits_remaining      -= to_write;
                    cur_bit_pos         += to_write;
                }
            },

            Align(len, pattern, alignment)  => { }

            Move(new_bit_pos)               => {
                // Move and resize
                cur_bit_pos = new_bit_pos as usize;
                while (cur_bit_pos >> 3) > result.len() {
                    result.resize(result.len() * 2, 0u8);
                }
            }
        }

        // Update the maximum bit position
        if cur_bit_pos > max_bit_pos { max_bit_pos = cur_bit_pos; }
    }

    let len = if (cur_bit_pos & 0x7) == 0 {
        cur_bit_pos / 8
    } else {
        cur_bit_pos / 8 + 1
    };
    result.resize(len, 0);
    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn assemble_nothing() {
        let byte = bitcode_to_bytes(vec![]);
        assert!(byte.len() == 0);
    }

    #[test]
    fn assemble_byte() {
        let byte = bitcode_to_bytes(vec![BitCode::Bits(8, 42)]);
        assert!(byte[0] == 42);
        assert!(byte.len() == 1);
    }

    #[test]
    fn assemble_two_bytes() {
        let bytes = bitcode_to_bytes(vec![BitCode::Bits(8, 42), BitCode::Bits(8, 12)]);
        assert!(bytes[0] == 42);
        assert!(bytes[1] == 12);
        assert!(bytes.len() == 2);
    }

    #[test]
    fn assemble_nybble() {
        let byte = bitcode_to_bytes(vec![BitCode::Bits(4, 0x2), BitCode::Bits(4, 0x4)]);
        assert!(byte[0] == 0x42);
        assert!(byte.len() == 1);
    }

    #[test]
    fn assemble_12_bits() {
        let bytes = bitcode_to_bytes(vec![BitCode::Bits(12, 0x654)]);
        assert!(bytes[0] == 0x54);
        assert!(bytes[1] == 0x6);
        assert!(bytes.len() == 2);
    }

    #[test]
    fn assemble_12_bits_and_a_nybble() {
        let bytes = bitcode_to_bytes(vec![BitCode::Bits(12, 0x654), BitCode::Bits(4, 0xf)]);
        assert!(bytes[0] == 0x54);
        assert!(bytes[1] == 0xf6);
        assert!(bytes.len() == 2);
    }

    #[test]
    fn assemble_5000() {
        let bytes = bitcode_to_bytes((0..5000u128).into_iter().map(|num| BitCode::Bits(8, num)));
        assert!(bytes.len() == 5000);

        for i in 0..5000 {
            assert!(bytes[i] == (i&0xff) as u8);
        }
    }

    #[test]
    fn move_8192() {
        let bytes = bitcode_to_bytes(vec![BitCode::Move(8192*8), BitCode::Bits(8, 0x42)]);
        assert!(bytes.len() == 8193);
        assert!(bytes[0] == 0x00);
        assert!(bytes[8192] == 0x42);
    }

    #[test]
    fn overwrite_nybble() {
        let byte = bitcode_to_bytes(vec![BitCode::Bits(8, 0x99), BitCode::Move(4), BitCode::Bits(4, 0xa)]);
        assert!(byte[0] == 0xa9);
        assert!(byte.len() == 1);
    }

    #[test]
    fn overwrite_middle_nybble() {
        let byte = bitcode_to_bytes(vec![BitCode::Bits(8, 0x99), BitCode::Move(2), BitCode::Bits(4, 0x9)]);
        assert!(byte[0] == 0xa5);
        assert!(byte.len() == 1);
    }
}
