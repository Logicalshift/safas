use crate::exec::*;
use crate::meta::*;

///
/// `(wrap x)` -> monad wrapping x
/// 
/// `wrap` is itself a monad, returning a function that returns a monad that wraps its value (this property makes
/// its return value a monad to the binder)
///
pub fn wrap_monad() -> CellRef {
    // Our monad just returns this wrapping function
    let wrap_fn     = FnMonad::from(|(val, ): (CellRef, )| {
        let wrap_monad  = WrapFlatMap(val.clone());
        let wrap_monad  = SafasCell::FrameMonad(Box::new(wrap_monad)).into();
        let wrap_monad  = SafasCell::Monad(val, MonadType::new(wrap_monad)).into();

        wrap_monad
    });
    let wrap_fn     = ReturnsMonad(wrap_fn);
    let wrap_fn     = SafasCell::FrameMonad(Box::new(wrap_fn)).into();

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
}
