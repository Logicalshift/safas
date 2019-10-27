use super::file_location::*;

///
/// Indicates an error with parsing a SAFAS file
///
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Found an unimplemented feature
    Unimplemented,

    /// Suffered an interior error
    InternalError(FileLocation, String),

    /// A value is not value as a character
    InvalidCharacter(FileLocation, String),

    /// Invalid character in a bit number
    NotABitNumber(FileLocation, String),

    /// The bit count/size of a number is not set to a valid value
    InvalidBitCount(FileLocation, String),

    /// Invalid character in a hex number
    NotAHexNumber(FileLocation, String),

    /// Invalid character in an integer number
    NotAnIntegerNumber(FileLocation, String),

    /// A close parenthesis was found when one was not expected
    UnexpectedCloseParen(FileLocation),

    /// An expected close parenthesis could not be found
    MissingCloseParen(FileLocation)
}
