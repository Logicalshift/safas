use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::convert::*;
use std::sync::*;

lazy_static! {
    static ref CLOSURE_ATOM: u64 = get_id_for_atom_with_name("CLOSURE");
    static ref LAMBDA_ATOM: u64  = get_id_for_atom_with_name("LAMBDA");
}

///
/// The fun monad defines the '(fun (x y) (statement) ...)' syntax
/// 
/// ```(fun (<arg> ...) <statements>)```
/// 
/// Defines a function that will bind the atoms specified in the arg list to the arguments passed in.
/// The result of the function is the value of the last of the list of statements.
///
pub fn fun_keyword() -> SyntaxCompiler {
    // Function binding is a bit complicated so we use our own monad implementation
    let bind    = FunBinder;

    // Compiling needs to call closures and just store lambdas
    let compile = |bound_value: CellRef| -> Result<_, BindError> {
        // Our monad generates something like (CLOSURE <some_closure>)
        let bound_value: ListTuple<(AtomId, CellRef)>   = bound_value.try_into()?;
        let ListTuple((fun_type, fun))                  = bound_value;

        if fun_type == AtomId(*CLOSURE_ATOM) {
            // The closure needs to be called to bind its values
            Ok(smallvec![Action::Value(fun), Action::Call])
        } else if fun_type == AtomId(*LAMBDA_ATOM) {
            // Lambdas can just 
            Ok(smallvec![Action::Value(fun)])
        } else {
            // Unknown type of function (binder error/input from the wrong place)
            Err(BindError::UnknownSymbol)
        }
    };

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compile)
    }
}

struct FunBinder;

impl BindingMonad for FunBinder {
    type Binding=Result<CellRef, BindError>;

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
            // Bind the statement
            let bound_statement = bind_statement(statement, inner_bindings)
                .and_then(|(bound, next_bindings)| {
                    match compile_statement(bound) {
                        Ok(actions) => Ok((actions, next_bindings)),
                        Err(err)    => Err((err.into(), next_bindings))
                    }
                });

            let (statement_actions, next_binding) = match bound_statement {
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
                match &*symbol_value {
                    SafasCell::FrameReference(our_cell_id, 0) => {
                        // Cell from this frame
                        cell_imports.push((*our_cell_id, import_into_cell_id));
                    },

                    SafasCell::FrameReference(their_cell_id, frame_count) => {
                        // Import from a parent frame
                        let our_cell_id = bindings.alloc_cell();
                        bindings.import(SafasCell::FrameReference(*their_cell_id, *frame_count).into(), our_cell_id);
                        cell_imports.push((our_cell_id, import_into_cell_id));
                    },

                    _ => panic!("Don't know how to import this type of symbol")
                }
            }

            // Return the closure
            let closure         = Closure::new(actions, cell_imports, num_cells, num_args);
            let closure         = Box::new(closure);
            let closure         = SafasCell::FrameMonad(closure);

            // Closure needs to be called to create the actual function
            (bindings, Ok(SafasCell::list_with_cells(vec![AtomId(*CLOSURE_ATOM).into(), closure.into()])))
        } else {
            // No imports, so return a straight lambda
            let lambda          = Lambda::new(actions, num_cells, num_args);
            let lambda          = Box::new(lambda);
            let lambda          = SafasCell::FrameMonad(lambda);

            // Lambda can just be executed directly
            (bindings, Ok(SafasCell::list_with_cells(vec![AtomId(*LAMBDA_ATOM).into(), lambda.into()])))
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
