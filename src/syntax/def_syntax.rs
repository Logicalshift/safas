use super::pattern_match::*;

use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use itertools::*;
use smallvec::*;
use std::sync::*;
use std::collections::{HashMap};

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
    pub fn new() -> SyntaxCompiler {
        unimplemented!()
    }
}

impl BindingMonad for DefSyntaxKeyword {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##def##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Fetch the arguments
        let args = bindings.args.clone().unwrap_or_else(|| SafasCell::Nil.into());

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
            let setup: CellRef = if let SafasCell::List(car, cdr) = args {
                SafasCell::List(Arc::clone(car), Arc::clone(cdr)).into()
            } else {
                SafasCell::Nil.into()
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

                // Group by symbol
                let matchers = matchers.into_iter()
                    .group_by(|(symbol, _matcher, _macro_def)| *symbol);

                // Generate the evaluators for each symbol
                let mut bindings    = bindings.push_new_frame();
                let mut syntax      = HashMap::new();

                for (symbol, syntax_symbol) in matchers.into_iter() {
                    // Create a syntax symbol for this item
                    let patterns                        = syntax_symbol.map(|(_symbol, matcher, macro_def)| (matcher, macro_def));
                    let (new_bindings, syntax_symbol)   = SyntaxSymbol::new(bindings, patterns.collect());
                    bindings = new_bindings;

                    // Add to the symbols
                    syntax.insert(symbol, syntax_symbol);
                }

                // Generate the syntax item
                let syntax          = Syntax::new(syntax);

                // Pop the frame we added for the syntax. import_values indicates what we need to bind to our syntax
                let (bindings, import_values) = bindings.pop();

                // TODO: syntax and syntaxsymbol need to be bindings
                // TODO: we need to bind atom values when creating syntax symbols
                (bindings, Ok(smallvec![]))
            },

            Err(err) => {
                // Invalid syntax
                (bindings, Err(err))
            }
        }
    }
}

///
/// The syntax struct creates the keyword that evaluates a particular syntax
///
struct Syntax {
    /// The extra keywords added by this syntax
    syntax: HashMap<u64, SyntaxSymbol> 
}

impl Syntax {
    ///
    /// Creates a new syntax keyword
    ///
    pub fn new(syntax: HashMap<u64, SyntaxSymbol>) -> Syntax {
        Syntax { syntax }
    }
}

///
/// The syntax symbol struct evaluates a single syntax symbol
///
struct SyntaxSymbol {
    /// The patterns that can be matched against this symbol (and their macro binding)
    patterns: Vec<(PatternMatch, CellRef)>
}

impl SyntaxSymbol {
    ///
    /// Creates a new syntax symbol that will match one of the specified patterns
    ///
    pub fn new(bindings: SymbolBindings, patterns: Vec<(PatternMatch, CellRef)>) -> (SymbolBindings, SyntaxSymbol) {
        (bindings, SyntaxSymbol { patterns: patterns })
    }
}