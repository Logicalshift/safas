use super::pattern_match::*;

use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use itertools::*;
use smallvec::*;
use std::sync::*;
use std::collections::{HashMap};
use std::convert::*;

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
pub fn def_syntax_keyword() -> SyntaxCompiler {
    let bind = get_expression_arguments().map(|args: Result<ListWithTail<(AtomId, CellRef), CellRef>, BindError>| {

        // First step: parse the arguments to the expression

        // Fetch the arguments
        let ListWithTail((name, patterns), statements) = args?;

        // Process the patterns (each is of the form <pattern> <macro>)
        let mut current_pattern = patterns;
        let mut macros          = vec![];
        while !current_pattern.is_nil() {
            // Each pattern is two cells, the pattern definition and the macro definition
            // Format is `(<symbol> . <pattern>) <macro>`
            let pattern_def: ListWithTail<(ListWithTail<(AtomId, ), CellRef>, CellRef), CellRef>    = ListWithTail::try_from(current_pattern)?;
            let ListWithTail((ListWithTail((symbol_name, ), pattern_def), macro_def), next_pattern) = pattern_def;

            // Compile the pattern
            let pattern_def = PatternMatch::from_pattern_as_cells(pattern_def)?;

            // Add to the macros
            macros.push((symbol_name, pattern_def, macro_def));

            // Move to the next pattern
            current_pattern = next_pattern;
        }

        // Group by symbol, so we a vec of each symbol we want to match and the corresponding macro definition
        let macros = macros.into_iter().group_by(|(AtomId(symbol_name), _pattern_def, _macro_def)| *symbol_name);
        let macros = macros.into_iter()
            .map(|(symbol, values)| {
                let values = values.into_iter().map(|(_symbol, pattern_def, macro_def)| (Arc::new(pattern_def), macro_def));
                (symbol, values.collect::<Vec<_>>())
            })
            .collect::<Vec<_>>();

        // Result of the first stage is the list of patterns
        Ok((name, Arc::new(macros), statements))

    }).and_then_ok(|args| {

        // Second step: bind each of the macros and generate the syntax item

        BindingFn(move |bindings| {

            // Fetch the values computed by the previous step
            let (name, macros, statements)  = &args;

            // Bind the macros in an inner frame
            let mut evaluation_bindings     = bindings.push_new_frame();

            // TODO: create the bindings for the symbols so macros can reference each other

            for (symbol, symbol_patterns) in macros.iter() {
                let mut bound_patterns = vec![];

                for (pattern_def, macro_def) in symbol_patterns.iter() {
                    let pattern_def             = Arc::clone(pattern_def);
                    let macro_def               = Arc::clone(macro_def);

                    // Create an inner frame with the values for this macro
                    let mut macro_bindings      = evaluation_bindings.push_interior_frame();

                    // Bind the arguments for the pattern
                    let mut pattern_cells = vec![];
                    for AtomId(arg_atom_id) in pattern_def.bindings() {
                        // Create a new cell for this atom
                        let arg_cell            = macro_bindings.alloc_cell();
                        let arg_cell: CellRef   = SafasCell::FrameReference(arg_cell, 0).into();

                        // Add to the bindings and the list of cells for this pattern
                        macro_bindings.symbols.insert(arg_atom_id, arg_cell.clone());
                        pattern_cells.push(arg_cell);
                    }
                    
                    // Bind the macro definition
                    let bind_result             = bind_statement(macro_def, macro_bindings);
                    let (macro_bindings, bind_result) = match bind_result { Ok((result, bindings)) => ((bindings, Ok(result))), Err((err, bindings)) => (bindings, Err(err)) };

                    // Store in the results
                    bound_patterns.push(bind_result.map(move |bound| (pattern_def, pattern_cells, bound)));

                    // Revert the inner frame
                    let (new_bindings, _)       = macro_bindings.pop();
                    evaluation_bindings         = new_bindings;
                }
            }

            // Pop the evaluation frame
            let (bindings, imports) = evaluation_bindings.pop();

            (bindings, Ok(SafasCell::Nil.into()))

        })
    });

    let compile = |args: CellRef| {
        Ok(smallvec![])
    };

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Box::new(compile)
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

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn evaluate_def_syntax() {
        eval("(def_syntax x ((lda #<x>) (d x)))").unwrap().0.to_string();
    }
}