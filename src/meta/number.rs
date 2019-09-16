use radix_fmt::*;

///
/// How SAFAS represents a number
///
#[derive(Copy, Clone, Debug)]
pub enum SafasNumber {
    /// A number that was specified with no precision
    Plain(u128),

    /// A number that should occupy the specified number of bits
    BitNumber(u8, u128),

    /// A signed number that should occupy the specified number of bits
    SignedBitNumber(u8, i128)
}

impl SafasNumber {
    ///
    /// Converts this number to a string representation
    ///
    pub fn to_string(&self) -> String {
        use self::SafasNumber::*;

        match self {
            Plain(val)              => val.to_string(),
            BitNumber(bits, num)    => {
                if *bits < 8 {
                    format!("{}b{}", radix(*num as u8, 2), bits)
                } else {
                    format!("${}u{}", radix(*num, 16), bits)
                }
            },

            SignedBitNumber(bits, num)  => {
                format!("{}i{}", *num, *bits)
            }
        }
    }
}
