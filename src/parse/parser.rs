use super::error::*;
use super::tokenizer::*;
use super::read_buffer::*;
use super::file_location::*;

use crate::meta::*;

use std::result::{Result};

///
/// Parses a file in SAFAS format and returns the resulting cell
///
pub fn parse_safas<Chars: Iterator<Item=char>>(code: &mut TokenReadBuffer<Chars>, location: FileLocation) -> Result<SafasCell, ParseError> {
    let mut location = location;

    loop {
        let (next_cell, next_location) = parse_cell(code, location)?;
        location = next_location;

        if let Some(next_cell) = next_cell {
            // Add to the result list
        } else {
            // End of file
            break;
        }

    }

    Err(ParseError::Unimplemented)
}

///
/// Parses the next cell on the token stream (returning None if there is no following cell)
///
fn parse_cell<Chars: Iterator<Item=char>>(code: &mut TokenReadBuffer<Chars>, location: FileLocation) -> Result<(Option<SafasCell>, FileLocation), ParseError> {
    // Skip whitespace and comments to find the first meaningful token
    let (token, token_text, location) = tokenize_no_comments(code, location);

    if token == Token::EndOfFile {
        // Found EOF: no token
        return Ok((None, location));
    }

    // Action depends on the token
    match token {
        Token::Whitespace | Token::Comment | Token::EndOfFile => { panic!("Whitespace tokens not expected here") },
        Token::BitNumber    => { unimplemented!() }
        Token::HexNumber    => { unimplemented!() }
        Token::IntNumber    => { unimplemented!() }
        Token::Atom         => { unimplemented!() }
        Token::Symbol(_)    => { unimplemented!() }
        Token::OpenParen    => { unimplemented!() }
        Token::CloseParen   => { unimplemented!() }
        Token::String       => { unimplemented!() }
        Token::Character    => { unimplemented!() }
    }

    unimplemented!()
}
