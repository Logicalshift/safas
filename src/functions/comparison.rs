use crate::exec::*;
use crate::meta::*;

use std::cmp::{Ordering};

///
/// `(> a b)` -> TRUE/FALSE
///
pub fn gt_fn()  -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(a, b): (CellRef, CellRef)| {

        if let Some(order) = (&*a).partial_cmp(&*b) {
            Ok(CellRef::new(SafasCell::Boolean(order == Ordering::Greater)))
        } else {
            Err(RuntimeError::CannotCompare(a, b))
        }

    })
}

///
/// `(>= a b)` -> TRUE/FALSE
///
pub fn ge_fn()  -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(a, b): (CellRef, CellRef)| {

        if let Some(order) = (&*a).partial_cmp(&*b) {
            Ok(CellRef::new(SafasCell::Boolean(order == Ordering::Greater || order == Ordering::Equal)))
        } else {
            Err(RuntimeError::CannotCompare(a, b))
        }

    })
}

///
/// `(< a b)` -> TRUE/FALSE
///
pub fn lt_fn()  -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(a, b): (CellRef, CellRef)| {

        if let Some(order) = (&*a).partial_cmp(&*b) {
            Ok(CellRef::new(SafasCell::Boolean(order == Ordering::Less)))
        } else {
            Err(RuntimeError::CannotCompare(a, b))
        }

    })
}

///
/// `(<= a b)` -> TRUE/FALSE
///
pub fn le_fn()  -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(a, b): (CellRef, CellRef)| {

        if let Some(order) = (&*a).partial_cmp(&*b) {
            Ok(CellRef::new(SafasCell::Boolean(order == Ordering::Less || order == Ordering::Equal)))
        } else {
            Err(RuntimeError::CannotCompare(a, b))
        }

    })
}

///
/// `(= a b)` -> TRUE/FALSE
///
pub fn eq_fn()  -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(a, b): (CellRef, CellRef)| {

        Ok(CellRef::new(SafasCell::Boolean((&*a) == (&*b))))

    })
}

///
/// `(!= a b)` -> TRUE/FALSE
///
pub fn ne_fn()  -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(a, b): (CellRef, CellRef)| {

        Ok(CellRef::new(SafasCell::Boolean((&*a) != (&*b))))

    })
}


#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    pub fn two_greater_than_one() {
        let val = eval(
                "(> 2 1)"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }

    #[test]
    pub fn one_not_greater_than_two() {
        let val = eval(
                "(> 1 2)"
            ).unwrap().to_string();
        assert!(val == "=f".to_string());
    }

    #[test]
    pub fn two_not_greater_than_two() {
        let val = eval(
                "(> 2 2)"
            ).unwrap().to_string();
        assert!(val == "=f".to_string());
    }

    #[test]
    pub fn two_greater_than_or_equal_to_two() {
        let val = eval(
                "(>= 2 2)"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }

    #[test]
    pub fn one_less_than_two() {
        let val = eval(
                "(< 1 2)"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }

    #[test]
    pub fn two_less_than_or_equal_to_two() {
        let val = eval(
                "(<= 2 2)"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }

    #[test]
    pub fn two_equals_two() {
        let val = eval(
                "(= 2 2)"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }

    #[test]
    pub fn one_gt_nil() {
        let val = eval(
                "(> 1 ())"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }

    #[test]
    pub fn one_not_equals_two() {
        let val = eval(
                "(!= 1 2)"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }

    #[test]
    pub fn string_equals_string() {
        let val = eval(
                "(= \"hello\" \"hello\")"
            ).unwrap().to_string();
        assert!(val == "=t".to_string());
    }
}
