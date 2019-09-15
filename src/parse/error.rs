use super::file_location::*;

///
/// Indicates an error with parsing a SAFAS file
///
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Found an unimplemented feature
    Unimplemented,

    /// Suffered an interior error
    InternalError(FileLocation, String),

    /// A value is not value as a character
    InvalidCharacter(FileLocation, String)
}
