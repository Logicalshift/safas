use radix_fmt::*;

use std::cmp::{Ordering};
use std::ops::{Add, Sub, Mul, Div};

///
/// How SAFAS represents a number
///
#[derive(Copy, Clone, Debug, PartialEq)]
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

    ///
    /// Returns this number as a u128
    ///
    pub fn to_u128(&self) -> u128 {
        use self::SafasNumber::*;

        match self {
            Plain(num)                  => *num as u128,
            BitNumber(_bits, num)       => *num as u128,
            SignedBitNumber(_bits, num) => *num as u128
        }
    }

    ///
    /// Returns this number as a i128
    ///
    pub fn to_i128(&self) -> i128 {
        use self::SafasNumber::*;

        match self {
            Plain(num)                  => *num as i128,
            BitNumber(_bits, num)       => *num as i128,
            SignedBitNumber(_bits, num) => *num as i128
        }
    }

    ///
    /// Returns the 'bits' value for this number
    ///
    pub fn bits(&self) -> u8 {
        use self::SafasNumber::*;

        match self {
            Plain(num)                  => {
                for bits in 0..128 {
                    if &(1u128<<bits) > num {
                        return bits;
                    }
                }

                return 128;
            },
            BitNumber(bits, _num)       => *bits,
            SignedBitNumber(bits, _num) => *bits
        }
    }

    ///
    /// Coerces this and another number to be the same type
    ///
    pub fn coerce(self, other: SafasNumber) -> (SafasNumber, SafasNumber) {
        use self::SafasNumber::*;

        match (self, other) {
            (SignedBitNumber(_, _), _)  => (self, SignedBitNumber(other.bits(), other.to_i128())),
            (_, SignedBitNumber(_, _))  => (SignedBitNumber(self.bits(), self.to_i128()), other),
            (BitNumber(_, _), _)        => (self, BitNumber(other.bits(), other.to_u128())),
            (_, BitNumber(_, _))        => (BitNumber(self.bits(), self.to_u128()), other),
            (Plain(_), _)               => (self, Plain(other.to_u128()))
        }
    }

    ///
    /// Returns this number as a usize
    ///
    pub fn to_usize(&self) -> usize {
        use self::SafasNumber::*;

        match self {
            Plain(num)                  => *num as usize,
            BitNumber(_bits, num)       => *num as usize,
            SignedBitNumber(_bits, num) => *num as usize
        }
    }
}

impl Add for SafasNumber {
    type Output = SafasNumber;

    fn add(self, to: SafasNumber) -> SafasNumber {
        use self::SafasNumber::*;

        let (a, b) = self.coerce(to);

        match a {
            SignedBitNumber(bits, val)  => SignedBitNumber(u8::max(bits, b.bits()), val + b.to_i128()),
            BitNumber(bits, val)        => BitNumber(u8::max(bits, b.bits()), val.wrapping_add(b.to_u128())),
            Plain(val)                  => Plain(val.wrapping_add(b.to_u128()))
        }
    }
}

impl Sub for SafasNumber {
    type Output = SafasNumber;

    fn sub(self, to: SafasNumber) -> SafasNumber {
        use self::SafasNumber::*;

        let (a, b) = self.coerce(to);

        match a {
            SignedBitNumber(bits, val)  => SignedBitNumber(u8::max(bits, b.bits()), val - b.to_i128()),
            BitNumber(bits, val)        => BitNumber(u8::max(bits, b.bits()), val.wrapping_sub(b.to_u128())),
            Plain(val)                  => Plain(val.wrapping_sub(b.to_u128()))
        }
    }
}

impl Mul for SafasNumber {
    type Output = SafasNumber;

    fn mul(self, to: SafasNumber) -> SafasNumber {
        use self::SafasNumber::*;

        let (a, b) = self.coerce(to);

        match a {
            SignedBitNumber(bits, val)  => SignedBitNumber(u8::max(bits, b.bits()), val * b.to_i128()),
            BitNumber(bits, val)        => BitNumber(u8::max(bits, b.bits()), val * b.to_u128()),
            Plain(val)                  => Plain(val * b.to_u128())
        }
    }
}

impl Div for SafasNumber {
    type Output = SafasNumber;

    fn div(self, to: SafasNumber) -> SafasNumber {
        use self::SafasNumber::*;

        let (a, b) = self.coerce(to);

        match a {
            SignedBitNumber(bits, val)  => SignedBitNumber(u8::max(bits, b.bits()), val / b.to_i128()),
            BitNumber(bits, val)        => BitNumber(u8::max(bits, b.bits()), val / b.to_u128()),
            Plain(val)                  => Plain(val / b.to_u128())
        }
    }
}

impl PartialOrd for SafasNumber {
    fn partial_cmp(&self, rhs: &SafasNumber) -> Option<Ordering> {
        use self::SafasNumber::*;

        let (a, b) = self.coerce(*rhs);

        match a {
            SignedBitNumber(_bits, val) => val.partial_cmp(&b.to_i128()),
            BitNumber(_bits, val)       => val.partial_cmp(&b.to_u128()),
            Plain(val)                  => val.partial_cmp(&b.to_u128())
        }
    }
}

impl Default for SafasNumber {
    fn default() -> Self {
        SafasNumber::Plain(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn four_divided_by_two() {
        let four = SafasNumber::SignedBitNumber(4, 4);
        let two = SafasNumber::Plain(2);

        let division = four / two;

        assert!(division == SafasNumber::SignedBitNumber(4, 2));
    }
}
