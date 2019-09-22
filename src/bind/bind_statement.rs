use super::symbol_bindings::*;
use super::symbol_value::*;
use super::bind_error::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;

///
/// Performs binding to generate the actions for a simple statement
///
pub fn bind_statement(source: Arc<SafasCell>, bindings: SymbolBindings) -> BindResult<SmallVec<[Action; 8]>> {
    use self::SafasCell::*;

    match &*source {
        // Lists are processed according to their first value
        List(car, cdr)  => { bind_list_statement(Arc::clone(car), Arc::clone(cdr), bindings) }

        // Atoms bind to their atom value
        Atom(atom_id)   => {
            // Look up the value for this symbol
            let symbol_value = bindings.look_up(*atom_id);

            if let Some(symbol_value) = symbol_value {
                use self::SymbolValue::*;

                match symbol_value {
                    Constant(value)                 => Ok((smallvec![Action::Value(Arc::clone(&value))], bindings)),
                    Unbound(_atom_id)               => Err((BindError::UnboundSymbol, bindings)),
                    FrameMonad(monad)               => Ok((smallvec![Action::Value(Arc::new(SafasCell::Monad(Arc::clone(&monad))))], bindings)),
                    MacroMonad(monad)               => Ok((smallvec![Action::Value(Arc::new(SafasCell::MacroMonad(Arc::clone(&monad))))], bindings)),
                    ActionMonad(monad)              => Ok((smallvec![Action::Value(Arc::new(SafasCell::ActionMonad(Arc::clone(&monad))))], bindings)),
                    FrameReference(cell_num, frame) => {
                        if frame == 0 {
                            // Local symbol
                            Ok((smallvec![Action::CellValue(cell_num)], bindings))
                        } else {
                            // Import from a parent frame
                            let mut bindings    = bindings;
                            let local_cell_id   = bindings.alloc_cell();
                            bindings.import(SymbolValue::FrameReference(cell_num, frame), local_cell_id);

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
pub fn bind_list_statement(car: Arc<SafasCell>, cdr: Arc<SafasCell>, bindings: SymbolBindings) -> BindResult<SmallVec<[Action; 8]>> {
    use self::SafasCell::*;

    // Action depends on the type of car
    match &*car {
        // Atoms can call a function or act as syntax in this context
        Atom(atom_id)   => {
            use self::SymbolValue::*;
            let symbol_value = bindings.look_up(*atom_id);

            match symbol_value {
                None                                    => return Err((BindError::UnknownSymbol, bindings)),
                Some(Unbound(_atom_id))                 => return Err((BindError::UnboundSymbol, bindings)),
                Some(FrameReference(_cell_num, _frame)) => { let (actions, bindings) = bind_statement(car, bindings)?; bind_call(actions, cdr, bindings) }
                Some(Constant(value))                   => { bind_call(smallvec![Action::Value(Arc::clone(&value))], cdr, bindings) },
                Some(FrameMonad(frame_monad))           => { bind_call(smallvec![Action::Value(Arc::new(SafasCell::Monad(Arc::clone(&frame_monad))))], cdr, bindings) }
                
                Some(ActionMonad(action_monad))         => {
                    let mut bindings        = bindings.push_interior_frame();
                    bindings.args           = Some(cdr);
                    let (bindings, actions) = action_monad.resolve(bindings);
                    let (bindings, imports) = bindings.pop();

                    if imports.len() > 0 { panic!("Should not need to import symbols into an interior frame"); }

                    match actions {
                        Ok(actions)     => Ok((actions, bindings)),
                        Err(error)      => Err((error, bindings))
                    }
                }

                Some(MacroMonad(macro_monad))           => { 
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
        },

        // Default action is just the literal value of the list
        _other          => Ok((smallvec![Action::Value(Arc::new(SafasCell::List(car, cdr)))], bindings))
    }
}

///
/// Binds a call function, given the actions needed to load the function value
///
pub fn bind_call(load_fn: SmallVec<[Action; 8]>, args: Arc<SafasCell>, bindings: SymbolBindings) -> BindResult<SmallVec<[Action; 8]>> {
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
