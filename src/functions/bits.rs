use crate::exec::*;
use crate::meta::*;

///
/// (bits 3 8) -> 8u3
///
pub fn bits_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(CellValue(bits), number): (_, SafasNumber)| {
        let mask    = (1u128<<bits)-1;
        let number  = match number {
            SafasNumber::Plain(val)                     => val as u128,
            SafasNumber::BitNumber(_bits, val)          => val,
            SafasNumber::SignedBitNumber(_bits, val)    => val as u128
        };
        let number  = number & mask;

        CellRef::new(SafasCell::Number(SafasNumber::BitNumber(bits, number)))
    })
}


///
/// (sbits 8 $ff) -> -1i8
///
pub fn sbits_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(CellValue(bits), CellValue(number)): (_, CellValue<i128>)| {
        let mask    = (1u128<<bits)-1;
        let number  = (number as u128) & mask;

        let number = if number & (1u128<<(bits-1)) != 0 {
            let sign_extend = (-1 << bits) as u128;
            let number      = number | sign_extend;
            number as i128
        } else {
            number as i128
        };

        CellRef::new(SafasCell::Number(SafasNumber::SignedBitNumber(bits, number as i128)))
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn simple_bits() {
        let val = eval(
                "(bits 8 $ae)"
            ).unwrap().to_string();
        assert!(val == "$aeu8".to_string());
    }

    #[test]
    fn bits_truncate() {
        let val = eval(
                "(bits 16 $fee7f00d)"
            ).unwrap().to_string();
        assert!(val == "$f00du16".to_string());
    }

    #[test]
    fn sbits_positive() {
        let val = eval(
                "(sbits 16 1000)"
            ).unwrap().to_string();
        assert!(val == "1000i16".to_string());
    }

    #[test]
    fn sbits_negative() {
        let val = eval(
                "(sbits 8 $ff)"
            ).unwrap().to_string();
        assert!(val == "-1i8".to_string());
    }

    #[test]
    fn sbits_and_sbits() {
        let val = eval(
                "(sbits 16 (sbits 8 $ff))"
            ).unwrap().to_string();
        assert!(val == "-1i16".to_string());
    }

    #[test]
    fn bits_and_sbits() {
        let val = eval(
                "(bits 16 (sbits 8 $ff))"
            ).unwrap().to_string();
        assert!(val == "$ffffu16".to_string());
    }
}
