use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use std::convert::{TryInto};

///
/// Binds a list of statements
///
fn bind_several_statements(statements: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    // Build up the list of results
    let mut result      = vec![];
    let mut pos         = &*statements;
    let mut bindings    = bindings;

    // Bind the statements one at a time from the list
    while let SafasCell::List(statement, next) = pos {
        let (bound_statement, new_bindings) = bind_statement(statement.clone(), bindings)?;
        bindings                            = new_bindings;
        result.push(bound_statement);

        pos = next;
    }

    Ok((SafasCell::list_with_cells(result).into(), bindings))
}

///
/// Compiles a list of statements
///
fn compile_several_statements(statements: CellRef) -> Result<CompiledActions, BindError> {
    // Start with an empty set of actions
    let mut result  = CompiledActions::empty();

    // Compile the list of statements
    let mut pos     = &*statements;
    while let SafasCell::List(statement, next) = pos {
        let next_statement = compile_statement(statement.clone())?;
        result.extend(next_statement);

        pos = next;
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
            let (bindings, conditional) = match bind_several_statements(conditional.clone(), bindings)  { Ok((result, bindings)) => (bindings, result), Err((err, bindings)) => return (bindings, Err(err)) };
            let (bindings, if_true)     = match bind_several_statements(if_true.clone(), bindings)      { Ok((result, bindings)) => (bindings, result), Err((err, bindings)) => return (bindings, Err(err)) };
            let (bindings, if_false)    = match bind_several_statements(if_false.clone(), bindings)     { Ok((result, bindings)) => (bindings, result), Err((err, bindings)) => return (bindings, Err(err)) };

            (bindings, Ok((conditional, if_true, if_false)))
        })
    }).map_result(|(conditional, if_true, if_false)| {
        let compiler = |statements: CellRef| -> Result<_, BindError> {
            let ListTuple((conditional, if_true, if_false)) = statements.try_into()?;

            // Compile the statements
            let mut conditional_actions = compile_several_statements(conditional)?;
            let mut if_true             = compile_several_statements(if_true)?;
            let if_false                = compile_several_statements(if_false)?;

            // Add the jump commands: if_true ends by jumping over the if_false statements, and the conditional actions jump over if_true when the condition is false
            if_true.push(Action::Jump((if_false.actions.len()+1) as isize));
            conditional_actions.push(Action::JumpIfFalse((if_true.actions.len()+1) as isize));

            // Combine into the result
            let mut result = conditional_actions;
            result.extend(if_true);
            result.extend(if_false);

            Ok(result)
        };

        Ok(SyntaxCompiler::with_compiler(compiler, SafasCell::list_with_cells(vec![conditional, if_true, if_false]).into()))
    })
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
}
