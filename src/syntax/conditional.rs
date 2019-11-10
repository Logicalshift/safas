use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use std::convert::{TryInto};

///
/// Binds a list of statements
///
fn bind_several_statements(statements: CellRef, bindings: SymbolBindings) -> BindResult<(CellRef, ReferenceType)> {
    // Build up the list of results
    let mut result          = vec![];
    let mut pos             = &*statements;
    let mut bindings        = bindings;
    let mut return_ref_type = ReferenceType::Value;

    // Bind the statements one at a time from the list
    while let SafasCell::List(statement, next) = pos {
        let (bound_statement, new_bindings) = bind_statement(statement.clone(), bindings)?;
        if return_ref_type != ReferenceType::Monad { return_ref_type = bound_statement.reference_type(); }

        bindings                            = new_bindings;
        result.push(bound_statement);

        pos = next;
    }

    Ok(((SafasCell::list_with_cells(result).into(), return_ref_type), bindings))
}

///
/// Compiles a list of statements
///
fn compile_several_statements(statements: CellRef) -> Result<CompiledActions, BindError> {
    // Start with an empty set of actions
    let mut result          = CompiledActions::empty();

    // Work out the reference type of the set of statements
    let mut return_ref_type = ReferenceType::Value;
    let mut pos             = &*statements;
    while let SafasCell::List(statement, next) = pos {
        // Monad statements end up being flat-mapped together: for other types we end up with the reference type of the last statement
        if return_ref_type != ReferenceType::Monad { return_ref_type = statement.reference_type() }

        pos = next;
    }

    // Compile the list of statements
    let mut pos         = &*statements;
    let mut first       = true;
    while let SafasCell::List(statement, next) = pos {
        // Compile the next statement
        let next_statement = compile_statement(statement.clone())?;
        result.extend(next_statement);

        // Flat_map monads together
        if return_ref_type == ReferenceType::Monad {
            let statement_ref_type = statement.reference_type();

            // Wrap statements that don't have a monad return value
            if statement_ref_type != ReferenceType::Monad {
                result.push(Action::Wrap);
            }

            // Push the first value, call next on the future ones
            if first {
                result.push(Action::Push);
            } else {
                result.push(Action::Next);
            }
        }

        pos     = next;
        first   = false;
    }

    // Finally, pop the monad if the return type is a monad
    if !first && return_ref_type == ReferenceType::Monad {
        result.push(Action::Pop);
    }

    Ok(result)
}

///
/// `(if (condition_statements) (if_true_statements) (if_false_statements))`: if the condition statements evaluates to true, evaluate 
/// the 'if true' statements, otherwise the 'if false' statements.
///
pub fn if_keyword()  -> impl BindingMonad<Binding=SyntaxCompiler> {
    get_expression_arguments().and_then(|ListTuple((conditional, if_true, if_false)): ListTuple<(CellRef, CellRef, CellRef)>| {
        BindingFn::from_binding_fn(move |bindings| {
            // Bind the statements
            let (bindings, (conditional, conditional_ref_type)) = match bind_several_statements(conditional.clone(), bindings)  { Ok((result, bindings)) => (bindings, result), Err((err, bindings)) => return (bindings, Err(err)) };
            let (bindings, (if_true, if_true_ref_type))         = match bind_several_statements(if_true.clone(), bindings)      { Ok((result, bindings)) => (bindings, result), Err((err, bindings)) => return (bindings, Err(err)) };
            let (bindings, (if_false, if_false_ref_type))       = match bind_several_statements(if_false.clone(), bindings)     { Ok((result, bindings)) => (bindings, result), Err((err, bindings)) => return (bindings, Err(err)) };

            (bindings, Ok((conditional, if_true, if_false, conditional_ref_type, if_true_ref_type, if_false_ref_type)))
        })

    }).and_then(|(conditional, if_true, if_false, conditional_ref_type, if_true_ref_type, if_false_ref_type)| -> Box<dyn BindingMonad<Binding=SyntaxCompiler>> {

        if conditional_ref_type == ReferenceType::Monad {

            // If the monad is a reftype, we need to flat_map it with the conditional values
            unimplemented!("Monad conditionals not supported yet")

        } else {

            // Standard values are just compiled as a straight 'if' function
            Box::new(BindingFn::from_binding_fn(move |bindings| {
                let result = compile_if_with_value_conditional(conditional.clone(), if_true.clone(), if_false.clone(), if_true_ref_type, if_false_ref_type);

                match result {
                    Ok(result)  => (bindings, Ok(result)),
                    Err(err)    => (bindings, Err(err))
                }
            }))

        }

    })
}

