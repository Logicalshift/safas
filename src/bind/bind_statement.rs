use super::symbol_bindings::*;
use super::bind_error::*;
use super::binding_monad::*;
use super::binding_monad_sugar::*;
use super::syntax_compiler::*;
use super::compile_statement::*;

use crate::meta::*;
use crate::exec::*;
use crate::syntax::*;

use std::sync::*;
use std::result::{Result};

lazy_static! {
    static ref WRAP_KEYWORD: CellRef = SafasCell::ActionMonad(wrap_keyword()).into();
}

///
/// Performs binding to generate the actions for a simple statement
///
pub fn bind_statement(source: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    use self::SafasCell::*;

    match &*source {
        // Lists are processed according to their first value
        List(car, cdr)  => { bind_list_statement(Arc::clone(car), Arc::clone(cdr), bindings) }

        // Atoms bind to their atom value
        Atom(atom_id)   => {
            // Look up the value for this symbol
            let symbol_value = bindings.look_up(*atom_id);

            if let Some((symbol_value, _symbol_level)) = symbol_value {
                use self::SafasCell::*;

                match &*symbol_value {
                    Nil                             |
                    Any(_)                          |
                    Number(_)                       |
                    Atom(_)                         |
                    String(_)                       |
                    BitCode(_)                      |
                    Char(_)                         |
                    List(_, _)                      |
                    Monad(_, _)                     |
                    FrameMonad(_)                   |
                    ActionMonad(_)                  => Ok((symbol_value, bindings)),
                    FrameReference(cell_num, frame) => {
                        let (cell_num, frame) = (*cell_num, *frame);
                        if frame == 0 {
                            // Local symbol
                            Ok((symbol_value, bindings))
                        } else {
                            // Import from a parent frame
                            let mut bindings    = bindings;
                            let local_cell_id   = bindings.alloc_cell();
                            bindings.import(SafasCell::FrameReference(cell_num, frame).into(), local_cell_id);

                            Ok((SafasCell::FrameReference(local_cell_id, 0).into(), bindings))
                        }
                    },
                }
            } else {
                // Not a valid symbol
                Err((BindError::UnknownSymbol, bindings))
            }
        }

        // Normal values just get loaded into cell 0
        _other          => { Ok((source, bindings)) }
    }
}

///
/// Binds a list statement, like `(cons 1 2)`
///
fn bind_list_statement(car: CellRef, cdr: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    use self::SafasCell::*;

    // Action depends on the type of car
    match &*car {
        // Atoms can call a function or act as syntax in this context
        Atom(atom_id)   => {
            use self::SafasCell::*;
            let symbol_value = bindings.look_up(*atom_id);

            if let Some((symbol_value, symbol_level)) = symbol_value {
                match &*symbol_value {
                    // Constant values just load that value and call it
                    Nil                                 |
                    Any(_)                              |
                    Number(_)                           |
                    Atom(_)                             |
                    String(_)                           |
                    BitCode(_)                          |
                    Char(_)                             |
                    Monad(_, _)                         |
                    FrameMonad(_)                       => { bind_call(symbol_value, cdr, bindings) },

                    // Lists bind themselves before calling
                    List(_, _)                          => { let (bound_symbol, bindings) = bind_statement(symbol_value, bindings)?; bind_call(bound_symbol, cdr, bindings) }

                    // Frame references load the value from the frame and call that
                    FrameReference(_cell_num, _frame)   => { let (actions, bindings) = bind_statement(car, bindings)?; bind_call(actions, cdr, bindings) }
                    
                    // Action and macro monads resolve their respective syntaxes
                    ActionMonad(syntax_compiler)        => {
                        // If we're on a different syntax level, try rebinding the monad (the syntax might need to import symbols from an outer frame, for example)
                        let mut bindings = bindings;

                        if symbol_level != 0 {
                            // Try to rebind the syntax from an outer frame
                            let (mut new_bindings, rebound_monad) = syntax_compiler.binding_monad.rebind_from_outer_frame(bindings, symbol_level);

                            // If the compiler rebinds itself...
                            if let Some(rebound_monad) = rebound_monad {
                                // ... create a new syntax using the rebound binding monad
                                let new_syntax = SyntaxCompiler {
                                    binding_monad:      rebound_monad,
                                    generate_actions:   Arc::clone(&syntax_compiler.generate_actions)
                                };

                                // Add to the symbols in the current bindings so we don't need to rebind the syntax multiple times
                                let new_syntax = ActionMonad(new_syntax).into();
                                new_bindings.symbols.insert(*atom_id, new_syntax);
                                new_bindings.export(*atom_id);

                                // Re-evaluate this binding (as we insert the binding at the current level we won't rebind the next time through)
                                return bind_list_statement(car, cdr, new_bindings);
                            }

                            // Update the bindings to apply the effects of the rebinding
                            bindings = new_bindings
                        }

                        let mut bindings        = bindings.push_interior_frame();
                        bindings.args           = Some(cdr);
                        bindings.depth          = Some(symbol_level);
                        let (bindings, bound)   = syntax_compiler.binding_monad.resolve(bindings);
                        let (bindings, imports) = bindings.pop();

                        if imports.len() > 0 { panic!("Should not need to import symbols into an interior frame"); }

                        match bound {
                            Ok(bound)       => Ok((SafasCell::List(symbol_value, bound).into(), bindings)),
                            Err(error)      => Err((error, bindings))
                        }
                    }
                } 
            } else {
                return Err((BindError::UnknownSymbol, bindings));
            }
        },

        // Default action is to evaluate the first item as a statement and call it
        _other          => {
            let (actions, bindings) = bind_statement(car, bindings)?;
            bind_call(actions, cdr, bindings)
        }
    }
}

