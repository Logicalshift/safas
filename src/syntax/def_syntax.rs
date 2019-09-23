use super::pattern_match::*;

use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;

///
/// The (def_syntax) keyword, expressed as a binding monad
/// 
/// Syntax is defined using:
/// 
/// ```(def_syntax <name> (<pattern> <macro> ...) [prelude_statements])```
/// 
/// <name> becomes a syntax item in the binding. We can use the new syntax like this:
/// 
/// ```(<name> <statements>)```
///
pub struct DefSyntaxKeyword {
}

impl DefSyntaxKeyword {
    ///
    /// Creates the def_syntax keyword
    ///
    pub fn new() -> DefSyntaxKeyword {
        DefSyntaxKeyword { }
    }
}

impl BindingMonad for DefSyntaxKeyword {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##def##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Fetch the arguments
        let args = bindings.args.clone().unwrap_or_else(|| Arc::new(SafasCell::Nil));

        // Parse the patterns
        let patterns: Result<_, BindError> = (|| {
            let args = &*args;

            // First symbol should be the name
            let (name, args)    = PatternMatch::match_symbol(&MatchSymbol::SymbolBinding(0), args)?;
            let name            = name.first().map(|binding| binding.bound_cell()).and_then(|cell| cell.to_atom_id()).ok_or(BindError::SyntaxExpectingAtom)?;

            // The next item is the syntax list
            let (syntax, args) = PatternMatch::match_symbol(&MatchSymbol::SymbolBinding(0), args)?;
            let syntax          = syntax.first().map(|binding| binding.bound_cell()).ok_or(BindError::SyntaxExpectingList)?;

            // Parse the syntax
            let mut patterns    = vec![];
            let mut syntax      = &*syntax;

            while !syntax.is_nil() {
                // Two items: the pattern and the macro
                let (pattern, new_syntax)   = PatternMatch::match_symbol(&MatchSymbol::SymbolBinding(0), syntax)?;
                let (macro_def, new_syntax) = PatternMatch::match_symbol(&MatchSymbol::SymbolBinding(0), new_syntax)?;
                syntax                      = new_syntax;

                // Fetch the bound cell for the pattern and the macro
                let pattern                 = pattern.first().map(|pattern| pattern.bound_cell()).ok_or(BindError::SyntaxExpectingList)?;
                let macro_def               = macro_def.first().map(|macro_def| macro_def.bound_cell()).ok_or(BindError::SyntaxExpectingList)?;

                // The pattern should begin with an atom, indicating the symbol that should be matched
                let (symbol, pattern)       = pattern.list_value().ok_or(BindError::SyntaxExpectingList)?;
                let symbol                  = symbol.to_atom_id().ok_or(BindError::SyntaxExpectingAtom)?;

                patterns.push((symbol, pattern, macro_def));
            }

            // After this we get the environment set up statements
            let setup = if let SafasCell::List(car, cdr) = args {
                Arc::new(SafasCell::List(Arc::clone(car), Arc::clone(cdr)))
            } else {
                Arc::new(SafasCell::Nil)
            };

            Ok((name, patterns, setup))
        })();

        match patterns {
            Ok((name_atom_id, patterns, setup)) => {
                // Turn the patterns into pattern matchers
                let matchers = patterns.into_iter()
                    .map(|(symbol, pattern, macro_def)| PatternMatch::from_pattern_as_cells(pattern).map(move |matcher| (symbol, matcher, macro_def)))
                    .collect::<Result<Vec<_>, _>>();
                let matchers = match matchers { Ok(matchers) => matchers, Err(err) => return (bindings, Err(err)) };

                // TODO
                (bindings, Ok(smallvec![]))
            },

            Err(err) => {
                // Invalid syntax
                (bindings, Err(err))
            }
        }
    }
}