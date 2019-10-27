use super::pattern_match::*;

use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use itertools::*;
use smallvec::*;
use std::sync::*;
use std::collections::{HashMap};
use std::convert::*;

lazy_static! {
    static ref RETURNS_VALUE_ATOM: u64  = get_id_for_atom_with_name("RETURNS_VALUE");
    static ref RETURNS_MONAD_ATOM: u64  = get_id_for_atom_with_name("RETURNS_MONAD");
}

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
    let bind = get_expression_arguments().map_result(|args: ListWithTail<(AtomId, CellRef), CellRef>| {

        // First step: parse the arguments to the expression

        // Fetch the arguments
        let ListWithTail((name, patterns), statements) = args;

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

    }).and_then(|args| {

        // Second step: bind each of the macros and generate the syntax item

        BindingFn::from_binding_fn(move |bindings| {

            // Fetch the values computed by the previous step
            let (name, macros, statements)  = &args;

            // Bind the macros in an inner frame
            let mut evaluation_bindings     = bindings.push_new_frame();
            let mut symbol_syntax           = vec![];

            // Macros can reference each other. Only back-references are allowed so we can bind them properly
            // Initially all symbols generate errors
            for (symbol_id, _) in macros.iter() {
                // Symbols are intially bound to some syntax that generates an error
                let error = SyntaxCompiler { binding_monad: Box::new(BindingFn::from_binding_fn(|bindings| (bindings, Err(BindError::ForwardReferencesNotAllowed)))), generate_actions: Arc::new(|_| Err(BindError::ForwardReferencesNotAllowed)) };
                evaluation_bindings.symbols.insert(*symbol_id, SafasCell::Syntax(error, NIL.clone()).into());
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
                        let arg_cell: CellRef   = SafasCell::FrameReference(arg_cell, 0, ReferenceType::Value).into();

                        // Add to the bindings and the list of cells for this pattern
                        macro_bindings.symbols.insert(arg_atom_id, arg_cell.clone());
                        pattern_cells.push(arg_cell);
                    }
                    
                    // Bind the macro definition (which is a series of statements)
                    let macro_def               = macro_def.to_vec();
                    let macro_def               = match macro_def { Some(def) => def, None => return (macro_bindings.pop().0.pop().0, Err(BindError::SyntaxExpectingList)) };

                    // Prebind each statement
                    for macro_statement in macro_def.iter() {
                        let (new_bindings, _)   = pre_bind_statement(Arc::clone(macro_statement), macro_bindings);
                        macro_bindings          = new_bindings;
                    }

                    // Finish binding them
                    let mut bind_result         = vec![];
                    for macro_statement in macro_def.into_iter() {
                        // Bind this statement
                        let bound_statement     = bind_statement(macro_statement, macro_bindings);
                        let (new_bindings, bound_statement) = match bound_statement { 
                            Ok((result, macro_bindings))    => ((macro_bindings, result)), 
                            Err((err, macro_bindings))      => { return (macro_bindings.pop().0.pop().0, Err(err)); }
                        };

                        // Store in the result
                        macro_bindings = new_bindings;
                        bind_result.push(bound_statement);
                    }

                    // Store in the results
                    bound_patterns.push((pattern_def, pattern_cells, SafasCell::list_with_cells(bind_result).into()));

                    // Revert the inner frame
                    let (new_bindings, _)       = macro_bindings.pop();
                    evaluation_bindings         = new_bindings;
                }

                // Create a syntax symbol
                let symbol = SyntaxSymbol::new(bound_patterns);
                let symbol = Arc::new(symbol);

                // Define this as our symbol name
                evaluation_bindings.symbols.insert(*symbol_id, SafasCell::Syntax(SyntaxSymbol::syntax(symbol.clone()), NIL.clone()).into());
                symbol_syntax.push((AtomId(*symbol_id), symbol))
            }

            // Pop the evaluation frame
            let (mut bindings, imports) = evaluation_bindings.pop();

            // Generate the imported symbol list for the macros
            let mut cell_imports        = HashMap::new();
            for (symbol_value, import_into_cell_id) in imports.into_iter() {
                match &*symbol_value {
                    SafasCell::FrameReference(_our_cell_id, 0, _type) => {
                        // Cell from this frame
                        cell_imports.insert(import_into_cell_id, symbol_value);
                    },

                    SafasCell::FrameReference(their_cell_id, frame_count, their_type) => {
                        // Import from a parent frame
                        let our_cell_id = bindings.alloc_cell();
                        bindings.import(SafasCell::FrameReference(*their_cell_id, *frame_count, *their_type).into(), our_cell_id);
                        cell_imports.insert(import_into_cell_id, SafasCell::FrameReference(our_cell_id, 0, *their_type).into());
                    },

                    _ => panic!("Don't know how to import this type of symbol")
                }
            }

            // Build a syntax closure from the arguments (these are currently bound to the current environment so they
            // can't be passed outside of the current function)
            let syntax_closure  = SyntaxClosure::new(symbol_syntax, Arc::new(cell_imports));

            // Bind to the name
            let AtomId(name_id) = name;
            let syntax          = SafasCell::Syntax(syntax_closure.syntax(), NIL.clone());
            bindings.symbols.insert(*name_id, syntax.into());
            bindings.export(*name_id);

            (bindings, Ok(NIL.clone()))

        })
    });

    let compile = |_args: CellRef| {
        Ok(smallvec![].into())
    };

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compile)
    }
}