///
/// Given an if statement with a function that
///
fn compile_if_with_value_conditional(conditional: CellRef, if_true: CellRef, if_false: CellRef, if_true_reftype: ReferenceType, if_false_reftype: ReferenceType) -> Result<SyntaxCompiler, BindError> {
    // The return reference type is a monad if either the if_true or if_false types are monads (or the conditional is one)
    let return_ref_type = if if_true_reftype == ReferenceType::Monad || if_false_reftype == ReferenceType::Monad {
        ReferenceType::Monad
    } else {
        if if_true_reftype == ReferenceType::ReturnsMonad && if_false_reftype == ReferenceType::ReturnsMonad {
            ReferenceType::ReturnsMonad
        } else {
            ReferenceType::Value
        }
    };

    let compiler = move |statements: CellRef| -> Result<_, BindError> {
        let ListTuple((conditional, if_true, if_false)) = statements.try_into()?;

        // Compile the statements
        let mut conditional_actions = compile_several_statements(conditional)?;
        let mut if_true             = compile_several_statements(if_true)?;
        let mut if_false            = compile_several_statements(if_false)?;

        // Wrap the results if they need to be due to the return value being a monad
        if return_ref_type == ReferenceType::Monad {
            if if_true_reftype != ReferenceType::Monad {
                if_true.push(Action::Wrap);
            }

            if if_false_reftype != ReferenceType::Monad {
                if_false.push(Action::Wrap);
            }
        }

        // Add the jump commands: if_true ends by jumping over the if_false statements, and the conditional actions jump over if_true when the condition is false
        if_true.push(Action::Jump((if_false.actions.len()+1) as isize));
        conditional_actions.push(Action::JumpIfFalse((if_true.actions.len()+1) as isize));

        // Combine into the result
        let mut result = conditional_actions;
        result.extend(if_true);
        result.extend(if_false);

        Ok(result)
    };

    Ok(SyntaxCompiler::with_compiler_and_reftype(compiler, SafasCell::list_with_cells(vec![conditional, if_true, if_false]).into(), return_ref_type))
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn if_true() {
        let val = eval("(if (=t) (1) (2) )").unwrap().to_string();
        assert!(val == "1".to_string());
    }

    #[test]
    fn if_false() {
        let val = eval("(if (=f) (1) (2) )").unwrap().to_string();
        assert!(val == "2".to_string());
    }

    #[test]
    fn if_with_true_condition() {
        let val = eval("(if ((> 2 1)) (1) (2) )").unwrap().to_string();
        assert!(val == "1".to_string());
    }

    #[test]
    fn if_with_false_condition() {
        let val = eval("(if ((< 2 1)) (1) (2) )").unwrap().to_string();
        assert!(val == "2".to_string());
    }

    #[test]
    fn if_with_monad_result_false() {
        let val = eval("
            (if (=f) 
                ((list 2 3)) 
                ((list 1 (wrap 2))) 
            )
        ").unwrap().to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn if_with_monad_result_true() {
        let val = eval("
            (if (=t) 
                ((list 1 (wrap 2))) 
                ((list 2 3)) 
            )
        ").unwrap().to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn if_with_monad_result_on_opposing_side_true() {
        let val = eval("
            (if (=t) 
                ((list 1 2)) 
                ((list 2 (wrap 3))) 
            )
        ").unwrap().to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn if_with_monad_result_on_opposing_side_false() {
        let val = eval("
            (if (=f) 
                ((list 2 (wrap 3))) 
                ((list 1 2)) 
            )
        ").unwrap().to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn if_with_monad_as_the_condition() {
        let val = eval("
            (if ((wrap =t)) 
                ((list 1 2)) 
                ((list 2 3)) 
            )
        ").unwrap().to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn if_with_monad_as_the_condition_and_the_result() {
        let val = eval("
            (if ((wrap =t)) 
                ((list 1 (wrap 2))) 
                ((list 2 3)) 
            )
        ").unwrap().to_string();
        assert!(val == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }
}
