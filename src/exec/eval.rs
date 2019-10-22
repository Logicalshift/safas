use super::frame::*;
use super::frame_monad::*;

use crate::meta::*;
use crate::bind::*;

///
/// Creates a monad that wraps the 'nil' value
///
fn wrap_nil() -> MonadType {
    let wrap            = WrapFlatMap(NIL.clone());
    let wrap_flat_map   = SafasCell::FrameMonad(Box::new(wrap));
    let monad_type      = MonadType::new(wrap_flat_map.into());

    monad_type
}

///
/// Wraps a value in a monad
///
fn wrap_value(value: CellRef) -> CellRef {
    let wrap            = WrapFlatMap(value);
    let wrap_flat_map   = SafasCell::FrameMonad(Box::new(wrap));
    let monad_type      = MonadType::new(wrap_flat_map.into());

    SafasCell::Monad(NIL.clone(), monad_type).into()
}

///
/// Evaluates a set of parsed statements
/// 
/// The input values are a statement list, an input monad (if the statements are 'monadic', this is the initial value passed into the monad)
/// and the initial frame and bindings (an updated version of these is returned). `monad` can be set to nil to indicate that a non-monad
/// result can be returned.
///
pub fn eval_statements(statement_list: CellRef, monad: CellRef, frame: Frame, bindings: SymbolBindings) -> (CellRef, Frame, SymbolBindings) {
    // These three values represent the state of the evaluation
    let mut result      = monad;
    let mut frame       = frame;
    let mut bindings    = bindings;

    // Pre-bind the statements
    let mut current_statement = &statement_list;

    while let SafasCell::List(statement, cdr) = &**current_statement {
        // Perform pre-binding
        let (new_bindings, _) = pre_bind_statement(statement.clone(), bindings);
        bindings = new_bindings;

        // Move to the next statement
        current_statement = cdr;
    }

    // Create the bound statements (and determine if we're going to use monad behaviour or not)
    let mut monadic_result      = result.reference_type() == ReferenceType::Monad;
    let mut current_statement   = &statement_list;
    let mut compiled_actions    = vec![];

    while let SafasCell::List(statement, cdr) = &**current_statement {
        // Perform binding
        let bound_statement = bind_statement(statement.clone(), bindings);

        match bound_statement {
            Err((err, new_bindings)) => {
                // Give up with an error
                return (SafasCell::Error(err.into()).into(), frame, new_bindings);
            }

            Ok((bound, new_bindings)) => {
                // Update the bindings and store this statement
                let expression_type = bound.reference_type();
                bindings            = new_bindings;

                if expression_type == ReferenceType::Monad { monadic_result = true; }

                // Compile the bound statement
                let actions         = compile_statement(bound);
                let actions         = match actions { Ok(actions) => actions, Err(err) => return (SafasCell::Error(err.into()).into(), frame, bindings) };

                compiled_actions.push(actions);
            }
        }

        // Move to the next statement
        current_statement = cdr;
    }

    // Prepare the frame for execution
    frame.allocate_for_bindings(&bindings);

    let (nil_monad_value, nil_monad_type) = (NIL.clone(), wrap_nil());

    // Evaluate the actions
    for actions in compiled_actions {
        // Collect the actions into a 
        let actions = actions.to_actions().collect::<Vec<_>>();

        let expr_result = actions.execute(frame);
        let expr_result = match expr_result {
            (new_frame, Ok(expr_result))    => { frame = new_frame; expr_result }
            (new_frame, Err(err))           => { return (SafasCell::Error(err.into()).into(), new_frame, bindings); }
        };

        // Combine the expression result into the final result
        if monadic_result {
            // Wrap the result if we're generating a monad return value
            let expr_result = if expr_result.reference_type() != ReferenceType::Monad {
                wrap_value(expr_result)
            } else {
                expr_result
            };

            // Fetch the current monad value/type from the reuslt
            let (monad_value, monad_type) = match &*result {
                SafasCell::Monad(monad_value, monad_type)   => (monad_value, monad_type),
                _                                           => (&nil_monad_value, &nil_monad_type)
            };

            // Map to the next result
            let (next_frame, next_result)   = monad_type.next(monad_value.clone(), expr_result, frame);
            frame                           = next_frame;

            match next_result {
                Ok(next_result)     => { result = next_result; },
                Err(err)            => { return (SafasCell::Error(err.into()).into(), frame, bindings); }
            }
        } else {
            // The result is the result of the last expression
            result = expr_result;
        }
    }

    (result, frame, bindings)
}
