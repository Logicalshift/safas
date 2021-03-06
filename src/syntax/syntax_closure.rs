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
    /// Optionally, a cell containing some existing syntax that this closure will extend (None if this defines all-new syntax)
    /// 
    /// This should contain a syntax cell with a BTree stored in the parameters. To extend the syntax,
    /// the closure will look up the 'syntax' key in the BTree. The value should be another BTree: any
    /// key/value pairs where the key is an atom will be imported into the new syntax.
    /// 
    /// (Syntaxes defined by def_syntax have this format)
    extend_syntax: Option<CellRef>,

    /// The syntax symbols to import into this closure (as the cells they should be bound to)
    syntax_cells: Vec<(u64, CellRef)>,

    /// The syntax symbols to import into this closure (as the SyntaxSymbols they were derived from)
    syntax_symbols: Vec<(u64, Arc<SyntaxSymbol>)>,

    /// The value of the special 'syntax' symbol that can be used to retrieve the symbols defined for this syntax
    syntax_btree: CellRef,

    /// The imported bindings used for the current set of symbols
    imported_bindings: Arc<HashMap<usize, CellRef>>
}

impl SyntaxClosure {
    ///
    /// Creates a syntax closure from a list of syntax symbols and imports
    ///
    pub fn new<SymbolList: IntoIterator<Item=(AtomId, Arc<SyntaxSymbol>)>>(syntax_symbols: SymbolList, imported_bindings: Arc<HashMap<usize, CellRef>>, extend_syntax: Option<CellRef>) -> SyntaxClosure {
        // Add the imported bindings into each syntax symbol to generate the syntax symbols list
        let mut bound_symbols: Vec<(u64, CellRef)>   = vec![];
        let mut all_symbols     = vec![];

        for (AtomId(symbol_id), symbol) in syntax_symbols.into_iter() {
            // Set the imported bindings for the symbol
            let mut symbol  = (*symbol).clone();
            symbol.imported_bindings = Arc::clone(&imported_bindings);
            let symbol      = Arc::new(symbol);

            // Turn into syntax that we can add to a binding environment
            let symbol_cell = SafasCell::Syntax(Box::new(SyntaxSymbol::syntax(symbol.clone())), NIL.clone()).into();

            // Push to the results
            bound_symbols.push((symbol_id, symbol_cell));
            all_symbols.push((symbol_id, symbol));
        }

        // Generate the syntax b-tree (starting at the syntax we're extending, if there is one)
        let mut syntax_btree = extend_syntax.as_ref()
            .and_then(|extend_syntax| match &**extend_syntax { SafasCell::Syntax(_, params) => Some(params.clone()), _ => None })
            .and_then(|params| match btree_search(params, SafasCell::atom("syntax")) { Ok(extended_syntax) => Some(extended_syntax), Err(_err) => None })
            .and_then(|extended_syntax| if extended_syntax.is_btree() { Some(extended_syntax) } else { None })
            .unwrap_or_else(|| btree_new());
        for (symbol_id, value) in bound_symbols.iter() {
            syntax_btree = btree_insert(syntax_btree, (SafasCell::Atom(*symbol_id).into(), value.clone())).expect("Valid BTree");
        }

        // Generate the closure
        SyntaxClosure {
            extend_syntax:      extend_syntax,
            syntax_cells:       bound_symbols, 
            syntax_symbols:     all_symbols, 
            syntax_btree:       syntax_btree,
            imported_bindings:  imported_bindings
        }
    }

