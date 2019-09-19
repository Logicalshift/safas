use super::symbol_bindings::*;
use super::symbol_value::*;
use super::bind_error::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::result::{Result};

///
/// Performs binding to generate the actions for a simple statement
///
pub fn bind_statement(source: Arc<SafasCell>, bindings: SymbolBindings) -> Result<(SmallVec<[Action; 8]>, SymbolBindings), BindError> {
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
                    Unbound(_atom_id)               => Err(BindError::UnboundSymbol),
                    FrameMonad(monad)               => Ok((smallvec![Action::Value(Arc::new(SafasCell::Monad(Arc::clone(&monad))))], bindings)),
                    MacroMonad(monad)               => Ok((smallvec![Action::Value(Arc::new(SafasCell::MacroMonad(Arc::clone(&monad))))], bindings)),
                    ActionMonad(monad)              => Ok((smallvec![Action::Value(Arc::new(SafasCell::ActionMonad(Arc::clone(&monad))))], bindings)),
                    FrameReference(cell_num, frame) => {
                        if frame == 0 {
                            Ok((smallvec![Action::CellValue(cell_num)], bindings))
                        } else {
                            unimplemented!("Closures not implemented yet")
                        }
                    },
                }
            } else {
                // Not a valid symbol
                Err(BindError::UnknownSymbol)
            }
        }

        // Normal values just get loaded into cell 0
        other           => { Ok((smallvec![Action::Value(Arc::clone(&source))], bindings)) }
    }
}

///
/// Binds a list statement, like `(cons 1 2)`
///
pub fn bind_list_statement(car: Arc<SafasCell>, cdr: Arc<SafasCell>, bindings: SymbolBindings) -> Result<(SmallVec<[Action; 8]>, SymbolBindings), BindError> {
    use self::SafasCell::*;

    // Action depends on the type of car
    match &*car {
        // Atoms can call a function or act as syntax in this context
        Atom(atom_id)   => {
            use self::SymbolValue::*;
            let symbol_value = bindings.look_up(*atom_id);

            match symbol_value {
                None                                    => return Err(BindError::UnknownSymbol),
                Some(Constant(value))                   => return Err(BindError::ConstantsCannotBeCalled),
                Some(Unbound(_atom_id))                 => return Err(BindError::UnboundSymbol),
                Some(FrameReference(_cell_num, _frame)) => { let (actions, bindings) = bind_statement(car, bindings)?; bind_call(actions, cdr, bindings) }
                Some(FrameMonad(frame_monad))           => { bind_call(smallvec![Action::Value(Arc::new(SafasCell::Monad(Arc::clone(&frame_monad))))], cdr, bindings) }
                
                Some(ActionMonad(action_monad))         => {
                    let mut bindings        = bindings.push_interior_frame();
                    bindings.args           = Some(cdr);
                    let (bindings, actions) = action_monad.resolve(bindings);
                    let bindings            = bindings.pop();
                    let actions             = (*actions?).clone();
                    Ok((actions, bindings))
                }

                Some(MacroMonad(macro_monad))           => { 
                    let mut bindings            = bindings.push_interior_frame();
                    bindings.args               = Some(cdr);
                    let (bindings, expanded)    = macro_monad.resolve(bindings);
                    let (actions, bindings)     = bind_statement(expanded?, bindings)?;
                    let bindings                = bindings.pop();
                    Ok((actions, bindings))
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
pub fn bind_call(load_fn: SmallVec<[Action; 8]>, args: Arc<SafasCell>, bindings: SymbolBindings) -> Result<(SmallVec<[Action; 8]>, SymbolBindings), BindError> {
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
