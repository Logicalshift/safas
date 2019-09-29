use super::cell::*;

use std::sync::*;

///
/// Represents the type of a monad cell
/// 
/// These are treated specially when binding functions (or macros) in that they turn their parent
/// function into a similar monad derived from this one.
///
pub struct MonadType {
    /// Represents the flat_map function. Should be `fn (fn y -> Monad) -> Monad`, extracting the value contained within this monad
    flat_map: CellRef,

    /// Represents the wrap function. Should be `fn x -> Monad`, providing a way to wrap a value in this monad
    wrap: CellRef
}

///
/// Reference to a monad type
///
pub type MonadTypeRef = Arc<MonadType>;