///
/// Binds a call function, given the actions needed to load the function value
///
fn bind_call(load_fn: CellRef, args: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    let mut bindings = bindings;

    // The function might be generated by a monad
    if load_fn.is_monad() {
        return bind_monad(vec![], load_fn, args, bindings);
    }

    // Start by pushing the function value onto the stack (we'll pop it later on to call the function)
    let mut actions = vec![load_fn];

    // Push the arguments
    let mut next_arg    = args;
    let mut hanging_cdr = false;

    loop {
        match &*next_arg {
            SafasCell::List(car, cdr) => {
                // Evaluate car and push it onto the stack
                let (next_action, next_bindings) = bind_statement(Arc::clone(car), bindings)?;

                if next_action.is_monad() {
                    // Convert to a monad
                    return bind_monad(actions, next_action, Arc::clone(cdr), next_bindings);
                }

                actions.push(next_action);

                bindings    = next_bindings;

                // cdr contains the next argument
                next_arg    = Arc::clone(cdr);
            }

            SafasCell::Nil => {
                // Got a complete list
                break;
            }

            _other => {
                // Incomplete list: evaluate the CDR value
                let (next_action, next_bindings) = bind_statement(next_arg, bindings)?;
                actions.push(next_action);

                bindings    = next_bindings;
                hanging_cdr = true;
                break;
            }
        }
    }

    // If there was a 'hanging' CDR, then generate a result with the same format, otherwise generate a well-formed list
    if hanging_cdr {
        let cdr = actions.pop();
        Ok((SafasCell::list_with_cells_and_cdr(actions, cdr.unwrap()).into(), bindings))
    } else {
        Ok((SafasCell::list_with_cells(actions).into(), bindings))
    }
}