///
/// The syntax symbol struct evaluates a single syntax symbol
///
#[derive(Clone)]
struct SyntaxSymbol {
    /// The patterns, their frame bindings and the partially-bound macro
    patterns: Vec<(Arc<PatternMatch>, Vec<CellRef>, CellRef)>,

    /// The bindings that were imported from outside of this symbol
    imported_bindings: Arc<HashMap<usize, CellRef>>,

    /// The type of referernce for this syntax symbol
    reference_type: ReferenceType
}

///
/// Given a partially-bound set of statements, returns if they'll return a monad or a value
/// 
/// We never return ReturnsMonad for a custom syntax, so syntaxes that generate a function can't use
/// this form at the moment: a possible future enhancement might be to return this instead of value
/// if the last statement evaluates this way
///
fn reference_type_for_partially_bound_statements(statements: &CellRef) -> ReferenceType {
    let mut pos = statements;

    while let SafasCell::List(statement, next) = &**pos {
        // The whole set of statements should be treated as a monad if any one of them is
        if statement.reference_type() == ReferenceType::Monad {
            return ReferenceType::Monad;
        }

        pos = next;
    }

    ReferenceType::Value
}

impl SyntaxSymbol {
    ///
    /// Creates a new syntax symbol that will match one of the specified patterns
    ///
    pub fn new(patterns: Vec<(Arc<PatternMatch>, Vec<CellRef>, CellRef)>) -> SyntaxSymbol {
        // This syntax should have a monad reference type if any of its statements have a monad reference type 
        let mut reference_type = ReferenceType::Value;

        for (_, _, partially_bound) in patterns.iter() {
            if reference_type_for_partially_bound_statements(partially_bound) == ReferenceType::Monad {
                // If any of the definitions for a symbol returns a monad, then assume they all do
                reference_type = ReferenceType::Monad;
                break;
            }
        }

        // TODO : we currently initialize the imported bindings to nothing, expecting to fill them in later but this has the
        // issue that when using a macro from within another macro, it won't work properly
        SyntaxSymbol { patterns: patterns, imported_bindings: Arc::new(HashMap::new()), reference_type: reference_type }
    }

    ///
    /// Creates the syntax compiler for this symbol
    ///
    pub fn syntax(symbol: Arc<SyntaxSymbol>) -> SyntaxCompiler {
        let is_monad    = symbol.reference_type == ReferenceType::Monad;

        SyntaxCompiler {
            binding_monad:      Box::new(symbol),
            generate_actions:   Arc::new(move |args| {
                // We compile each of the statements generated by the binding
                let mut actions = CompiledActions::empty();
                let mut first   = true;

                for statement in args.to_vec().unwrap_or_else(|| vec![]) {
                    // Perform basic compilation
                    actions.extend(compile_statement(statement.clone())?);

                    // Map between values if the value is a monad
                    if is_monad {
                        if statement.reference_type() != ReferenceType::Monad {
                            // Wrap the statement if it doesn't return a monad
                            actions.push(Action::Wrap);
                        }

                        if first {
                            // First monad is just pushed onto the stack
                            actions.push(Action::Push);
                        } else {
                            // Others are mapped using the next function
                            actions.push(Action::Next);
                        }
                    }

                    first = false;
                }

                if is_monad && !first {
                    // Pop the monad value if we're in monad
                    actions.push(Action::Pop);
                }

                Ok(actions)
            })
        }
    }
}

impl BindingMonad for Arc<SyntaxSymbol> {
    type Binding=CellRef;

