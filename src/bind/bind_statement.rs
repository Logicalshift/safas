use super::symbol_bindings::*;
use super::bind_error::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;

///
/// Performs binding to generate the actions for a simple statement
///
pub fn bind_statement(source: CellRef, bindings: SymbolBindings) -> BindResult<SmallVec<[Action; 8]>> {
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
                    Number(_)                       |
                    Atom(_)                         |
                    String(_)                       |
                    Char(_)                         |
                    List(_, _)                      |
                    Monad(_)                        |
                    MacroMonad(_)                   |
                    ActionMonad(_)                  => Ok((smallvec![Action::Value(symbol_value)], bindings)),
                    FrameReference(cell_num, frame) => {
                        let (cell_num, frame) = (*cell_num, *frame);
                        if frame == 0 {
                            // Local symbol
                            Ok((smallvec![Action::CellValue(cell_num)], bindings))
                        } else {
                            // Import from a parent frame
                            let mut bindings    = bindings;
                            let local_cell_id   = bindings.alloc_cell();
                            bindings.import(SafasCell::FrameReference(cell_num, frame).into(), local_cell_id);

                            Ok((smallvec![Action::CellValue(local_cell_id)], bindings))
                        }
                    },
                }
            } else {
                // Not a valid symbol
                Err((BindError::UnknownSymbol, bindings))
            }
        }

        // Normal values just get loaded into cell 0
        _other          => { Ok((smallvec![Action::Value(Arc::clone(&source))], bindings)) }
    }
}

///
/// Binds a list statement, like `(cons 1 2)`
///
pub fn bind_list_statement(car: CellRef, cdr: CellRef, bindings: SymbolBindings) -> BindResult<SmallVec<[Action; 8]>> {
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
                    Number(_)                           |
                    Atom(_)                             |
                    String(_)                           |
                    Char(_)                             |
                    List(_, _)                          |
                    Monad(_)                            => { bind_call(smallvec![Action::Value(symbol_value)], cdr, bindings) },

                    // Frame references load the value from the frame and call that
                    FrameReference(_cell_num, _frame)   => { let (actions, bindings) = bind_statement(car, bindings)?; bind_call(actions, cdr, bindings) }
                    
                    // Action and macro monads resolve their respective syntaxes
                    ActionMonad(syntax_compiler)        => {
                        let mut bindings        = bindings.push_interior_frame();
                        bindings.args           = Some(cdr);
                        let (bindings, bound)   = syntax_compiler.binding_monad.resolve(bindings);
                        let (bindings, actions) = match bound {
                            Ok(bound) => {
                                let action_monad = (syntax_compiler.generate_actions)(bound);
                                action_monad.resolve(bindings)
                            },
                            Err(err) => {
                                (bindings, Err(err))
                            }
                        };
                        let (bindings, imports) = bindings.pop();

                        if imports.len() > 0 { panic!("Should not need to import symbols into an interior frame"); }

                        match actions {
                            Ok(actions)     => Ok((actions, bindings)),
                            Err(error)      => Err((error, bindings))
                        }
                    }

                    MacroMonad(macro_monad)             => { 
                        let mut bindings            = bindings.push_interior_frame();
                        bindings.args               = Some(cdr);
                        let (bindings, expanded)    = macro_monad.resolve(bindings);

                        // Rust doesn't really help with the error handling here. We want to bind the statement or preserve the error
                        // then we want to pop the bindings regardless of the error.
                        let actions                 = match expanded {
                            Ok(expanded)            => bind_statement(expanded, bindings),
                            Err(error)              => Err((error, bindings))
                        };

                        match actions {
                            Ok((actions, bindings)) => Ok((actions, bindings.pop().0)),
                            Err((error, bindings))  => Err((error, bindings.pop().0))
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
pub fn bind_call(load_fn: SmallVec<[Action; 8]>, args: CellRef, bindings: SymbolBindings) -> BindResult<SmallVec<[Action; 8]>> {
    let mut bindings = bindings;

    // Start by pushing the function value onto the stack (we'll pop it later on to call the function)
    let mut actions = load_fn;
    actions.push(Action::Push);

    // Push the arguments
    let mut arg_count   = 0;
    let mut next_arg    = args;

    loop {
        match &*next_arg {
            SafasCell::List(car, cdr) => {
                // Evaluate car and push it onto the stack
                let (next_action, next_bindings) = bind_statement(Arc::clone(car), bindings)?;
                actions.extend(next_action);
                actions.push(Action::Push);

                bindings    = next_bindings;

                // cdr contains the next argument
                next_arg    = Arc::clone(cdr);
                arg_count   += 1;
            }

            SafasCell::Nil => {
                // Got a complete list: pop all of the arguments from the stack to call the function
                actions.push(Action::PopList(arg_count));
                break;
            }

            _other => {
                // Incomplete list: evaluate the CDR value
                let (next_action, next_bindings) = bind_statement(next_arg, bindings)?;
                actions.extend(next_action);
                actions.push(Action::Push);

                bindings = next_bindings;

                // Build the args by setting the 'hanging' value as the CDR
                actions.push(Action::PopListWithCdr(arg_count));
                break;
            }
        }
    }

    // Store the arg values into cell 0 (used by call)
    actions.push(Action::StoreCell(0));

    // Pop the function value and call it
    actions.push(Action::Pop);
    actions.push(Action::Call);

    Ok((actions, bindings))
}
