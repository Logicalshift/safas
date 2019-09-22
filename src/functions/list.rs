use crate::exec::*;
use crate::meta::*;

use std::sync::*;

///
/// (list x y z) -> (x y z)
///
pub fn list_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|items: Arc<SafasCell>| items)
}

///
/// (cons a b) -> (a . b)
///
pub fn cons_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(car, cdr): (Arc<SafasCell>, Arc<SafasCell>)| {
        Arc::new(SafasCell::List(car, cdr))
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn make_list() {
        let val = eval(
                "(list 1 2 3)"
            ).unwrap().0.to_string();
        assert!(val == "(1 2 3)".to_string());
    }

    #[test]
    fn evaluate_list_parameters() {
        let val = eval(
                "(list ((fun (x) x) 1) 2 3)"
            ).unwrap().0.to_string();
        assert!(val == "(1 2 3)".to_string());
    }

    #[test]
    fn cons_list() {
        let val = eval(
                "(cons 1 (list 2 3))"
            ).unwrap().0.to_string();
        assert!(val == "(1 2 3)".to_string());
    }
}
