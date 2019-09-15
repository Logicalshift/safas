///
/// Indicates an error with parsing a SAFAS file
///
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Found an unimplemented feature
    Unimplemented
}