    ///
    /// Generates the syntax compiler for this closure
    ///
    pub fn syntax(self) -> impl BindingMonad<Binding=SyntaxCompiler> {
        self.map(|bound_syntax| {
            let bound_syntax    = bound_syntax.clone();
            let is_monad        = if let SafasCell::List(reference_type, statements) = &*bound_syntax {
                // The reference_type indicates whether or not the statements evaluate to a monad
                reference_type.to_atom_id() == Some(*RETURNS_MONAD_ATOM)
            } else { 
                false
            };

            let generate_actions = move |bound_syntax: CellRef| {
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

            let reference_type = if is_monad { ReferenceType::Monad } else { ReferenceType::Value };
            SyntaxCompiler::with_compiler_and_reftype(generate_actions, bound_syntax, reference_type)
        })
    }

    ///
    /// Retrieves the syntax b-tree for this closure
    ///
    pub fn syntax_btree(&self) -> CellRef {
        self.syntax_btree.clone()
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
        if let Some(extend_syntax) = self.extend_syntax.as_ref() {
            // The syntax to extend should be a syntax item with a 'syntax' key defined in its parameters
            if let SafasCell::Syntax(_, extend_parameters) = &**extend_syntax {
                if let Ok(syntax) = btree_search(extend_parameters.clone(), SafasCell::atom("syntax")) {
                    // Define any atoms from the original syntax in our interior bindings
                    for (key, value) in btree_iterate(syntax) {
                        if let SafasCell::Atom(atom_id) = &*key {
                            interior_bindings.symbols.insert(*atom_id, value);
                        }
                    }
                }
            }
        }

        for (atom_id, symbol) in self.syntax_cells.iter() {
            interior_bindings.symbols.insert(*atom_id, symbol.clone());
        }
        interior_bindings.symbols.insert(get_id_for_atom_with_name("syntax"), self.syntax_btree.clone());

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

    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, _parameter: CellRef, frame_depth: u32) -> (SymbolBindings, Option<(Box<dyn BindingMonad<Binding=Self::Binding>>, CellRef)>) {
        // Rebind the imported bindings to the new frame
        let (bindings, rebound_imported_bindings)   = rebind_imported_bindings(Arc::clone(&self.imported_bindings), bindings, frame_depth);
        let rebound_imported_bindings               = rebound_imported_bindings.unwrap_or_else(|| self.imported_bindings.clone());

        // Regenerate the syntax symbols with the new imported bindings
        let mut bindings                = bindings;
        let mut new_syntax              = vec![];
        for (atom_id, symbol) in self.syntax_symbols.iter() {
            let (new_bindings, fallback) = match symbol.fallback_syntax { Some(ref fallback) => rebind_cell(fallback, bindings, frame_depth), None => (bindings, None) };
            bindings            = new_bindings;
            let patterns        = symbol.patterns.clone();
            let new_symbol      = SyntaxSymbol {
                patterns:           patterns, 
                imported_bindings:  Arc::clone(&rebound_imported_bindings),
                reference_type:     symbol.reference_type,
                fallback_syntax:    fallback
            };

            new_syntax.push((AtomId(*atom_id), Arc::new(new_symbol)))
        }

        // Rebind the syntax we're extending
        let (bindings, extend_syntax) = match self.extend_syntax {
            Some(ref extend_syntax) => {
                let (bindings, new_extend_syntax) = rebind_cell(extend_syntax, bindings, frame_depth);
                (bindings, Some(new_extend_syntax.unwrap_or_else(|| extend_syntax.clone())))
            },
            None                => (bindings, None)
        };

        // Create a new syntax closure with these symbols
        let new_syntax_closure  = SyntaxClosure::new(new_syntax, rebound_imported_bindings, extend_syntax);
        let mut btree           = btree_new();
        btree                   = btree_insert(btree, (SafasCell::atom("syntax"), new_syntax_closure.syntax_btree())).unwrap();

        (bindings, Some((Box::new(new_syntax_closure), btree)))
    }
}

///
/// Rebinds a single cell to a new frame depth
///
pub (super) fn rebind_cell(binding: &CellRef, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<CellRef>) {
    match &**binding {
        // Frame references need to be imported into the current frame
        SafasCell::FrameReference(outer_cell_id, bound_level, cell_type) => {
            // Import this frame reference
            let mut bindings    = bindings;
            let local_cell_id   = bindings.alloc_cell();
            let outer_cell      = SafasCell::FrameReference(*outer_cell_id, *bound_level + frame_depth, *cell_type).into();
            bindings.import(outer_cell, local_cell_id);

            // Update the binding
            (bindings, Some(SafasCell::FrameReference(local_cell_id, 0, *cell_type).into()))
        }

        // Syntax might need to be rebound to the current frame
        SafasCell::Syntax(old_syntax, val) => {
            // Try to rebind the syntax
            let (new_bindings, new_syntax) = old_syntax.rebind_from_outer_frame(bindings, val.clone(), frame_depth);

            // Update the binding if the syntax update
            if let Some((new_syntax, new_val)) = new_syntax {
                (new_bindings, Some(SafasCell::Syntax(new_syntax, new_val).into()))
            } else {
                (new_bindings, None)
            }
        }

        // Other types are not affected by rebinding
        _ => { (bindings, None) }
    }
}

///
/// Rebinds an imported_bindings hashmap to a new frame at a particular depth
///
pub (super) fn rebind_imported_bindings(imported_bindings: Arc<HashMap<usize, CellRef>>, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<Arc<HashMap<usize, CellRef>>>) {
    // Rebind all of the imported bindings, importing the frame reference and the syntax if there are any
    let mut bindings                    = bindings;
    let mut rebound_imported_bindings   = (*imported_bindings).clone();
    let mut rebound                     = false;

    for (_cell, binding) in rebound_imported_bindings.iter_mut() {
        // Try to rebind the cell
        let (new_bindings, new_binding) = rebind_cell(binding, bindings, frame_depth);
        bindings                        = new_bindings;

        if let Some(new_binding) = new_binding {
            // Copy to the binding and mark as rebound
            *binding    = new_binding;
            rebound     = true;
        }
    }

    // If no bindings were updated, just keep using the same syntax as before
    if !rebound {
        return (bindings, None);
    }

    // Regenerate the syntax symbols with the new imported bindings
    let rebound_imported_bindings   = Arc::new(rebound_imported_bindings);

    (bindings, Some(rebound_imported_bindings))
}
