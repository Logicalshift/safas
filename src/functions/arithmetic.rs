use crate::exec::*;
use crate::meta::*;

///
/// (+ a b c) -> a+b+c
///
pub fn add_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|items: Vec<SafasNumber>| {
        SafasCell::Number(items.into_iter().fold(SafasNumber::Plain(0), |a, b| a+b)).into()
    })
}

///
/// (- a b c) -> a-b-c
///
pub fn sub_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|items: Vec<SafasNumber>| {
        if items.len() == 0 {
            SafasCell::Number(SafasNumber::Plain(0)).into()
        } else if items.len() == 1 {
            SafasCell::Number(SafasNumber::SignedBitNumber(items[0].bits(), -items[0].to_i128())).into()
        } else {
            let initial = items[0];
            SafasCell::Number(items.into_iter().skip(1).fold(initial, |a, b| a-b)).into()
        }
    })
}

///
/// (* a b c) -> a*b*c
///
pub fn mul_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|items: Vec<SafasNumber>| {
        if items.len() == 0 {
            SafasCell::Number(SafasNumber::Plain(0)).into()
        } else {
            let initial = items[0];
            SafasCell::Number(items.into_iter().skip(1).fold(initial, |a, b| a*b)).into()
        }
    })
}

///
/// (/ a b c) -> a/b/c
///
pub fn div_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|items: Vec<SafasNumber>| {
        if items.len() == 0 {
            SafasCell::Number(SafasNumber::Plain(0)).into()
        } else {
            let initial = items[0];
            SafasCell::Number(items.into_iter().skip(1).fold(initial, |a, b| a/b)).into()
        }
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn add() {
        let val = eval(
                "(+ 4 10 6)"
            ).unwrap().to_string();
        assert!(val == "20".to_string());
    }

    #[test]
    fn sub() {
        let val = eval(
                "(- 6 3 2)"
            ).unwrap().to_string();
        assert!(val == "1".to_string());
    }

    #[test]
    fn negate() {
        let val = eval(
                "(- 6)"
            ).unwrap().to_string();
        assert!(val == "-6i3".to_string());
    }

    #[test]
    fn mul() {
        let val = eval(
                "(* 6 3 2)"
            ).unwrap().to_string();
        assert!(val == "36".to_string());
    }

    #[test]
    fn div() {
        let val = eval(
                "(/ 100 3 2)"
            ).unwrap().to_string();
        assert!(val == "16".to_string());
    }
}
