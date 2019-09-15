use super::error::*;

use crate::meta::*;

use std::result::{Result};

///
/// Parses a file in SAFAS format and returns the resulting cell
///
pub fn parse_safas(code: &str) -> Result<SafasCell, ParseError> {
    Err(ParseError::Unimplemented)
}
