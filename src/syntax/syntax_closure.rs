use super::syntax_symbol::*;

use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use std::sync::*;
use std::collections::{HashMap};
use std::convert::*;

lazy_static! {
    static ref RETURNS_VALUE_ATOM: u64  = get_id_for_atom_with_name("RETURNS_VALUE");
    static ref RETURNS_MONAD_ATOM: u64  = get_id_for_atom_with_name("RETURNS_MONAD");
}

///
/// Represents a syntax closure, which binds syntax to the environment
/// 
/// This implements the `(some_syntax statements ...)` syntax. It basically creates a new binding
/// environment for its statements and adds in the syntax symbols. It also handles moving syntax
/// between closures by rebinding the symbols if necessary.
///
pub struct SyntaxClosure {
    /// The syntax symbols to import into this closure (as the cells they should be bound to)
    syntax_cells: Vec<(u64, CellRef)>,

    /// The syntax symbols to import into this closure (as the SyntaxSymbols they were derived from)
    syntax_symbols: Vec<(u64, Arc<SyntaxSymbol>)>,

    /// The imported bindings used for the current set of symbols
    imported_bindings: Arc<HashMap<usize, CellRef>>
}

impl SyntaxClosure {
    ///
    /// Creates a syntax closure from a list of syntax symbols and imports
    ///
    pub fn new<SymbolList: IntoIterator<Item=(AtomId, Arc<SyntaxSymbol>)>>(syntax_symbols: SymbolList, imported_bindings: Arc<HashMap<usize, CellRef>>) -> SyntaxClosure {
        // Add the imported bindings into each syntax symbol to generate the syntax symbols list
        let mut bound_symbols   = vec![];
        let mut all_symbols     = vec![];

        for (AtomId(symbol_id), symbol) in syntax_symbols.into_iter() {
            // Set the imported bindings for the symbol
            let mut symbol  = (*symbol).clone();
            symbol.imported_bindings = Arc::clone(&imported_bindings);
            let symbol      = Arc::new(symbol);

            // Turn into syntax that we can add to a binding environment
            let symbol_cell = SafasCell::Syntax(SyntaxSymbol::syntax(symbol.clone()), NIL.clone()).into();

            // Push to the results
            bound_symbols.push((symbol_id, symbol_cell));
            all_symbols.push((symbol_id, symbol));
        }

        // Generate the closure
        SyntaxClosure {
            syntax_cells:       bound_symbols, 
            syntax_symbols:     all_symbols, 
            imported_bindings:  imported_bindings
        }
    }

    ///
    /// Generates the syntax compiler for this closure
    ///
    pub fn syntax(self) -> SyntaxCompiler {
        let generate_actions = |bound_syntax: CellRef| {
            let mut actions = CompiledActions::empty();

            if let SafasCell::List(reference_type, statements) = &*bound_syntax {
                // The reference_type indicates whether or not the statements evaluate to a monad
                let is_monad    = reference_type.to_atom_id() == Some(*RETURNS_MONAD_ATOM);

                // Iterate through the list of statements
                let mut pos     = &**statements;
                let mut first   = true;

                while let SafasCell::List(statement, next) = pos {
                    // Compile this statement
                    actions.extend(compile_statement(statement.clone())?);

                    if is_monad {
                        if statement.reference_type() != ReferenceType::Monad {
                            // All return values need to be wrapped into a monad
                            actions.push(Action::Wrap);
                        }

                        if first {
                            // First instruction pushes the monad value
                            actions.push(Action::Push);
                        } else {
                            // Others just call next to perform the flat_mapping operation
                            actions.push(Action::Next);
                        }
                    }

                    // Move on to the next statement
                    pos     = &*next;
                    first   = false;
                }

                if is_monad && !first {
                    // For a monad value, the result is the monad sitting on the stack
                    actions.push(Action::Pop);
                }
            }

            Ok(actions)
        };

        SyntaxCompiler {
            binding_monad:      Box::new(self),
            generate_actions:   Arc::new(generate_actions)
        }
    }
}

impl BindingMonad for SyntaxClosure {
    type Binding=CellRef;

    fn description(&self) -> String { "##syntax_closure##".to_string() }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        let args = bindings.args.clone().unwrap_or_else(|| NIL.clone());
        (bindings, args)
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        // Get the arguments for this symbol
        let args                    = bindings.args.clone().unwrap_or_else(|| NIL.clone());

