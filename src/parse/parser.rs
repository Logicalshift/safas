use super::error::*;
use super::tokenizer::*;
use super::read_buffer::*;
use super::file_location::*;

use crate::meta::*;

use smallvec::*;
use std::result::{Result};
use std::sync::*;

///
/// Parses a file in SAFAS format and returns the resulting cell
///
pub fn parse_safas<Chars: Iterator<Item=char>>(code: &mut TokenReadBuffer<Chars>, location: FileLocation) -> Result<Arc<SafasCell>, ParseError> {
    // Initial location
    let mut location    = location;

    // Results, stored as a 
    let mut results     = vec![];

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

    Ok(SafasCell::list_with_cells(results))
}

///
/// Parses the next cell on the token stream (returning None if there is no following cell)
///
fn parse_cell<Chars: Iterator<Item=char>>(code: &mut TokenReadBuffer<Chars>, location: FileLocation) -> Result<(Option<Arc<SafasCell>>, FileLocation), ParseError> {
    // Skip whitespace and comments to find the first meaningful token
    let original_location               = location.clone();
    let (token, token_text, location)   = tokenize_no_comments(code, location);

    parse_cell_from_token(code, original_location, token, token_text, location)
}

///
/// Parses the next cell on the token stream, with a token read from the stream (returning None if there is no following cell)
///
fn parse_cell_from_token<Chars: Iterator<Item=char>>(code: &mut TokenReadBuffer<Chars>, original_location: FileLocation, token: Token, token_text: String, location: FileLocation) -> Result<(Option<Arc<SafasCell>>, FileLocation), ParseError> {
    // Action depends on the token
    match token {
        Token::Whitespace | Token::Comment => { Err(ParseError::InternalError(original_location, "Whitespace should not make it through to this point".to_string())) },
        Token::EndOfFile    => { Ok((None, location)) }

        Token::BitNumber    => { (Ok((Some(Arc::new(bit_number(&token_text, &original_location)?)), location))) }
        Token::HexNumber    => { (Ok((Some(Arc::new(hex_number(&token_text, &original_location)?)), location))) }
        Token::IntNumber    => { (Ok((Some(Arc::new(int_number(&token_text, &original_location)?)), location))) }
        Token::Atom         => { Ok((Some(Arc::new(SafasCell::Atom(get_id_for_atom_with_name(&token_text)))), location)) }
        Token::Symbol(_)    => { Ok((Some(Arc::new(SafasCell::Atom(get_id_for_atom_with_name(&token_text)))), location)) }
        Token::OpenParen    => { unimplemented!() }
        Token::CloseParen   => { unimplemented!() }
        Token::String       => { Ok((Some(Arc::new(SafasCell::String(unquote_string(token_text)))), location)) }

        Token::Character    => {
            let chr_string = unquote_string(token_text);
            if chr_string.chars().count() != 1 {
                Err(ParseError::InvalidCharacter(original_location, chr_string))
            } else {
                Ok((Some(Arc::new(SafasCell::Char(chr_string.chars().nth(0).unwrap()))), location))
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

///
/// Parses a bit number (01010b6) as a cell
///
fn bit_number(number_string: &str, location: &FileLocation) -> Result<SafasCell, ParseError> {
    // Fetch the characters from the string
    let chrs = number_string.chars().collect::<SmallVec<[_; 8]>>();

    // Format is '1111b8'
    let mut num = 0u128;
    let mut bits = 0u8;

    // Location of the 'b' indicating the number of bits in the number
    let b_pos = chrs.iter()
        .enumerate()
        .filter(|(_pos, chr)| **chr == 'b')
        .map(|(pos, _chr)| pos)
        .nth(0)
        .ok_or_else(|| ParseError::NotABitNumber(location.clone(), number_string.to_string()))?;

    // Parse the bits themselves
    for idx in 0..b_pos {
        num <<= 1;
        match chrs[idx] {
            '0' => { num |= 0; }
            '1' => { num |= 1; }
            _   => { return Err(ParseError::NotABitNumber(location.clone(), number_string.to_string())); }
        }
    }

    // Parse the number of bits
    for idx in (b_pos+1)..chrs.len() {
        bits *= 10;
        if chrs[idx] >= '0' && chrs[idx] <= '9' {
            bits += ((chrs[idx] as u16) - ('0' as u16)) as u8;
        } else {
            return Err(ParseError::InvalidBitCount(location.clone(), number_string.to_string()));
        }
    }

    Ok(SafasCell::Number(SafasNumber::BitNumber(bits, num)))
}

///
/// Parses a hex number ($12ffu8) as a cell
///
fn hex_number(number_string: &str, location: &FileLocation) -> Result<SafasCell, ParseError> {
    // Fetch the characters from the string
    let chrs = number_string.chars().collect::<SmallVec<[_; 8]>>();

    if chrs[0] != '$' {
        return Err(ParseError::NotAHexNumber(location.clone(), number_string.to_string()));
    }

    // Format is '1111b8'
    let mut num = 0u128;
    let mut bits = 0u8;

    // Location of the 'b' indicating the number of bits in the number
    let b_pos = chrs.iter()
        .enumerate()
        .filter(|(_pos, chr)| **chr == 'u' || **chr == 'i')
        .map(|(pos, _chr)| pos)
        .nth(0)
        .unwrap_or_else(|| chrs.len());

    // Parse the bits themselves
    for idx in 1..b_pos {
        num <<= 4;

        let chr = chrs[idx];

        if chr >= '0' && chr <= '9' {
            num |= ((chr as u8) - ('0' as u8)) as u128;
        } else if chr >= 'a' && chr <= 'f' {
            num |= ((chr as u8) - ('a' as u8) + 10) as u128;
        } else if chr >= 'A' && chr <= 'F' {
            num |= ((chr as u8) - ('A' as u8) + 10) as u128;
        } else {
            return Err(ParseError::NotAHexNumber(location.clone(), number_string.to_string()));
        }
    }

    if b_pos < chrs.len() {
        // Parse the number of bits
        for idx in (b_pos+1)..chrs.len() {
            bits *= 10;
            if chrs[idx] >= '0' && chrs[idx] <= '9' {
                bits += ((chrs[idx] as u16) - ('0' as u16)) as u8;
            } else {
                return Err(ParseError::InvalidBitCount(location.clone(), number_string.to_string()));
            }
        }

        if chrs[b_pos] == 'i' {
            let num = if num & (1<<(bits-1)) != 0 {
                let sign_extend = -1i128 << bits;
                num | (sign_extend as u128)
            } else {
                num
            };

            Ok(SafasCell::Number(SafasNumber::SignedBitNumber(bits, num as i128)))
        } else {
            Ok(SafasCell::Number(SafasNumber::BitNumber(bits, num)))
        }
    } else {
        Ok(SafasCell::Number(SafasNumber::Plain(num)))
    }
}

///
/// Parses an int number (1234u8) as a cell
///
fn int_number(number_string: &str, location: &FileLocation) -> Result<SafasCell, ParseError> {
    // Fetch the characters from the string
    let chrs        = number_string.chars().collect::<SmallVec<[_; 8]>>();
    let is_negative = chrs[0] == '-';
    let n_start     = if is_negative { 1 } else { 0 };

    // Format is '1111b8'
    let mut num = 0u128;
    let mut bits = 0u8;

    // Location of the 'b' indicating the number of bits in the number
    let b_pos = chrs.iter()
        .enumerate()
        .filter(|(_pos, chr)| **chr == 'u' || **chr == 'i')
        .map(|(pos, _chr)| pos)
        .nth(0)
        .unwrap_or_else(|| chrs.len());

    // Parse the bits themselves
    for idx in n_start..b_pos {
        num *= 10;

        let chr = chrs[idx];

        if chr >= '0' && chr <= '9' {
            num += ((chr as u8) - ('0' as u8)) as u128;
        } else {
            return Err(ParseError::NotAnIntegerNumber(location.clone(), number_string.to_string()));
        }
    }

    if is_negative {
        num = (-(num as i128)) as u128;
    }

    if b_pos < chrs.len() {
        // Parse the number of bits
        for idx in (b_pos+1)..chrs.len() {
            bits *= 10;
            if chrs[idx] >= '0' && chrs[idx] <= '9' {
                bits += ((chrs[idx] as u16) - ('0' as u16)) as u8;
            } else {
                return Err(ParseError::InvalidBitCount(location.clone(), number_string.to_string()));
            }
        }

        if chrs[b_pos] == 'i' {
            let num = if num & (1<<(bits-1)) != 0 {
                let sign_extend = -1i128 << bits;
                num | (sign_extend as u128)
            } else {
                num
            };

            Ok(SafasCell::Number(SafasNumber::SignedBitNumber(bits, num as i128)))
        } else {
            Ok(SafasCell::Number(SafasNumber::BitNumber(bits, num)))
        }
    } else {
        Ok(SafasCell::Number(SafasNumber::Plain(num)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_bit_number() {
        let mut buf         = TokenReadBuffer::new("11110b5".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "(11110b5)")
    }

    #[test]
    fn parse_hex_number_1() {
        let mut buf         = TokenReadBuffer::new("$12ffu16".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "($12ffu16)")
    }

    #[test]
    fn parse_hex_number_2() {
        let mut buf         = TokenReadBuffer::new("$f234i16".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "(-3532i16)")
    }

    #[test]
    fn parse_hex_number_3() {
        let mut buf         = TokenReadBuffer::new("$f0i16".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "(240i16)")
    }

    #[test]
    fn parse_int_number_1() {
        let mut buf         = TokenReadBuffer::new("1234u16".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "($4d2u16)")
    }

    #[test]
    fn parse_int_number_2() {
        let mut buf         = TokenReadBuffer::new("1234i15".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "(1234i15)")
    }

    #[test]
    fn parse_int_number_3() {
        let mut buf         = TokenReadBuffer::new("255i8".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "(-1i8)")
    }

    #[test]
    fn parse_atom() {
        let mut buf         = TokenReadBuffer::new("atom".chars());
        let parse_result    = parse_safas(&mut buf, FileLocation::new("test")).unwrap().to_string();
        assert!(parse_result == "(atom)".to_string());
    }
}
