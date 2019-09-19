use super::binding_monad::*;
use super::bind_error::*;

use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;
use std::result::{Result};

///
/// The possible bindings of a symbols value
///
#[derive(Clone)]
pub enum SymbolValue {
    /// Symbol has a constant value defined by a safas cell
    Constant(Arc<SafasCell>),

    /// Symbol should be bound once the value of a particular Atom is known
    Unbound(u64),

    /// A reference to an item in a frame (or a parent frame)
    FrameReference(u32, u32),

    /// An external frame monad
    FrameMonad(Arc<dyn FrameMonad>),

    /// A macro expands to a statement, which is recursively compiled
    MacroMonad(Arc<dyn BindingMonad<Binding=Result<Arc<SafasCell>, BindError>>>),

    /// An action expands directly to a set of interpreter actions
    ActionMonad(Arc<dyn BindingMonad<Binding=Result<Arc<SmallVec<[Action; 8]>>, BindError>>>)
}
