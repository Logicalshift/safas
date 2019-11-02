use super::pattern_match::*;
use super::syntax_closure::*;

use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use std::sync::*;
use std::collections::{HashMap};
use std::convert::*;

///
/// A syntax symbol represents the possible pattern matches and bindings for a single start symbol
/// 
/// This is generally used with the syntax closure for a specific type of symbol to match against.
/// 
/// The way a syntax symbol works is that we bind our expression as if it were in a new frame, with the values
/// in the pattern as the arguments. This captures values as if it were a closure, with the arguments as
/// frame cells. To compile the syntax, these 'fake' frame cells are substituted for the real ones.
/// 
/// Values used from where the syntax is defined are captured as they are for function closures, except instead
/// of using a closure, we fetch the value from the `imported_bindings`. This ensures that syntax macros are
/// `hygenic` in that all symbols will bind to the ones available when the syntax was created.
/// 
/// This is a bit dependent on some implementation details of all the other syntax items. Specifically, we must
/// be able to see and replace FrameReferences to resolve bindings. Syntaxes that store frame references using
/// their own format rather than frame reference cells, or which don't support substituting frame references for
/// other kinds of item will generate errors with this scheme.
///
#[derive(Clone)]
pub struct SyntaxSymbol {
    /// The patterns to match, the frame references that the values in the pattern bind to, and the partially bound expression that it should evaluate to
    pub (super) patterns: Vec<(Arc<PatternMatch>, Vec<CellRef>, CellRef)>,

    /// The bindings that were imported from outside of this symbol
    pub (super) imported_bindings: Arc<HashMap<usize, CellRef>>,

    /// The type of referernce for this syntax symbol
    pub (super) reference_type: ReferenceType
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
    pub fn syntax(symbol: Arc<SyntaxSymbol>) -> impl BindingMonad<Binding=SyntaxCompiler> {
        let is_monad    = symbol.reference_type == ReferenceType::Monad;
        symbol.map(move |args| {
            let args        = args.clone();
            let compile     = move |args: CellRef| {
                // We compile each of the statements generated by the binding
                let mut actions = CompiledActions::empty();
                let mut first   = true;

                for statement in args.to_vec().unwrap_or_else(|| vec![]) {
                    // Compile the statement
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
            };

            let reference_type = if is_monad { ReferenceType::Monad } else { ReferenceType::Value };
            SyntaxCompiler::with_compiler_and_reftype(compile, args, reference_type)
        })
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

                let mut substitutions       = vec![];

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
                    substitutions.push((cell_id, bound_val));
                }

                // Substitute the arguments into the macro statements
                return bind_syntax_monad(bindings, substitutions, partially_bound, &self.imported_bindings);
            }
        }

        // No matching pattern
        (bindings, Err(BindError::SyntaxMatchFailed))
    }

    fn reference_type(&self, _bound_value: CellRef) -> ReferenceType {
        self.reference_type
    }

    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<Box<dyn BindingMonad<Binding=Self::Binding>>>) {
        // Rebind the imported bindings to the new frame
        let (bindings, rebound_imported_bindings)   = rebind_imported_bindings(Arc::clone(&self.imported_bindings), bindings, frame_depth);

        // Map to a new syntax symbol
        let rebound_syntax                          = rebound_imported_bindings.map(|rebound_imported_bindings| {
            SyntaxSymbol {
                patterns:           self.patterns.clone(),
                imported_bindings:  rebound_imported_bindings,
                reference_type:     self.reference_type
            }
        });
        let rebound_syntax                          = rebound_syntax.map(|rebound_syntax| -> Box<dyn BindingMonad<Binding=Self::Binding>> { Box::new(Arc::new(rebound_syntax)) });

        (bindings, rebound_syntax)
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

        SafasCell::BoundSyntax(compiler) => {
            let mut bindings    = bindings;
            let substitute      = compiler.substitute_frame_refs(|FrameReference(cell_id, frame, cell_type)| {
                if frame == 0 {
                    // Is from the macro frame: bind via the subtitutions function
                    if let Some(actual_cell) = substitutions(cell_id) {
                        Some(actual_cell)
                    } else {
                        let bound_cell_id = if let Some(bound_cell_id) = allocated_cells.get(&cell_id) {
                            // We've already bound this cell to a value on frame
                            *bound_cell_id
                        } else {
                            // This cell needs to be allocated on the current frame
                            let bound_cell_id = bindings.alloc_cell();
                            allocated_cells.insert(cell_id, bound_cell_id);
                            bound_cell_id
                        };

                        // Return the bound cell
                        Some(SafasCell::FrameReference(bound_cell_id, 0, cell_type).into())
                    }
                } else {
                    // Bound from a different frame
                    None
                }
            });
            (SafasCell::BoundSyntax(substitute).into(), bindings)
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
/// If the specified set of substitutions contains a monad, rebinds with a flat_map to generate the final expression
///
fn bind_syntax_monad(bindings: SymbolBindings, substitutions: Vec<(usize, CellRef)>, partially_bound: &CellRef, imported_bindings: &Arc<HashMap<usize, CellRef>>) -> (SymbolBindings, Result<CellRef, BindError>) {
    // Iterate through the substitutions. We're looking for the first one that has a monad reference type
    let mut monad_index = (0..substitutions.len()).into_iter()
        .filter(|index| substitutions[*index].1.reference_type() == ReferenceType::Monad)
        .nth(0);

    if let Some(monad_index) = monad_index {
        // Replace the monad with the parameter of a stack closure and try again
        unimplemented!("Can't rewrite monads yet: {}", monad_index)
    } else {
        // If none of the parameters are monads, we can just substitute straight into the macro
        let substitutions = substitutions.into_iter().collect::<HashMap<_, _>>();

        let (bound, bindings) = substitute_cells(bindings, &mut HashMap::new(), partially_bound, &move |cell_id| {
            substitutions.get(&cell_id)
                .or_else(|| imported_bindings.get(&cell_id))
                .cloned()
        });

        // This is the result
        (bindings, Ok(bound))
    }
}
