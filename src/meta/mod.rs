//!
//! The meta module describes the metadata and memory data structures used by the assembler
//! and its macro language.
//!

mod cell;
mod number;
mod atom;
mod bitcode;

pub use cell::*;
pub use number::*;
pub use atom::*;
pub use bitcode::*;