        // Push an interior frame
        let mut interior_bindings   = bindings.push_interior_frame();

        // Add the syntax symbols
        for (atom_id, symbol) in self.syntax_cells.iter() {
            interior_bindings.symbols.insert(*atom_id, symbol.clone());
        }

        // The arguments are the statements for these macros: compile them one after the other
        let mut pos                 = &*args;
        let mut bound               = vec![];
        let mut reference_type      = ReferenceType::Value;
        while let SafasCell::List(argument, next) = pos {
            // Bind the argument
            match bind_statement(argument.clone(), interior_bindings) {
                Ok((bound_statement, new_bindings)) => {
                    // Note for later if this returns a monad or a reference
                    if bound_statement.reference_type() == ReferenceType::Monad {
                        reference_type = ReferenceType::Monad;
                    }

                    // Update hte bindings and add the statement
                    interior_bindings = new_bindings;
                    bound.push(bound_statement);
                },

                Err((err, new_bindings)) => {
                    let (bindings, _imports) = new_bindings.pop();
                    return (bindings, Err(err));
                }
            }

            // Move on to the next argument in the list
            pos = &*next;
        }

        let bound                   = SafasCell::list_with_cells(bound);
        let reference_type          = match reference_type { ReferenceType::Monad => SafasCell::Atom(*RETURNS_MONAD_ATOM).into(), _ => SafasCell::Atom(*RETURNS_VALUE_ATOM).into() };
        let bound                   = SafasCell::List(reference_type, bound).into();

        // Finish up: pop the interior bindings and return
        let (bindings, _imports)    = interior_bindings.pop();
        (bindings, Ok(bound))
    }

    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<Box<dyn BindingMonad<Binding=Self::Binding>>>) {
        // Rebind all of the imported bindings, importing the frame reference and the syntax if there are any
        let mut bindings                    = bindings;
        let mut rebound_imported_bindings   = (*self.imported_bindings).clone();
        let mut rebound                     = false;

        for (_cell, binding) in rebound_imported_bindings.iter_mut() {
            match &**binding {
                // Frame references need to be imported into the current frame
                SafasCell::FrameReference(outer_cell_id, bound_level, cell_type) => {
                    // Import this frame reference
                    let local_cell_id   = bindings.alloc_cell();
                    let outer_cell      = SafasCell::FrameReference(*outer_cell_id, *bound_level + frame_depth, *cell_type).into();
                    bindings.import(outer_cell, local_cell_id);

                    // Update the binding
                    *binding            = SafasCell::FrameReference(local_cell_id, 0, *cell_type).into();
                    rebound             = true;
                }

                // Syntax might need to be rebound to the current frame
                SafasCell::Syntax(old_syntax, _) => {
                    // Try to rebind the syntax
                    let (new_bindings, new_syntax) = old_syntax.binding_monad.rebind_from_outer_frame(bindings, frame_depth);

                    // Update the binding if the syntax update
                    if let Some(new_syntax) = new_syntax {
                        let new_syntax  = SyntaxCompiler { binding_monad: new_syntax, generate_actions: old_syntax.generate_actions.clone() };
                        *binding        = SafasCell::Syntax(new_syntax, NIL.clone()).into();
                        rebound         = true;
                    }

                    // Update the bindings from the result
                    bindings = new_bindings;
                }

                // Other types are not affected by rebinding
                _ => { }
            }
        }

        // If no bindings were updated, just keep using the same syntax as before
        if !rebound {
            return (bindings, None);
        }

        // Regenerate the syntax symbols with the new imported bindings
        let rebound_imported_bindings   = Arc::new(rebound_imported_bindings);
        let new_syntax                  = self.syntax_symbols.iter()
            .map(|(atom_id, symbol)| {
                let patterns    = symbol.patterns.clone();
                let new_symbol  = SyntaxSymbol {
                    patterns:           patterns, 
                    imported_bindings:  Arc::clone(&rebound_imported_bindings),
                    reference_type:     symbol.reference_type
                };

                (AtomId(*atom_id), Arc::new(new_symbol))
            })
            .collect::<Vec<_>>();

        // Create a new syntax closure with these symbols
        let new_syntax_closure = SyntaxClosure::new(new_syntax, rebound_imported_bindings);

        (bindings, Some(Box::new(new_syntax_closure)))
    }
}