    fn description(&self) -> String { "##syntax_symbol##".to_string() }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        let args = bindings.args.clone().unwrap_or_else(|| NIL.clone());
        (bindings, args)
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        // Get the arguments for this symbol
        let args            = bindings.args.clone().unwrap_or_else(|| NIL.clone());
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
                    let FrameReference(cell_id, _, _) = pattern_cells[arg_idx].clone().try_into().unwrap();

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
                let (bound, bindings) = substitute_cells(bindings, &mut HashMap::new(), partially_bound, &move |cell_id| {
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

    fn reference_type(&self, _bound_value: CellRef) -> ReferenceType {
        self.reference_type
    }
}

///
/// Substitutes any FrameReferences in the partially bound statement for bound values, and rebinds any FrameReferences that are
/// not currently bound
///
fn substitute_cells<SubstituteFn: Fn(usize) -> Option<CellRef>>(bindings: SymbolBindings, allocated_cells: &mut HashMap<usize, usize>, partially_bound: &CellRef, substitutions: &SubstituteFn) -> (CellRef, SymbolBindings) {
    // Bind the cells
    let pos                 = partially_bound;

    match &**pos {
        // Lists are bound recursively
        SafasCell::List(car, cdr) => {
            // TODO: would be more efficient to bind in a loop
            let (car, bindings) = substitute_cells(bindings, allocated_cells, car, substitutions);
            let (cdr, bindings) = substitute_cells(bindings, allocated_cells, cdr, substitutions);

            (SafasCell::List(car, cdr).into(), bindings)
        }

        // Frame references are bound by the substitution function
        SafasCell::FrameReference(cell_id, frame, cell_type) => {
            if *frame == 0 {
                // Is from the macro frame: bind via the subtitutions function
                if let Some(actual_cell) = substitutions(*cell_id) {
                    (actual_cell, bindings)
                } else {
                    // Cells that aren't substituted are allocated on the current frame (they should be internal bindings introduced by calls like def)
                    let mut bindings = bindings;

                    let bound_cell_id = if let Some(bound_cell_id) = allocated_cells.get(cell_id) {
                        // We've already bound this cell to a value on frame
                        *bound_cell_id
                    } else {
                        // This cell needs to be allocated on the current frame
                        let bound_cell_id = bindings.alloc_cell();
                        allocated_cells.insert(*cell_id, bound_cell_id);
                        bound_cell_id
                    };

                    // Return the bound cell
                    (SafasCell::FrameReference(bound_cell_id, 0, *cell_type).into(), bindings)
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

///
/// Represents a syntax closure, which binds syntax to the environment
///
struct SyntaxClosure {
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
        while let SafasCell::List(car, cdr) = pos {
            // Bind car
            match bind_statement(car.clone(), interior_bindings) {
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

            // Next item in the list
            pos = &*cdr;
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

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn evaluate_def_syntax() {
        eval("(def_syntax x ((lda #<x>) (d x)))").unwrap().to_string();
    }

    #[test]
    fn evaluate_macro() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "3");
    }

    #[test]
    fn choose_syntax_1() {
        let val = eval(
            "(def_syntax some_syntax ( (lda #<x>) ((list 1 x))   (lda <x>) ((list 2 x)) ))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "(1 3)");
    }

    #[test]
    fn choose_syntax_2() {
        let val = eval(
            "(def_syntax some_syntax ( (lda #<x>) ((list 1 x))   (lda <x>) ((list 2 x)) ))
            (some_syntax (lda 3))"
            ).unwrap().to_string();

        assert!(val == "(2 3)");
    }

    #[test]
    fn choose_syntax_3() {
        let val = eval(
            "(def_syntax some_syntax ( (lda #<x>) ((list 1 x))   (ldx <x>) ((list 2 x)) ))
            (some_syntax (ldx 3))"
            ).unwrap().to_string();

        assert!(val == "(2 3)");
    }

    #[test]
    fn read_external_binding() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn macro_in_macro() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (def_syntax other_syntax ((ld #<x>) ( (some_syntax (lda #x)) )))
            (other_syntax (ld #3))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn macro_in_function() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            ((fun () (some_syntax (lda #3))))"
            ).unwrap().to_string();

        assert!(val == "3");
    }

    #[test]
    fn read_external_binding_in_function() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            ((fun () (some_syntax (lda #3))))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn macro_in_macro_in_function() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (def_syntax other_syntax ((ld #<x>) ( (some_syntax (lda #x)) )))
            ((fun () (other_syntax (ld #3))))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn external_bindings_are_hygenic() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (def z 5)
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn define_value_in_macro() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) ((def y x) y)))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "3");
    }

    #[test]
    fn define_value_in_macro_list() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) ((def y x) y)))
            (some_syntax (list (lda #3) (lda #4) (lda #5)))"
            ).unwrap().to_string();

        assert!(val == "(3 4 5)");
    }
}
