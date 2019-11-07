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

    /// #t or #f
    Boolean,

    /// 1101b4 or similar
    BitNumber,

    /// We've run out of characters
    EndOfFile
}

///
/// Reads tokens until the first non-comment/whitespace token
///
pub fn tokenize_no_comments<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    let mut location = location;

    loop {
        let (token, token_text, next_location) = tokenize(buffer, location);
        location = next_location;

        match token {
            Token::Comment | Token::Whitespace  => { },
            _                                   => return (token, token_text, location)
        }
    }
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
        Some('#')       => {
            let next_char = buffer.read_next();
            if next_char == Some('t') || next_char == Some('f') {
                (Token::Boolean, buffer.read_characters(), buffer.update_location(location))
            } else {
                if next_char.is_some() {
                    buffer.push_back();
                }
                (Token::Symbol('#'), buffer.read_characters(), buffer.update_location(location))
            }
        },

        Some(symbol)    => {
            if symbol.is_alphabetic() || symbol == '_' {
                read_atom(buffer, location)
            } else if symbol.is_numeric() {
                read_number(buffer, location)
            } else {
                // Repeated symbol values create a longer atom
                let mut symbol_string = symbol.to_string();

                loop {
                    // Collect as many copies of the symbol as we can
                    let next = buffer.read_next();
                    if next.is_none() {
                        break;
                    } else if next != Some(symbol) {
                        buffer.push_back();
                        break;
                    }

                    symbol_string.push(symbol);
                }

                if symbol_string.len() <= 1 {
                    (Token::Symbol(symbol), buffer.read_characters(), buffer.update_location(location))
                } else {
                    (Token::Atom, buffer.read_characters(), buffer.update_location(location))
                }
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
    loop {
        let next_char = buffer.read_next();

        match next_char {
            Some(' ')       |
            Some('\t')      |
            Some('\n')      |
            Some('\r')      => {},
            None            => break,
            _               => {
                buffer.push_back();
                break;
            }
        }
    }

    (Token::Whitespace, buffer.read_characters(), buffer.update_location(location))
}

///
/// After reading a ', reads the rest of the character value
///
fn read_character<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    // Characters continue until the next '"' or the next unquoted newline character
    let mut quoted = false;
    loop {
        let next_chr    = buffer.read_next();
        let was_quoted  = quoted;
        quoted          = false;

        match (next_chr, was_quoted) {
            (None, _)           => break,
            (Some('\''), false) => break,
            (Some('\\'), false) => { quoted = true; },
            (Some('\n'), false) |
            (Some('\r'), false) => {
                buffer.push_back();
                break;
            }
            (Some(_), _)        => { }
        }
    }

    (Token::Character, buffer.read_characters(), buffer.update_location(location))
}

///
/// After reading a ", reads the rest of the string value
///
fn read_string<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    // Strings continue until the next '"' or the next unquoted newline character
    let mut quoted = false;
    loop {
        let next_chr    = buffer.read_next();
        let was_quoted  = quoted;
        quoted          = false;

        match (next_chr, was_quoted) {
            (None, _)           => break,
            (Some('"'), false)  => break,
            (Some('\\'), false) => { quoted = true; },
            (Some('\n'), false) |
            (Some('\r'), false) => {
                buffer.push_back();
                break;
            }
            (Some(_), _)        => { }
        }
    }

    (Token::String, buffer.read_characters(), buffer.update_location(location))
}

///
/// After reading an alphabetic character, reads the rest of the atom value
///
fn read_atom<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    loop {
        let next_char = buffer.read_next();

        match next_char {
            Some(chr)       => {
                if !chr.is_alphanumeric() && chr != '_' {
                    buffer.push_back();
                    break;
                }
            }
            None            => break,
        }
    }

    (Token::Atom, buffer.read_characters(), buffer.update_location(location))
}

///
/// Reads the 'u5' or 'i4' bit count suffix from a number
///
fn read_number_suffix<Chars: Iterator<Item=char>>(int_token: Token, unsigned_token: Token, buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    // Numbers are followed by 'u' or 'i' to indicate that they contain a certain number of bits
    let next_char   = buffer.read_next();
    let token       = match next_char {
        Some('u')   => unsigned_token,
        Some('i')   => int_token,
        None        => {
            return (int_token, buffer.read_characters(), buffer.update_location(location))
        }
        _           => {
            buffer.push_back();
            return (int_token, buffer.read_characters(), buffer.update_location(location))
        }
    };

    // Should be followed by a number
    let mut num_bits = 0;
    loop {
        let bit_count   = buffer.read_next();
        match bit_count {
            Some(num) => {
                if num >= '0' && num <= '9' {
                } else {
                    buffer.push_back();
                    break;
                }
            }
            None => { 
                break;
            }
        }

        num_bits += 1;
    }

    // Should be at least one bit character (up to 2 is sensible, though we use 128-bit numbers everywhere, so 3 is also possible)
    if num_bits == 0 {
        buffer.push_back();
        return (int_token, buffer.read_characters(), buffer.update_location(location))
    }

    (token, buffer.read_characters(), buffer.update_location(location))
}

