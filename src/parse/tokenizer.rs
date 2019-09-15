use super::file_location::*;
use super::read_buffer::*;

///
/// The types of token used by SAFAS
///
#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// 1234 or 1234u1 or similar
    IntNumber,

    /// $1234 or $1234u1 or similar
    HexNumber,

    /// 1101b4 or similar
    BitNumber,

    /// We've run out of characters
    EndOfFile
}

///
/// Reads a token from a string, starting at the specified location. Returns the token, the text for the token and the updated location.
///
pub fn tokenize<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    match buffer.read_next() {
        None            => (Token::EndOfFile, String::from(""), location),
        Some(';')       => read_comment(buffer, location),
        Some('(')       => (Token::OpenParen, buffer.read_characters(), buffer.update_location(location)),
        Some(')')       => (Token::CloseParen, buffer.read_characters(), buffer.update_location(location)),
        Some(' ')       |
        Some('\t')      |
        Some('\n')      |
        Some('\r')      => read_whitespace(buffer, location),
        Some('\'')      => read_character(buffer, location),
        Some('\"')      => read_string(buffer, location),
        Some('$')       => read_hex_number(buffer, location),

        Some(symbol)    => {
            if symbol.is_alphabetic() {
                read_atom(buffer, location)
            } else if symbol.is_numeric() {
                read_number(buffer, location)
            } else {
                (Token::Symbol(symbol), buffer.read_characters(), buffer.update_location(location))
            }
        }
    }
}

///
/// After reading a ';', reads the remainder of the comment
///
fn read_comment<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    // Read until the next newline
    loop {
        // Read the next character
        let next_chr = buffer.read_next();

        // Stop at EOF or on a newline
        match next_chr {
            None        => break,
            Some('\n')  => break,
            Some('\r')  => {
                if buffer.read_next() != Some('\n') {
                    buffer.push_back();
                }
                break;
            }
            _           =>  {}
        }
    }

    (Token::Comment, buffer.read_characters(), buffer.update_location(location))
}

///
/// After reading a whitespace character, reads the remainder of the whitespace
///
fn read_whitespace<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    unimplemented!()
}

///
/// After reading a ', reads the rest of the character value
///
fn read_character<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    unimplemented!()
}

///
/// After reading a ", reads the rest of the string value
///
fn read_string<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    unimplemented!()
}

///
/// After reading a '$', reads the rest of the hex number
///
fn read_hex_number<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    unimplemented!()
}

///
/// After reading an alphabetic character, reads the rest of the atom value
///
fn read_atom<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    unimplemented!()
}

///
/// After reading a numeric value, reads the rest of the number
///
fn read_number<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    unimplemented!()
}

#[cfg(test)]
mod test {
    use super::*;

    fn tokens_for(text: &str) -> Vec<Token> {
        let mut result  = vec![];
        let mut buf     = TokenReadBuffer::new(text.chars());

        loop {
            let (tok, _val, _location) = tokenize(&mut buf, FileLocation::new("test"));
            if tok == Token::EndOfFile { break; }
            result.push(tok);
        }

        result
    }

    #[test]
    fn tokenize_comment() {
        assert!(tokens_for("; comment") == vec![Token::Comment]);
    }

    #[test]
    fn tokenize_comment_dos_1() {
        assert!(tokens_for("; comment\r\n#") == vec![Token::Comment, Token::Symbol('#')]);
    }

    #[test]
    fn tokenize_commment_dos_2() {
        assert!(tokens_for("; comment\r#") == vec![Token::Comment, Token::Symbol('#')]);
    }

    #[test]
    fn tokenize_whitespace() {
        assert!(tokens_for("     \n\r \t  ") == vec![Token::Whitespace]);
    }

    #[test]
    fn tokenize_whitespace_dos_1() {
        assert!(tokens_for("     \r\n#") == vec![Token::Whitespace, Token::Symbol('#')]);
    }

    #[test]
    fn tokenize_whitespace_dos_2() {
        assert!(tokens_for("     \r#") == vec![Token::Whitespace, Token::Symbol('#')]);
    }

    #[test]
    fn tokenize_parens() {
        assert!(tokens_for("()") == vec![Token::OpenParen, Token::CloseParen]);
    }

    #[test]
    fn tokenize_atom() {
        assert!(tokens_for("atom") == vec![Token::Atom]);
    }

    #[test]
    fn tokenize_hexnumber_1() {
        assert!(tokens_for("$1234") == vec![Token::HexNumber]);
    }

    #[test]
    fn tokenize_hexnumber_2() {
        assert!(tokens_for("$1234u8") == vec![Token::HexNumber]);
    }

    #[test]
    fn tokenize_hexnumber_3() {
        assert!(tokens_for("$1234i4") == vec![Token::HexNumber]);
    }

    #[test]
    fn tokenize_number_1() {
        assert!(tokens_for("1234") == vec![Token::IntNumber]);
    }

    #[test]
    fn tokenize_number_2() {
        assert!(tokens_for("1234i4") == vec![Token::IntNumber]);
    }

    #[test]
    fn tokenize_number_3() {
        assert!(tokens_for("1234u5") == vec![Token::IntNumber]);
    }

    #[test]
    fn tokenize_bit_number() {
        assert!(tokens_for("1101b4") == vec![Token::BitNumber]);
    }

    #[test]
    fn tokenize_character() {
        assert!(tokens_for("\'x\'") == vec![Token::Character]);
    }

    #[test]
    fn tokenize_string() {
        assert!(tokens_for("\"x\"") == vec![Token::String]);
    }

    #[test]
    fn tokenize_symbol_1() {
        assert!(tokens_for("#") == vec![Token::Symbol('#')]);
    }

    #[test]
    fn tokenize_symbol_2() {
        assert!(tokens_for("#123") == vec![Token::Symbol('#'), Token::IntNumber]);
    }
}
