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

            // Macros can reference each other. Only back-references are allowed so we can bind them properly
            // Initially all symbols generate errors
            for (symbol_id, _) in macros.iter() {
                // Symbols are intially bound to some syntax that generates an error
                let error = SyntaxCompiler { binding_monad: Box::new(BindingFn(|bindings| (bindings, Err(BindError::ForwardReferencesNotAllowed)))), generate_actions: Box::new(|_| Err(BindError::ForwardReferencesNotAllowed)) };
                evaluation_bindings.symbols.insert(*symbol_id, SafasCell::ActionMonad(error).into());
            }

            for (symbol_id, symbol_patterns) in macros.iter() {
                // bound_patterns will store the patterns that will be bound by this syntax
                let mut bound_patterns          = vec![];

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
                    let (macro_bindings, bind_result) = match bind_result { 
                        Ok((result, bindings))  => ((bindings, result)), 
                        Err((err, bindings))    => {
                            return (bindings.pop().0.pop().0, Err(err));
                        }
                    };

                    // Store in the results
                    bound_patterns.push((pattern_def, pattern_cells, bind_result));

                    // Revert the inner frame
                    let (new_bindings, _)       = macro_bindings.pop();
                    evaluation_bindings         = new_bindings;
                }

                // Create a syntax symbol
                let symbol = SyntaxSymbol::new(bound_patterns);

                // Define this as our symbol name
                evaluation_bindings.symbols.insert(*symbol_id, SafasCell::ActionMonad(symbol.syntax()).into());
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
/// The syntax symbol struct evaluates a single syntax symbol
///
struct SyntaxSymbol {
    /// The patterns, their frame bindings and the partially-bound macro
    patterns: Vec<(Arc<PatternMatch>, Vec<CellRef>, CellRef)>,

    /// The bindings that were imported from outside of this symbol
    imported_bindings: Arc<HashMap<usize, CellRef>>
}

impl SyntaxSymbol {
    ///
    /// Creates a new syntax symbol that will match one of the specified patterns
    ///
    pub fn new(patterns: Vec<(Arc<PatternMatch>, Vec<CellRef>, CellRef)>) -> SyntaxSymbol {
        // TODO : we currently initialize the imported bindings to nothing, expecting to fill them in later but this has the
        // issue that when using a macro from within another macro, it won't work properly
        SyntaxSymbol { patterns: patterns, imported_bindings: Arc::new(HashMap::new()) }
    }

    ///
    /// Creates the syntax compiler for this symbol
    ///
    pub fn syntax(self) -> SyntaxCompiler {
        SyntaxCompiler {
            binding_monad:      Box::new(self),
            generate_actions:   Box::new(|_| unimplemented!())
        }
    }
}

impl BindingMonad for SyntaxSymbol {
    type Binding=Result<CellRef, BindError>;

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Get the arguments for this symbol
        let args            = bindings.args.clone().unwrap_or_else(|| SafasCell::Nil.into());
        let mut bindings    = bindings;

        // Try to match them against each pattern
        for (pattern_match, pattern_cells, partially_bound) in self.patterns.iter() {
            if let Ok(pattern) = pattern_match.match_against(&args) {

                // Substitute the arguments into the pattern
                // 
                // Every value in the macro will refer to the 'fake' macro frame so will be a FrameReference(foo, 0). We
                // substitute these for the actual values.
                // 
                // Some values will be imported from outside the macro (we can find these in imported_bindings), and some
                // will be bound by the pattern. We start by finding the pattern that matches the arguments and then
                // binding those statements.
                // 
                // Some values will defined within the macro; these are left unbound after the binding has completed and
                // we assign new cells to them after binding everything else

                let mut substitutions = HashMap::new();

                for arg_idx in 0..pattern_cells.len() {
                    // The pattern cell is expected to always be a frame reference
                    let FrameReference(cell_id, _) = pattern_cells[arg_idx].clone().try_into().unwrap();

                    // Bind the value in this argument
                    let bound_val = match &pattern[arg_idx] {
                        MatchBinding::Statement(_atom_id, statement_val)    => bind_statement(statement_val.clone(), bindings),
                        MatchBinding::Symbol(_atom_id, symbol_val)          => Ok((symbol_val.clone(), bindings)),
                    };

                    // Check for errors
                    let (bound_val, new_bindings) = match bound_val {
                        Ok((bound_val, bindings))   => (bound_val, bindings),
                        Err((err, bindings))        => return (bindings, Err(err))
                    };
                    bindings = new_bindings;

                    // Store as a substitution
                    substitutions.insert(cell_id, bound_val);
                }

                // Perform the substititions
                let (bound, bindings) = substitute_cells(bindings, partially_bound, &move |cell_id| {
                    substitutions.get(&cell_id)
                        .or_else(|| self.imported_bindings.get(&cell_id))
                        .cloned()
                });

                // This is the result
                return (bindings, Ok(bound));
            }
        }

        // No matching pattern
        (bindings, Err(BindError::SyntaxMatchFailed))
    }
}

///
/// Substitutes any FrameReferences in the partially bound statement for bound values, and rebinds any FrameReferences that are
/// not currently bound
///
fn substitute_cells<SubstituteFn: Fn(usize) -> Option<CellRef>>(bindings: SymbolBindings, partially_bound: &CellRef, substitutions: &SubstituteFn) -> (CellRef, SymbolBindings) {
    // Bind the cells
    let pos = partially_bound;

    match &**pos {
        // Lists are bound recursively
        SafasCell::List(car, cdr) => {
            // TODO: would be more efficient to bind in a loop
            let (car, bindings) = substitute_cells(bindings, car, substitutions);
            let (cdr, bindings) = substitute_cells(bindings, cdr, substitutions);

            (SafasCell::List(car, cdr).into(), bindings)
        }

        // Frame references are bound by the substitution function
        SafasCell::FrameReference(cell_id, frame) => {
            if *frame == 0 {
                // Is from the macro frame: bind via the subtitutions function
                if let Some(actual_cell) = substitutions(*cell_id) {
                    (actual_cell, bindings)
                } else {
                    // TODO
                    unimplemented!("Need to be able to bind unbound cells in macros")
                }
            } else {
                // Bound from a different frame
                (pos.clone(), bindings)
            }
        }

        // Other cell types have no binding to do
        _ => (pos.clone(), bindings)
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