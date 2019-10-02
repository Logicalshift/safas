use crate::exec::*;
use crate::meta::*;

///
/// `(wrap x)` -> monad wrapping x
///
pub fn wrap_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    // Our monad just returns this wrapping function
    let wrap_fn     = FnMonad::from(|(val, ): (CellRef, )| {
        let wrap_monad  = WrapFlatMap(val.clone());
        let wrap_monad  = SafasCell::FrameMonad(Box::new(wrap_monad)).into();
        let wrap_monad  = SafasCell::Monad(SafasCell::Nil.into(), MonadType::new(wrap_monad)).into();

        wrap_monad
    });
    let wrap_fn     = ReturnsMonad(wrap_fn);

    wrap_fn
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn simple_wrap() {
        let val = eval(
                "(wrap 1)"
            ).unwrap().0;
        assert!(val.is_monad());
    }

    #[test]
    fn fun_wrap() {
        let val = eval(
                "(def y (fun (y) y))
                (y (wrap 1))"
            ).unwrap().0;
        assert!(val.is_monad());
    }

    #[test]
    fn list_wrap_1() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                (y (wrap 1) 2)"
            ).unwrap().0;
        assert!(val.is_monad());
    }

    #[test]
    fn list_wrap_2() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                (y 1 (wrap 2))"
            ).unwrap().0;
        assert!(val.is_monad());
    }

    #[test]
    fn list_wrap_3() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                (y (wrap 1) (wrap 2))"
            ).unwrap().0;
        assert!(val.is_monad());
    }

    #[test]
    fn list_wrap_4() {
        let val = eval(
                "(def y (fun (a b c) (list a b c) ))
                (y (wrap 1) (wrap 2) (wrap 3))"
            ).unwrap().0;
        assert!(val.is_monad());
    }

    #[test]
    fn list_wrap_5() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                ((fun () (y 1 (wrap 2))))"
            ).unwrap().0;
        assert!(val.is_monad());
    }

    #[test]
    fn wrap_monad_value() {
        let val = eval(
                "(wrap 1)"
            ).unwrap().0.to_string();
        assert!(val == "monad#()#(flat_map: ##wrap(1))".to_string());
    }

    #[test]
    fn monad_value_from_list_call_1() {
        let val = eval(
                "(list (wrap 1) 2)"
            ).unwrap().0.to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn monad_value_from_list_call_2() {
        let val = eval(
                "(list 1 (wrap 2))"
            ).unwrap().0.to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn def_monad() {
        // (def foo (wrap 1)) should produce a value that works like (wrap 1) (ie, which we see as a monad)
        let val = eval(
                "(def some_monad (wrap 2))
                (list 1 some_monad)"
            ).unwrap().0;
        assert!(val.is_monad());
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    /*
    #[test]
    fn fun_monad() {
        // Calling a function that returns a monad should act like a monad in context
        let val = eval(
                "(def my_wrap (fun (x) (wrap x)))
                (list 1 (my_wrap 2))"
            ).unwrap().0;
        assert!(val.is_monad());
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }
    */
}
