use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;

///
/// The fun monad defines the '(fun (x y) (statement) ...)' syntax
/// 
/// ```(fun (<arg> ...) <statements>)```
/// 
/// Defines a function that will bind the atoms specified in the arg list to the arguments passed in.
/// The result of the function is the value of the last of the list of statements.
///
pub struct FunKeyword {
}

impl FunKeyword {
    pub fn new() -> FunKeyword {
        FunKeyword { }
    }
}

impl BindingMonad for FunKeyword {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##fun##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Arguments are the argument list and the statements
        let args = bindings.args.clone();
        let args = args.and_then(|args| args.to_vec());
        let args = match args { Some(args) => args, None => return (bindings, Err(BindError::ArgumentsWereNotSupplied)) };

        // Syntax is (args) statements ...
        if args.len() < 2 { return (bindings, Err(BindError::MissingArgument)); }

        // First argument should be a list of atoms, specifying the variables in the lambda
        let mut args            = args;
        let fun_args            = args.remove(0);
        let statements          = args;

        let fun_args            = fun_args.to_vec();
        let fun_args            = match fun_args { Some(fun_args) => fun_args, None => return (bindings, Err(BindError::FunArgumentsNotSupplied)) };

        // Map the args to atom IDs
        let fun_args            = fun_args.into_iter()
            .map(|arg| arg.to_atom_id())
            .collect::<Option<Vec<_>>>();
        let fun_args            = match fun_args { Some(fun_args) => fun_args, None => return (bindings, Err(BindError::VariablesMustBeAtoms)) };

        // Define the initial lambda frame binding
        let num_args            = fun_args.len();
        let mut inner_bindings  = bindings.push_new_frame();

        for fun_arg_atom in fun_args {
            // Create a cell ID for this atom
            inner_bindings.bind_atom_to_new_cell(fun_arg_atom);
        }

        // Compile the statements
        let mut actions             = smallvec![];

        for statement in statements {
            // bind the statement
            let (statement_actions, next_binding) = match bind_statement(statement, inner_bindings) {
                Ok((statement_actions, next_binding))   => (statement_actions, next_binding),
                Err((error, next_binding))              => return (next_binding.pop().0, Err(error))
            };

            // Add these actions to our own
            actions.extend(statement_actions);

            inner_bindings = next_binding;
        }

        // Capture the number of cells required for the lambda
        let num_cells           = inner_bindings.num_cells;

        // Pop the bindings to return to the parent context
        let (bindings, imports) = inner_bindings.pop();

        if imports.len() > 0 {
            // If there are any imports, turn into a closure
            let mut cell_imports    = vec![];
            let mut bindings        = bindings;

            // Work out the cells to import into the closure
            for (symbol_value, import_into_cell_id) in imports.into_iter() {
                match symbol_value {
                    SymbolValue::FrameReference(our_cell_id, 0) => {
                        // Cell from this frame
                        cell_imports.push((our_cell_id, import_into_cell_id));
                    },

                    SymbolValue::FrameReference(their_cell_id, frame_count) => {
                        // Import from a parent frame
                        let our_cell_id = bindings.alloc_cell();
                        bindings.import(SymbolValue::FrameReference(their_cell_id, frame_count), our_cell_id);
                        cell_imports.push((our_cell_id, import_into_cell_id));
                    },

                    _ => panic!("Don't know how to import this type of symbol")
                }
            }

            // Return the closure
            let closure         = Closure::new(actions, cell_imports, num_cells, num_args);
            let closure         = Arc::new(closure);
            let closure         = SafasCell::Monad(closure);

            // Call the closure to bind it here
            (bindings, Ok(smallvec![Action::Value(Arc::new(closure)), Action::Call]))
        } else {
            // No imports, so return a straight lambda
            let lambda          = Lambda::new(actions, num_cells, num_args);
            let lambda          = Arc::new(lambda);
            let lambda          = SafasCell::Monad(lambda);

            (bindings, Ok(smallvec![Action::Value(Arc::new(lambda))]))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn define_and_call_function() {
        let val = eval(
            "(def a (fun (x) x))\
            (a 42)"
            ).unwrap().0.to_string();
        assert!(val == "42".to_string());
    }


    #[test]
    fn define_and_call_function_with_no_args() {
        let val = eval(
            "(def a (fun () 42))\
            (a)"
            ).unwrap().0.to_string();
        assert!(val == "42".to_string());
    }

    #[test]
    fn call_function_directly() {
        let val = eval(
            "((fun (x) x) 42)"
            ).unwrap().0.to_string();
        assert!(val == "42".to_string());
    }

    #[test]
    fn define_and_call_function_with_closure() {
        let val = eval(
                "(def a (fun (x) x)) \
                (def b (fun (x) (a x))) \
                (b 42)"
            ).unwrap().0.to_string();
        assert!(val == "42".to_string());
    }

    #[test]
    fn define_and_call_function_with_recursive_closure() {
        let val = eval(
                "(def a (fun (x) x)) \
                (def b (fun (x) \
                    (def c (fun (y) (a y))) \
                    (c x))) \
                (b 42)"
            ).unwrap().0.to_string();
        assert!(val == "42".to_string());
    }
}
