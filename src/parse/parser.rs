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
    let mut results = vec![];

    loop {
        let (next_cell, next_location) = parse_cell(code, location)?;
        location = next_location;

        if let Some(next_cell) = next_cell {
            // Add to the result list
            results.push(next_cell);
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
    let original_location               = location.clone();
    let (token, token_text, location)   = tokenize_no_comments(code, location);

    // Action depends on the token
    match token {
        Token::Whitespace | Token::Comment => { Err(ParseError::InternalError(original_location, "Whitespace should not make it through to this point".to_string())) },
        Token::EndOfFile    => { Ok((None, location)) }

        Token::BitNumber    => { unimplemented!() }
        Token::HexNumber    => { unimplemented!() }
        Token::IntNumber    => { unimplemented!() }
        Token::Atom         => { Ok((Some(SafasCell::Atom(get_id_for_atom_with_name(&token_text))), location)) }
        Token::Symbol(_)    => { Ok((Some(SafasCell::Atom(get_id_for_atom_with_name(&token_text))), location)) }
        Token::OpenParen    => { unimplemented!() }
        Token::CloseParen   => { unimplemented!() }
        Token::String       => { Ok((Some(SafasCell::String(unquote_string(token_text))), location)) }

        Token::Character    => {
            let chr_string = unquote_string(token_text);
            if chr_string.chars().count() != 1 {
                Err(ParseError::InvalidCharacter(original_location, chr_string))
            } else {
                Ok((Some(SafasCell::Char(chr_string.chars().nth(0).unwrap())), location))
            }
        }
    }
}

///
/// Removes the quotes around a string (or character), replaces characters like '\n' with their 'real' equivalent
///
fn unquote_string(in_string: String) -> String {
    // String should always begin with a quote
    if in_string.len() == 0 { return in_string; }

    let mut out_string  = String::new();
    let mut chars       = in_string.chars();

    // First character is skipped (will be a string quote)
    chars.next();
    let mut quoted = false;

    for chr in chars {
        match (chr, quoted) {
            ('\\', false)   => { quoted = false; }
            ('n', true)     => { out_string.push('\n'); }
            ('r', true)     => { out_string.push('\r'); }
            ('t', true)     => { out_string.push('\t'); }
            (any, _)        => { out_string.push(any); }
        }
    }

    // Remove the last character
    if out_string.chars().last() == in_string.chars().nth(0) {
        out_string.pop();
    }

    // out_string contains the result
    out_string
}
