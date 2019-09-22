//!
//! The meta module describes the metadata and memory data structures used by the assembler
//! and its macro language.
//!

mod cell;
mod number;
mod atom;
mod bitcode;
mod list;

pub use self::cell::*;
pub use self::number::*;
pub use self::atom::*;
pub use self::bitcode::*;
pub use self::list::*;
