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
    use crate::meta::*;
    use crate::interactive::*;

    #[test]
    fn simple_wrap() {
        let val = eval(
                "(wrap 1)"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn fun_wrap() {
        let val = eval(
                "(def y (fun (y) y))
                (y (wrap 1))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn list_wrap_1() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                (y (wrap 1) 2)"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn list_wrap_2() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                (y 1 (wrap 2))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn list_wrap_3() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                (y (wrap 1) (wrap 2))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn list_wrap_4() {
        let val = eval(
                "(def y (fun (a b c) (list a b c) ))
                (y (wrap 1) (wrap 2) (wrap 3))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn list_wrap_5() {
        let val = eval(
                "(def y (fun (a b) (list a b) ))
                ((fun () (y 1 (wrap 2))))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
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
        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn def_monad_closure() {
        // (def foo (wrap 1)) should produce a value that works like (wrap 1) (ie, which we see as a monad)
        let val = eval(
                "(def some_monad (wrap 2))
                ( (fun () (list 1 some_monad)) )"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn fun_monad_1() {
        // A function that contains a monad should indicate it returns a monad
        let val = eval(
                "(fun (x) (wrap x))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::ReturnsMonad);
    }

    #[test]
    fn fun_monad_2() {
        // A function that contains a monad should indicate return a monad when called
        let val = eval(
                "((fun (x) (wrap x)))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn fun_monad_3() {
        // Calling a function like this should work like a monad
        let val = eval(
                "(list ((fun () (wrap 1))) 2)"
            ).unwrap().0;
        println!("{}", val.to_string());
        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn fun_monad_4() {
        // Calling a function that returns a monad should act like a monad in context
        let val = eval(
                "(def my_wrap (fun (x) (wrap x)))
                (list 1 (my_wrap 2))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn closure_monad() {
        // Calling a function that returns a monad should act like a monad in context
        let val = eval(
                "(def my_wrap (fun (x) (wrap x)))
                (def some_val 1)
                (list some_val (my_wrap 2))"
            ).unwrap().0;
        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }
}
