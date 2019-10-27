use crate::meta::*;

use include_dir::*;

const LIBRARY: Dir = include_dir!("./library");

///
/// Returns the default value for the built_ins symbol
///
pub fn builtin_library() -> CellRef {
    unimplemented!()
}
