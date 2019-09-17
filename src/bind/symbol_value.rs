use crate::exec::*;
use crate::meta::*;

use std::sync::*;

///
/// The possible bindings of a symbols value
///
#[derive(Clone)]
pub enum SymbolValue {
    /// Symbol has a constant value defined by a safas cell
    Constant(SafasCell),

    /// Symbol should be bound once the value of a particular Atom is known
    Unbound(u64),

    /// A reference to an item in a frame (or a parent frame)
    FrameReference(u32, u32),

    /// An external function
    ExternalFunction(Arc<dyn Fn(SafasCell) -> SafasCell>),

    /// An external frame monad
    FrameMonad(Arc<dyn FrameMonad>)
}
