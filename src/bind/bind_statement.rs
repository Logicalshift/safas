use super::symbol_bindings::*;
use super::bind_error::*;
use super::binding_monad::*;
use super::binding_monad_sugar::*;

use crate::meta::*;

use std::sync::*;
use std::result::{Result};

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

            if let Some(symbol_value) = symbol_value {
                use self::SafasCell::*;

                match &*symbol_value {
                    Nil                             |
                    Any(_)                          |
                    Number(_)                       |
                    Atom(_)                         |
                    String(_)                       |
                    Char(_)                         |
                    List(_, _)                      |
                    Monad(_)                        |
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

            if let Some(symbol_value) = symbol_value {
                match &*symbol_value {
                    // Constant values just load that value and call it
                    Nil                                 |
                    Any(_)                              |
                    Number(_)                           |
                    Atom(_)                             |
                    String(_)                           |
                    Char(_)                             |
                    Monad(_)                            => { bind_call(symbol_value, cdr, bindings) },

                    // Lists bind themselves before calling
                    List(_, _)                          => { let (bound_symbol, bindings) = bind_statement(symbol_value, bindings)?; bind_call(bound_symbol, cdr, bindings) }

                    // Frame references load the value from the frame and call that
                    FrameReference(_cell_num, _frame)   => { let (actions, bindings) = bind_statement(car, bindings)?; bind_call(actions, cdr, bindings) }
                    
                    // Action and macro monads resolve their respective syntaxes
                    ActionMonad(syntax_compiler)        => {
                        let mut bindings        = bindings.push_interior_frame();
                        bindings.args           = Some(cdr);
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

    if hanging_cdr {
        let cdr = actions.pop();
        Ok((SafasCell::list_with_cells_and_cdr(actions, cdr.unwrap()).into(), bindings))
    } else {
        Ok((SafasCell::list_with_cells(actions).into(), bindings))
    }
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
