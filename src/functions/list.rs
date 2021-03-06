use crate::exec::*;
use crate::meta::*;

///
/// (list x y z) -> (x y z)
///
pub fn list_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|VarArgs(items)| items)
}

///
/// (cons a b) -> (a . b)
///
pub fn cons_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(car, cdr): (CellRef, CellRef)| {
        CellRef::new(SafasCell::List(car, cdr))
    })
}

///
/// (car a)
///
pub fn car_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(SafasList(car, _cdr), )| {
        car.clone()
    })
}

///
/// (cdr a)
///
pub fn cdr_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(SafasList(_car, cdr), )| {
        cdr.clone()
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn make_list() {
        let val = eval(
                "(list 1 2 3)"
            ).unwrap().to_string();
        assert!(val == "(1 2 3)".to_string());
    }

    #[test]
    fn evaluate_list_parameters() {
        let val = eval(
                "(list ((fun (x) x) 1) 2 3)"
            ).unwrap().to_string();
        assert!(val == "(1 2 3)".to_string());
    }

    #[test]
    fn cons() {
        let val = eval(
                "(cons 1 (list 2 3))"
            ).unwrap().to_string();
        assert!(val == "(1 2 3)".to_string());
    }

    #[test]
    fn cons_dotted() {
        let val = eval(
                "(cons 1 2)"
            ).unwrap().to_string();
        assert!(val == "(1 . 2)".to_string());
    }

    #[test]
    fn car() {
        let val = eval(
                "(car (list 1 2 3))"
            ).unwrap().to_string();
        assert!(val == "1".to_string());
    }

    #[test]
    fn cdr() {
        let val = eval(
                "(cdr (list 1 2 3))"
            ).unwrap().to_string();
        assert!(val == "(2 3)".to_string());
    }
}
