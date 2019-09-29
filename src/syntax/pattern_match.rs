use crate::meta::*;
use crate::bind::*;

use smallvec::*;
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
    SymbolBinding(u64),

    /// Matches the end of input symbol
    EndOfInput
}

///
/// Bindings generated by a symbol match
///
pub enum MatchBinding {
    /// Specified atom should be bound to the result of the specified statement
    Statement(u64, CellRef),

    /// Specified atom should be bound to the specified absolute symbol value
    Symbol(u64, CellRef)
}

impl MatchBinding {
    ///
    /// Retrieves the cell that this value is bound to
    ///
    pub fn bound_cell(&self) -> CellRef {
        match self {
            MatchBinding::Statement(_, cell)    => Arc::clone(cell),
            MatchBinding::Symbol(_, cell)       => Arc::clone(cell)
        }
    }
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
    /// Returns the atom IDs that will be bound by this pattern (in the order that they will appear in the result)
    ///
    pub fn bindings(&self) -> Vec<AtomId> {
        self.symbols.iter()
            .filter_map(|symbol| {
                match symbol {
                    MatchSymbol::StatementBinding(atom_id)  => Some(AtomId(*atom_id)),
                    MatchSymbol::SymbolBinding(atom_id)     => Some(AtomId(*atom_id)),
                    _                                       => None
                }
            })
            .collect()
    }

