///
/// Error that can occur during evaluating a frame
///
#[derive(Clone, Debug)]
pub enum RuntimeError {
    /// Expected to pop a value from the stack but couldn't
    StackIsEmpty,

    /// Value cannot be called as a function
    NotAFunction
}
