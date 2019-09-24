use super::number::*;

use crate::exec::*;

use std::result::{Result};
use std::convert::{TryFrom};

impl TryFrom<SafasNumber> for u8 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<u8, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(u8::try_from(val)?),
            BitNumber(_bits, val)       => Ok(u8::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(u8::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for u16 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<u16, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(u16::try_from(val)?),
            BitNumber(_bits, val)       => Ok(u16::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(u16::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for u32 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<u32, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(u32::try_from(val)?),
            BitNumber(_bits, val)       => Ok(u32::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(u32::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for u64 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<u64, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(u64::try_from(val)?),
            BitNumber(_bits, val)       => Ok(u64::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(u64::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for u128 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<u128, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(u128::try_from(val)?),
            BitNumber(_bits, val)       => Ok(u128::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(u128::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for i8 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<i8, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(i8::try_from(val)?),
            BitNumber(_bits, val)       => Ok(i8::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(i8::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for i16 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<i16, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(i16::try_from(val)?),
            BitNumber(_bits, val)       => Ok(i16::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(i16::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for i32 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<i32, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(i32::try_from(val)?),
            BitNumber(_bits, val)       => Ok(i32::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(i32::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for i64 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<i64, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(i64::try_from(val)?),
            BitNumber(_bits, val)       => Ok(i64::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(i64::try_from(val)?),
        }
    }
}

impl TryFrom<SafasNumber> for i128 {
    type Error = RuntimeError;

    fn try_from(num: SafasNumber) -> Result<i128, RuntimeError> {
        use self::SafasNumber::*;

        match num {
            Plain(val)                  => Ok(i128::try_from(val)?),
            BitNumber(_bits, val)       => Ok(i128::try_from(val)?), 
            SignedBitNumber(_bits, val) => Ok(i128::try_from(val)?),
        }
    }
}

impl Into<SafasNumber> for i32 {
    fn into(self) -> SafasNumber { SafasNumber::Plain(self as u128) }
}
