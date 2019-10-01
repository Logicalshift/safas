use crate::exec::*;
use crate::meta::*;

///
/// `(wrap x)` -> monad wrapping x
///
pub fn wrap_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(val, )| {
        let wrap    = WrapFlatMap(val);
        let wrap    = SafasCell::FrameMonad(Box::new(wrap)).into(); 
        SafasCell::Monad(SafasCell::Nil.into(), MonadType::new(wrap)).into()
    })
}
