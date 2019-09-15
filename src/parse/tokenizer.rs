///
/// The types of token used by SAFAS
///
#[derive(Debug, Clone, Copy)]
pub enum Token {
    /// '; comment'
    Comment,

    /// Any amount of whitespace
    Whitespace,

    /// '('
    OpenParen,

    /// ')'
    CloseParen,

    /// A symbolic character ('-', '#', etc)
    Symbol(char),

    /// An atomic name
    Atom,

    /// A string in quotes
    String,

    /// A character in quotes
    Character,

    /// $1234 or $1234u1 or similar
    HexNumber,

    /// 1101b4 or similar
    BitNumber,
}
