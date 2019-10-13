use super::code::*;
use super::bitcode_monad::*;

use crate::exec::*;

///
/// Assembles the bitcode generated by a bitcode monad, producing the final bitcode
///
pub fn assemble(monad: BitCodeMonad) -> Result<Vec<BitCode>, RuntimeError> {
    Ok(vec![])
}
