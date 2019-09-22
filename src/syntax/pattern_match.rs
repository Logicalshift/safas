use crate::meta::*;
use crate::bind::*;

use std::sync::*;
use std::result::{Result};

///
/// A symbol to be matched in a pattern
///
pub enum MatchSymbol {
    /// Match an atom
    Atom(u64),

    /// Match 'nil'
    Nil,

    /// Match a string
    String(String),

    /// Match a character
    Char(char),

    /// Match a specific number
    Number(SafasNumber),

    /// Match a list of values
    List(Vec<MatchSymbol>),

    /// Matches a statement and binds its result to an atom
    StatementBinding(u64),

    /// Matches a symbol without evaluating it and binds it to an atom
    SymbolBinding(u64)
}

///
/// Describes a pattern that can be matched against a cell
///
pub struct PatternMatch {
    /// The symbols to match against
    symbols: Vec<MatchSymbol>
}

impl PatternMatch {
    ///
    /// Creates a pattern matcher that will match against the specified symbols
    ///
    pub fn new(symbols: Vec<MatchSymbol>) -> PatternMatch {
        PatternMatch {
            symbols
        }
    }

    ///
    /// Creates a pattern matcher from a list of SafasCells
    ///
    pub fn from_pattern_as_cells(list: Arc<SafasCell>) -> Result<PatternMatch, BindError> {
        // Set up to iterate through the list and generate the list of symbols
        let mut list_pos    = &list;
        let mut symbols     = vec![];

        // Need some specific atoms for some parts of the parsing
        let angle_open      = get_id_for_atom_with_name("<");
        let angle_close     = get_id_for_atom_with_name(">");
        let curly_open      = get_id_for_atom_with_name("{");
        let curly_close     = get_id_for_atom_with_name("}");

        // Iterate through the list
        while let SafasCell::List(car, cdr) = &**list_pos {
            // Action depends on car
            match &**car {
                SafasCell::Atom(atom_id) => {
                    let atom_id = *atom_id;
                    if atom_id == angle_open {

                        // <foo> = bind statement to 'foo' - this is a bit annoying to parse
                        list_pos = cdr;

                        if let SafasCell::List(bind_atom, cdr) = &**list_pos {
                            if let SafasCell::Atom(bind_atom) = &**bind_atom {
                                symbols.push(MatchSymbol::StatementBinding(*bind_atom));
                                list_pos = cdr;
                            } else {
                                return Err(BindError::SyntaxExpectingAtom)
                            }
                        } else {
                            return Err(BindError::SyntaxExpectingAtom)
                        }

                        // Should be '>' (tedious to parse this out :-/)
                        if let SafasCell::List(close_bracket, cdr) = &**list_pos {
                            if let SafasCell::Atom(atom) = &**close_bracket {
                                if *atom != angle_close {
                                    return Err(BindError::SyntaxMissingBracket('>'));
                                }

                                // Continue after the '>'
                                list_pos = cdr;
                                continue;
                            } else {
                                return Err(BindError::SyntaxMissingBracket('>'));
                            }
                        } else {
                            return Err(BindError::SyntaxMissingBracket('>'));
                        }

                    } else if atom_id == curly_open {

                        // {foo} = bind whatever appears to 'foo' - same parsing mechanism as for <foo> except different brackets
                        list_pos = cdr;

                        if let SafasCell::List(bind_atom, cdr) = &**list_pos {
                            if let SafasCell::Atom(bind_atom) = &**bind_atom {
                                symbols.push(MatchSymbol::SymbolBinding(*bind_atom));
                                list_pos = cdr;
                            } else {
                                return Err(BindError::SyntaxExpectingAtom)
                            }
                        } else {
                            return Err(BindError::SyntaxExpectingAtom)
                        }

                        // Should be '}'
                        if let SafasCell::List(close_bracket, cdr) = &**list_pos {
                            if let SafasCell::Atom(atom) = &**close_bracket {
                                if *atom != curly_close {
                                    return Err(BindError::SyntaxMissingBracket('}'));
                                }

                                // Continue after the '}'
                                list_pos = cdr;
                                continue;
                            } else {
                                return Err(BindError::SyntaxMissingBracket('}'));
                            }
                        } else {
                            return Err(BindError::SyntaxMissingBracket('}'));
                        }

                    } else {
                        // Bind straight to the atom (<< and {{ are mapped to < and {)
                        let mut atom_name = name_for_atom_with_id(atom_id);
                        
                        // Strip extra '<' or '{'
                        let atom_id = if atom_name.chars().nth(0) == Some('<') {
                            atom_name.remove(0);
                            get_id_for_atom_with_name(&atom_name)
                        } else if atom_name.chars().nth(0) == Some('{') {
                            atom_name.remove(0);
                            get_id_for_atom_with_name(&atom_name)
                        } else {
                            atom_id
                        };

                        // Match a symbol directly
                        symbols.push(MatchSymbol::Atom(atom_id));
                    }
                }

                SafasCell::List(_, _) => { 
                    // Generate a pattern from the list and push it into the pattern we're building
                    let list_pattern = Self::from_pattern_as_cells(Arc::clone(car))?;
                    symbols.push(MatchSymbol::List(list_pattern.symbols));
                }

                SafasCell::Nil              => { symbols.push(MatchSymbol::Nil); }
                SafasCell::Number(number)   => { symbols.push(MatchSymbol::Number(number.clone())); }
                SafasCell::String(string)   => { symbols.push(MatchSymbol::String(string.clone())); }
                SafasCell::Char(chr)        => { symbols.push(MatchSymbol::Char(*chr)); }

                SafasCell::Monad(_) | SafasCell::MacroMonad(_) | SafasCell::ActionMonad(_) => { return Err(BindError::NotValidInSyntax) }
            }

            // Move to the next cell
            list_pos = cdr;
        }

        Ok(Self::new(symbols))
    }
}
