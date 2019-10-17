use super::symbol_bindings::*;
use super::binding_monad::*;

use crate::meta::*;

use std::sync::*;

///
/// Performs pre-binding to update the bindings prior to compiling a series of statements
/// 
/// The return value currently has no meaning other than to the pre-binding routines of other syntax elements. It's
/// usually the same statement again.
///
pub fn pre_bind_statement(source: CellRef, bindings: SymbolBindings) -> (SymbolBindings, CellRef) {
    use self::SafasCell::*;

    match &*source {
        // Lists are processed according to their first value
        List(car, cdr)  => { pre_bind_list_statement(Arc::clone(car), Arc::clone(cdr), bindings) }

        // Atoms bind to their atom value
        Atom(atom_id)   => {
            // Look up the value for this symbol
            let symbol_value = bindings.look_up(*atom_id);

            if let Some((symbol_value, _symbol_level)) = symbol_value {
                use self::SafasCell::*;

                match &*symbol_value {
                    Nil                     |
                    Any(_)                  |
                    Number(_)               |
                    Atom(_)                 |
                    String(_)               |
                    BitCode(_)              |
                    Char(_)                 |
                    List(_, _)              |
                    Monad(_, _)             |
                    FrameMonad(_)           |
                    ActionMonad(_, _)       |
                    FrameReference(_, _, _) => (bindings, symbol_value),
                }
            } else {
                // Not a valid symbol, or not defined yet
                // 
                // Most definitions aren't actually added to bindings until they can be accessed, so any symbols defined in the
                // current context won't yet be available at this point.
                // 
                // TODO: one issue here is that if there's a symbol that wants pre-binding but doesn't want to declare itself as
                // a 'forward' declaration it can't currently get any pre-binding behaviour (to fix this we need a 'pre-bound' cell type
                // that acts transparent in the main binding but is returned here)
                (bindings, source)
            }
        }

        // Normal values just get loaded into cell 0
        _other          => { (bindings, source) }
    }
}

///
/// Pre-binds a list statement, like `(cons 1 2)`
///
fn pre_bind_list_statement(car: CellRef, cdr: CellRef, bindings: SymbolBindings) -> (SymbolBindings, CellRef) {
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
                    Nil                                         |
                    Any(_)                                      |
                    Number(_)                                   |
                    Atom(_)                                     |
                    String(_)                                   |
                    BitCode(_)                                  |
                    Char(_)                                     |
                    Monad(_, _)                                 |
                    FrameMonad(_)                               => { pre_bind_call(symbol_value, cdr, bindings) },

                    // Lists bind themselves before calling
                    List(_, _)                                  => { let (bindings, bound_symbol) = pre_bind_statement(symbol_value, bindings); pre_bind_call(bound_symbol, cdr, bindings) }

                    // Frame references load the value from the frame and call that
                    FrameReference(_cell_num, _frame, _type)    => { let (bindings, value) = pre_bind_statement(car, bindings); pre_bind_call(value, cdr, bindings) }
                    
                    // Action and macro monads resolve their respective syntaxes
                    ActionMonad(syntax_compiler, _)             => {
                        // During pre-binding, we don't perform any imports
                        let mut bindings        = bindings.push_interior_frame();
                        bindings.args           = Some(cdr);
                        bindings.depth          = Some(symbol_level);
                        let (bindings, bound)   = syntax_compiler.binding_monad.pre_bind(bindings);
                        let (bindings, imports) = bindings.pop();

                        if imports.len() > 0 { panic!("Should be no imports when pre-binding"); }

                        (bindings, SafasCell::List(symbol_value, bound).into())
                    }
                } 
            } else {
                // Not a valid symbol, or not defined yet (pre-bind like a function call)
                // 
                // TODO: one issue here is that if there's a symbol that wants pre-binding but doesn't want to declare itself as
                // a 'forward' declaration it can't currently get any pre-binding behaviour (to fix this we need a 'pre-bound' cell type
                // that acts transparent in the main binding but is returned here)
                let (bindings, value) = pre_bind_statement(car, bindings);
                pre_bind_call(value, cdr, bindings)
            }
        },

        // Default action is to evaluate the first item as a statement and call it
        _other          => {
            let (bindings, value) = pre_bind_statement(car, bindings);
            pre_bind_call(value, cdr, bindings)
        }
    }
}

///
/// Pre-binds a call operation given the value that will evaluate to the function
///
fn pre_bind_call(load_fn: CellRef, args: CellRef, bindings: SymbolBindings) -> (SymbolBindings, CellRef) {
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
                let (next_bindings, next_action) = pre_bind_statement(Arc::clone(car), bindings);

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
                let (next_bindings, next_action) = pre_bind_statement(next_arg, bindings);
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
        (bindings, SafasCell::list_with_cells_and_cdr(actions, cdr.unwrap()).into())
    } else {
        (bindings, SafasCell::list_with_cells(actions).into())
    }
}