    ///
    /// Creates a pattern matcher from a list of SafasCells
    ///
    pub fn from_pattern_as_cells(list: CellRef) -> Result<PatternMatch, BindError> {
        // Set up to iterate through the list and generate the list of symbols
        let mut list_pos    = &*list;
        let mut symbols     = vec![];

        // Need some specific atoms for some parts of the parsing
        let angle_open      = get_id_for_atom_with_name("<");
        let angle_close     = get_id_for_atom_with_name(">");
        let curly_open      = get_id_for_atom_with_name("{");
        let curly_close     = get_id_for_atom_with_name("}");

        // Iterate through the list
        while let SafasCell::List(car, cdr) = list_pos {
            // Action depends on car
            match &**car {
                SafasCell::Atom(atom_id) => {
                    let atom_id = *atom_id;
                    if atom_id == angle_open {

                        // <foo> = bind statement to 'foo' - this is a bit annoying to parse
                        list_pos = cdr;

                        if let SafasCell::List(bind_atom, cdr) = list_pos {
                            if let SafasCell::Atom(bind_atom) = &**bind_atom {
                                symbols.push(MatchSymbol::StatementBinding(*bind_atom));
                                list_pos = &*cdr;
                            } else {
                                return Err(BindError::SyntaxExpectingAtom)
                            }
                        } else {
                            return Err(BindError::SyntaxExpectingAtom)
                        }

                        // Should be '>' (tedious to parse this out :-/)
                        if let SafasCell::List(close_bracket, cdr) = list_pos {
                            if let SafasCell::Atom(atom) = &**close_bracket {
                                if *atom != angle_close {
                                    return Err(BindError::SyntaxMissingBracket('>'));
                                }

                                // Continue after the '>'
                                list_pos = &*cdr;
                                continue;
                            } else {
                                return Err(BindError::SyntaxMissingBracket('>'));
                            }
                        } else {
                            return Err(BindError::SyntaxMissingBracket('>'));
                        }

                    } else if atom_id == curly_open {

                        // {foo} = bind whatever appears to 'foo' - same parsing mechanism as for <foo> except different brackets
                        list_pos = &*cdr;

                        if let SafasCell::List(bind_atom, cdr) = list_pos {
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
                        if let SafasCell::List(close_bracket, cdr) = list_pos {
                            if let SafasCell::Atom(atom) = &**close_bracket {
                                if *atom != curly_close {
                                    return Err(BindError::SyntaxMissingBracket('}'));
                                }

                                // Continue after the '}'
                                list_pos = &*cdr;
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

                SafasCell::Any(_) | SafasCell::FrameMonad(_) | SafasCell::ActionMonad(_) | SafasCell::FrameReference(_, _) => { return Err(BindError::NotValidInSyntax) }
            }

            // Move to the next cell
            list_pos = cdr;
        }

        Ok(Self::new(symbols))
    }

    ///
    /// Attempts to match this pattern against some input, returning the bindings if it matches
    ///
    pub fn match_against(&self, input: &CellRef) -> Result<Vec<MatchBinding>, BindError> {
        Self::match_with_symbols(&self.symbols, input)
    }

    ///
    /// Matches a single symbol, returning the bindings and the next item in the list if the match was successful
    ///
    pub fn match_symbol<'a>(symbol: &MatchSymbol, list_item: &'a SafasCell) -> Result<(SmallVec<[MatchBinding; 1]>, &'a SafasCell), BindError> {
        if let (MatchSymbol::Nil, SafasCell::Nil) = (symbol, list_item) {
            // EndOfInput matches if the list is nil
            Ok((smallvec![], list_item))
        } else if let SafasCell::List(car, cdr) = list_item {
            // Match car against the symbol
            let mut bindings = smallvec![];

            use self::MatchSymbol::*;
            match symbol {
                Atom(atom_id)               => { if car.to_atom_id() != Some(*atom_id)              { return Err(BindError::SyntaxMatchFailed); } },
                Nil                         => { if !car.is_nil()                                   { return Err(BindError::SyntaxMatchFailed); } },
                String(string)              => { if car.string_value().as_ref() != Some(string)     { return Err(BindError::SyntaxMatchFailed); } },
                Char(chr)                   => { if car.char_value() != Some(*chr)                  { return Err(BindError::SyntaxMatchFailed); } },
                Number(number)              => { if car.number_value() != Some(*number)             { return Err(BindError::SyntaxMatchFailed); } },
                EndOfInput                  => { /* Matched implicitly */ },

                StatementBinding(atom_id)   => { bindings.push(MatchBinding::Statement(*atom_id, Arc::clone(car))); }
                SymbolBinding(atom_id)      => { bindings.push(MatchBinding::Symbol(*atom_id, Arc::clone(car))); }

                List(list_pattern)  => {
                    let list_bindings = Self::match_with_symbols(list_pattern, &Arc::clone(car))?;
                    bindings.extend(list_bindings);
                }
            }

            Ok((bindings, &**cdr))
        } else {
            // List item is not a list
            Err(BindError::SyntaxMatchFailed)
        }
    }

    ///
    /// Performs matching directly against a symbol list
    ///
    fn match_with_symbols(symbols: &Vec<MatchSymbol>, input: &CellRef) -> Result<Vec<MatchBinding>, BindError> {
        // Current position in the input
        let mut input_pos   = &**input;
        let mut bindings    = vec![];

        // Match the input position against the expected symbol
        for symbol in symbols.iter() {
            // Match the next symbol
            let (new_bindings, new_pos) = Self::match_symbol(symbol, input_pos)?;

            // Store the bindings
            bindings.extend(new_bindings);

            // Advance the input position
            input_pos = new_pos;
        }

        if let SafasCell::Nil = input_pos {
            // Reached the end of the input: match succeded
            Ok(bindings)
        } else {
            // Only matched a prefix of what was expected
            Err(BindError::SyntaxMatchedPrefix)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::interactive::*;

    #[test]
    fn pattern_match_lda_example() {
        let pattern         = eval("(quote (lda #<val>))").unwrap().0;
        let matcher         = PatternMatch::from_pattern_as_cells(pattern).unwrap();
        let match_against   = eval("(quote (lda #10))").unwrap().0;

        let bindings        = matcher.match_against(&match_against).unwrap();
        assert!(bindings.len() == 1);

        if let MatchBinding::Statement(atom_id, val) = &bindings[0] {
            assert!(*atom_id == get_id_for_atom_with_name("val"));
            assert!(val.number_value() == Some(SafasNumber::Plain(10)));
        } else {
            assert!(false)
        }
    }

    #[test]
    fn pattern_match_with_list() {
        let pattern         = eval("(quote (lda #<val>, (X)))").unwrap().0;
        let matcher         = PatternMatch::from_pattern_as_cells(pattern).unwrap();
        let match_against   = eval("(quote (lda #10, (X)))").unwrap().0;

        let bindings        = matcher.match_against(&match_against).unwrap();
        assert!(bindings.len() == 1);

        if let MatchBinding::Statement(atom_id, val) = &bindings[0] {
            assert!(*atom_id == get_id_for_atom_with_name("val"));
            assert!(val.number_value() == Some(SafasNumber::Plain(10)));
        } else {
            assert!(false)
        }
    }

    #[test]
    fn pattern_match_binding_in_list() {
        let pattern         = eval("(quote (lda (#<val>)))").unwrap().0;
        let matcher         = PatternMatch::from_pattern_as_cells(pattern).unwrap();
        let match_against   = eval("(quote (lda (#10)))").unwrap().0;

        let bindings        = matcher.match_against(&match_against).unwrap();
        assert!(bindings.len() == 1);

        if let MatchBinding::Statement(atom_id, val) = &bindings[0] {
            assert!(*atom_id == get_id_for_atom_with_name("val"));
            assert!(val.number_value() == Some(SafasNumber::Plain(10)));
        } else {
            assert!(false)
        }
    }

    #[test]
    fn pattern_match_error_1() {
        let pattern         = eval("(quote (lda <val>))").unwrap().0;
        let matcher         = PatternMatch::from_pattern_as_cells(pattern).unwrap();
        let match_against   = eval("(quote (lda #10))").unwrap().0;

        let bindings        = matcher.match_against(&match_against);
        assert!(bindings.is_err());
        assert!(if let Err(BindError::SyntaxMatchedPrefix) = bindings { true } else { false });
    }

    #[test]
    fn pattern_match_error_2() {
        let pattern         = eval("(quote (lda #<val>))").unwrap().0;
        let matcher         = PatternMatch::from_pattern_as_cells(pattern).unwrap();
        let match_against   = eval("(quote (lda 10))").unwrap().0;

        let bindings        = matcher.match_against(&match_against);
        assert!(bindings.is_err());
        assert!(if let Err(BindError::SyntaxMatchFailed) = bindings { true } else { false });
    }
}