///
/// Given a partially bound function with a monad parameter, rewrites it as a flat_map binding
/// 
/// Say we are evaluating the call (foo x) where 'x' is a monad. This will map this to (flat_map (fun (x) (foo x)) x),
/// returning a new monad as the result of the call. (This is equivalent to 'do' syntax in languages like Haskell but
/// taking account of SAFAS's use of dynamic types instead of static ones)
///
fn bind_monad(args_so_far: Vec<CellRef>, monad: CellRef, remaining_args: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    // TODO: to make this fully work we need to make (fun () monad) itself a monad

    // The remainder of the function will need to be evaluated in a function
    let mut interior_frame  = bindings.push_new_frame();

    // The first parameter of the flat_map function is the value of the monad argument
    let monad_value_cell    = interior_frame.alloc_cell();

    // Next parameters are bound from the closure and are the arguments so far (including the function, if present)
    let other_arguments     = args_so_far.iter().map(|_| interior_frame.alloc_cell()).collect::<Vec<_>>();

    // Generate a partially-bound statement using these arguments (remaining_args are still unbound and go on the end)
    let monad_fn            = SafasCell::List(SafasCell::FrameReference(monad_value_cell, 0).into(), remaining_args);
    let mut monad_fn        = Arc::new(monad_fn);
    for cell_id in other_arguments.iter().rev() {
        monad_fn = SafasCell::List(SafasCell::FrameReference(*cell_id, 0).into(), monad_fn).into();
    }

    // The return value from the monad fn is wrapped to make it a monad too
    let monad_fn            = SafasCell::List(monad_fn, SafasCell::Nil.into()).into();
    let monad_fn            = SafasCell::List(WRAP_KEYWORD.clone(), monad_fn).into();

    // Bind this function
    // (Note: the args_so_far are all frame references here so they should bind to themselves, saving us some issues with rebinding)
    let bound_monad_fn                      = bind_statement(monad_fn, interior_frame);
    let (bound_monad_fn, interior_frame)    = match bound_monad_fn { Ok(fun) => fun, Err((err, interior_frame)) => return Err((err, interior_frame.pop().0)) };

    // Compile to a closure (this generates the function passed to FlatMap later on)
    let monad_flat_map                      = compile_statement(bound_monad_fn);
    let monad_flat_map                      = match monad_flat_map { Ok(flat_map) => flat_map, Err(err) => return Err((err, interior_frame.pop().0)) };
    let interior_frame_size                 = interior_frame.num_cells;

    // Pop the interior frame and bring in any imports
    let (bindings, imports)                 = interior_frame.pop();

    // Add any imports to the list of arguments (all the arguments get imported into our closure)
    let mut other_arguments                 = other_arguments;
    let mut args_so_far                     = args_so_far;
    let mut bindings                        = bindings;

    for (symbol_value, import_into_cell_id) in imports.into_iter() {
        match &*symbol_value {
            SafasCell::FrameReference(_our_cell_id, 0) => {
                // Cell from this frame
                other_arguments.push(import_into_cell_id);
                args_so_far.push(symbol_value);
            },

            SafasCell::FrameReference(their_cell_id, frame_count) => {
                // Import from a parent frame
                let our_cell_id = bindings.alloc_cell();
                bindings.import(SafasCell::FrameReference(*their_cell_id, *frame_count).into(), our_cell_id);

                other_arguments.push(import_into_cell_id);
                args_so_far.push(SafasCell::FrameReference(our_cell_id, 0).into());
            },

            _ => panic!("Don't know how to import this type of symbol")
        }
    }


    // Bind to a closure
    let monad_flat_map                      = StackClosure::new(monad_flat_map, other_arguments, interior_frame_size, 1);

    // Convert things to the final result
    let args_so_far                         = SafasCell::list_with_cells(args_so_far);
    let monad_flat_map                      = SafasCell::FrameMonad(Box::new(monad_flat_map)).into();

    // Result is a list starting with the monad
    let result                              = SafasCell::list_with_cells(vec![monad, args_so_far, monad_flat_map]).into();

    Ok((result, bindings))
}

///
/// Monad that performs binding on a statement
///
struct BindMonad {
    source: Vec<CellRef>
}

impl BindingMonad for BindMonad {
    type Binding=Result<Vec<CellRef>, BindError>;

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        let mut result      = vec![];
        let mut bindings    = bindings;

        for cell in self.source.iter() {
            match bind_statement(cell.clone(), bindings) {
                Ok((bound, new_bindings))   => { bindings = new_bindings; result.push(bound); }
                Err((err, bindings))        => return (bindings, Err(err))
            }
        }

        (bindings, Ok(result))
    }
}

///
/// Creates a binding monad that will bind the specified source
///
pub fn bind(source: CellRef) -> impl BindingMonad<Binding=Result<CellRef, BindError>> {
    BindMonad { source: vec![source] }.and_then_ok(|mut results| wrap_binding(Ok(results.pop().unwrap())))
}

///
/// Creates a binding monad that will bind many items from the specified source
///
pub fn bind_all<Items: IntoIterator<Item=CellRef>>(source: Items) -> impl BindingMonad<Binding=Result<Vec<CellRef>, BindError>> {
    BindMonad { source: source.into_iter().collect() }
}
