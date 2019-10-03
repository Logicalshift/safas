use super::bind_error::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::convert::*;
use std::result::{Result};

///
/// Compiles a statement once it has been bound by bind_statement
///
pub fn compile_statement(source: CellRef) -> Result<SmallVec<[Action; 8]>, BindError> {
    let actions = compile_statement_quick(source)?;

    Ok(Action::peephole_optimise(actions))
}

///
/// Compiles a statement once it has been bound by bind_statement (without perfoming optimisation on the result)
/// 
/// Calling the _quick variant is faster when building sequences of actions, as the peephole optimiser only
/// needs to be run once all the actions are together
///
pub fn compile_statement_quick(source: CellRef) -> Result<SmallVec<[Action; 8]>, BindError> {
    use self::SafasCell::*;

    match &*source {
        // Lists are processed according to their first value
        List(car, cdr)  => { compile_list_statement(Arc::clone(car), Arc::clone(cdr)) }

        // Frame references load their respective references
        FrameReference(cell_id, frame, _type) => {
            if *frame != 0 {
                Err(BindError::CannotLoadCellInOtherFrame)
            } else {
                Ok(smallvec![Action::CellValue(*cell_id)])
            }
        }

        // Normal values just get loaded into cell 0
        _other          => { Ok(smallvec![Action::Value(Arc::clone(&source))]) }
    }
}

///
/// Compiles a list statement, like `(cons 1 2)`
///
fn compile_list_statement(car: CellRef, cdr: CellRef) -> Result<SmallVec<[Action; 8]>, BindError> {
    use self::SafasCell::*;

    // Action depends on the type of car
    match &*car {
        // Constant values just load that value and call it
        Nil                                         |
        Any(_)                                      |
        Number(_)                                   |
        Atom(_)                                     |
        String(_)                                   |
        Char(_)                                     |
        BitCode(_)                                  |
        Monad(_, _)                                 |
        FrameMonad(_)                               => {
            if car.is_monad() {
                compile_monad_flat_map(smallvec![Action::Value(car)], cdr)
            } else {
                compile_call(smallvec![Action::Value(car)], cdr) 
            }
        },

        // Lists evaluate to their usual value before calling
        List(_, _)                                  => { 
            if car.is_monad() {
                let actions = compile_statement_quick(car)?; compile_monad_flat_map(actions, cdr) 
            } else {
                let actions = compile_statement_quick(car)?; compile_call(actions, cdr) 
            }
        }

        // Frame references load the value from the frame and call that
        FrameReference(_cell_num, _frame, _type)    => { let actions = compile_statement_quick(car)?; compile_call(actions, cdr) }
        
        // Action and macro monads resolve their respective syntaxes
        ActionMonad(syntax_compiler)                => (syntax_compiler.generate_actions)(cdr),
    }
}

///
/// Compiles a call function, given the actions needed to load the function value
///
pub fn compile_call(load_fn: SmallVec<[Action; 8]>, args: CellRef) -> Result<SmallVec<[Action; 8]>, BindError> {
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
                let next_action = compile_statement_quick(Arc::clone(car))?;
                actions.extend(next_action);
                actions.push(Action::Push);

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
                let next_action = compile_statement_quick(next_arg)?;
                actions.extend(next_action);
                actions.push(Action::Push);

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

    Ok(actions)
}

///
/// Compiles a monad flat_map expression
/// 
/// load_monad specifies the actions required to load the monad into the current value
/// 
/// args specifies the arguments of the monad. There are two of these: the bindings to compile and push on to the stack,
/// and the closure to resolve with these bindings.
///
pub fn compile_monad_flat_map(load_monad: SmallVec<[Action; 8]>, args: CellRef) -> Result<SmallVec<[Action; 8]>, BindError> {
    // Start by pushing the monad onto the stack
    let mut actions = load_monad;
    actions.push(Action::Push);

    // Next push the closure arguments onto the stack. The first value in args should be a list of bindings
    let ListTuple((values, closure)): ListTuple<(CellRef, CellRef)> = args.try_into()?;

    let mut pos = &values;
    while let SafasCell::List(car, cdr) = &**pos {
        // Compile this value
        actions.extend(compile_statement_quick(car.clone())?);

        // Push onto the stack
        actions.push(Action::Push);

        // Move to the next value in the list
        pos = cdr;
    }

    // Generate the closure body by evaluating it
    actions.push(Action::Value(closure));
    actions.push(Action::Call);

    // Call FlatMap to resolve the monad
    actions.push(Action::FlatMap);

    Ok(actions)
}