///
/// After reading a '$', reads the rest of the hex number
///
fn read_hex_number<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    loop {
        let next_char = buffer.read_next();

        match next_char {
            Some(chr)       => {
                if (chr >= '0' && chr <= '9') || (chr >= 'a' && chr <= 'f') || (chr >= 'A' && chr <= 'F') {

                } else {
                    buffer.push_back();
                    break;
                }
            }
            None            => break,
        }
    }

    read_number_suffix(Token::HexNumber, Token::HexNumber, buffer, location)
}

///
/// After reading a numeric value, reads the rest of the number
///
fn read_number<Chars: Iterator<Item=char>>(buffer: &mut TokenReadBuffer<Chars>, location: FileLocation) -> (Token, String, FileLocation) {
    loop {
        let next_char = buffer.read_next();

        match next_char {
            Some(chr)       => {
                if chr >= '0' && chr <= '9' {

                } else {
                    buffer.push_back();
                    break;
                }
            }
            None            => break,
        }
    }

    match buffer.read_next() {
        Some('b') => {
            // Might be a bitnumber (number followed by 'b<bits>')
            let mut num_bits = 0;
            loop {
                let bit_count   = buffer.read_next();
                match bit_count {
                    Some(num) => {
                        if num >= '0' && num <= '9' {
                        } else {
                            buffer.push_back();
                            break;
                        }
                    }
                    None => { 
                        break;
                    }
                }

                num_bits += 1;
            }

            // Should be at least one bit character (up to 2 is sensible, though we use 128-bit numbers everywhere, so 3 is also possible)
            if num_bits == 0 {
                buffer.push_back();
                (Token::IntNumber, buffer.read_characters(), buffer.update_location(location))
            } else {
                (Token::BitNumber, buffer.read_characters(), buffer.update_location(location))
            }
        }

        Some(_) => {
            // Not a bitnumber
            buffer.push_back();
            read_number_suffix(Token::IntNumber, Token::IntNumber, buffer, location)
        },

        None => {
            // No suffix
            (Token::IntNumber, buffer.read_characters(), buffer.update_location(location))
        }
    }
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

    fn tokens_no_comments_for(text: &str) -> Vec<Token> {
        let mut result  = vec![];
        let mut buf     = TokenReadBuffer::new(text.chars());

        loop {
            let (tok, _val, _location) = tokenize_no_comments(&mut buf, FileLocation::new("test"));
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
    fn tokenize_atom_1() {
        assert!(tokens_for("atom") == vec![Token::Atom]);
    }

    #[test]
    fn tokenize_atom_2() {
        assert!(tokens_for("atom123") == vec![Token::Atom]);
    }

    #[test]
    fn tokenize_atom_3() {
        assert!(tokens_for("atom_123") == vec![Token::Atom]);
    }

    #[test]
    fn tokenize_atom_4() {
        assert!(tokens_for("_123") == vec![Token::Atom]);
    }

    #[test]
    fn tokenize_atom_5() {
        // <atom> is used in syntax definitions
        assert!(tokens_for("<atom>") == vec![Token::Symbol('<'), Token::Atom, Token::Symbol('>')]);
    }

    #[test]
    fn tokenize_atom_6() {
        // Repeated symbols arrive as single atoms
        assert!(tokens_for("...") == vec![Token::Atom]);
    }

    #[test]
    fn tokenize_atom_7() {
        // Repeated symbols arrive as single atoms
        assert!(tokens_for("<<") == vec![Token::Atom]);
    }

    #[test]
    fn tokenize_atom_8() {
        // But symbols don't combine into longer atoms when they're repeated
        assert!(tokens_for("<<atom>>") == vec![Token::Atom, Token::Atom, Token::Atom]);
    }

    #[test]
    fn tokenize_atom_9() {
        // But symbols don't combine into longer atoms when they're repeated
        assert!(tokens_for("<<a>>") == vec![Token::Atom, Token::Atom, Token::Atom]);
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
    fn tokenize_hexnumber_4() {
        assert!(tokens_for("$1234irq") == vec![Token::HexNumber, Token::Atom]);
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

    #[test]
    fn tokenize_symbol_3() {
        assert!(tokens_for("#atom") == vec![Token::Symbol('#'), Token::Atom]);
    }

    #[test]
    fn tokenize_boolean_1() {
        assert!(tokens_for("#t") == vec![Token::Boolean]);
    }

    #[test]
    fn tokenize_boolean_2() {
        assert!(tokens_for("#f") == vec![Token::Boolean]);
    }

    #[test]
    fn tokenize_skipping_comments_and_whitespace() {
        assert!(tokens_no_comments_for("# atom ; comment") == vec![Token::Symbol('#'), Token::Atom]);
    }
}
