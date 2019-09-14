use super::super::meta::*;

///
/// The possible bindings of a symbols value
///
pub enum SymbolValue {
    /// Symbol has a constant value defined by a safas cell
    Constant(SafasCell),

    /// A reference to an item in a frame (or a parent frame)
    FrameReference(u32, u32),

    /// An external function
    ExternalFunction(Box<dyn Fn(SafasCell) -> SafasCell>)
}
