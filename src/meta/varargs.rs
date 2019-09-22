use super::cell::*;

use std::sync::*;

///
/// Varargs is used with FnMonad to represent a function that can take any number of parameters
///
pub struct VarArgs(pub Arc<SafasCell>);
