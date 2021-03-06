use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::convert::*;

lazy_static! {
    static ref CLOSURE_ATOM: u64 = get_id_for_atom_with_name("CLOSURE");
    static ref LAMBDA_ATOM: u64  = get_id_for_atom_with_name("LAMBDA");
    static ref MONAD_ATOM: u64   = get_id_for_atom_with_name("MONAD");
}

///
/// The fun monad defines the '(fun (x y) (statement) ...)' syntax
/// 
/// ```(fun (<arg> ...) <statements>)```
/// 
/// Defines a function that will bind the atoms specified in the arg list to the arguments passed in.
/// The result of the function is the value of the last of the list of statements.
///
pub fn fun_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    // Function binding is a bit complicated so we use our own monad implementation
    // TODO (maybe): FunBinder doesn't need to return a cellref any more so we can return a custom structure if needed
    FunBinder.map_result(|bindings| {
        let bound_value = bindings.clone();

        if let Some(bound_value) = bound_value {
            // Need two copies of the function for the syntax compiler
            let function_def        = bound_value.clone();
            let substitute_function = bound_value.clone();

            Ok(SyntaxCompiler::custom(
                move || { compile_function(&function_def) },
                move |map_cell| { subtitute_function_references(&substitute_function, map_cell) },
                bound_value.reference_type
            ))
        } else {
            Err(BindError::ArgumentsWereNotSupplied)
        }
    })
}

///
/// Compiles a function definition to the corresponding actions
///
fn compile_function(function_def: &FunctionBinding) -> Result<CompiledActions, BindError> {
    // Create the cell that contains the function
    let fun = match (function_def.reference_type, &*function_def.definition) {
        (ReferenceType::ReturnsMonad, FunctionDefinition::Lambda(fun))  => SafasCell::FrameMonad(Box::new(ReturnsMonad(fun.clone()))),
        (ReferenceType::ReturnsMonad, FunctionDefinition::Closure(fun)) => SafasCell::FrameMonad(Box::new(ReturnsMonad(fun.clone()))),
        (_, FunctionDefinition::Lambda(fun))                            => SafasCell::FrameMonad(Box::new(fun.clone())),
        (_, FunctionDefinition::Closure(fun))                           => SafasCell::FrameMonad(Box::new(fun.clone()))
    };

    // Closures need to be called to bind their values before the function can be called
    match &*function_def.definition {
        FunctionDefinition::Lambda(_)       => Ok(smallvec![Action::Value(fun.into())].into()),
        FunctionDefinition::Closure(_)      => Ok(smallvec![Action::Value(fun.into()), Action::Call].into())
    }
}

///
/// Substitutes the values in a function reference
///
fn subtitute_function_references(function_def: &FunctionBinding, substitute: &mut dyn FnMut(FrameReference) -> Option<CellRef>) -> SyntaxCompiler {
    // Rebind the function. Lambdas require no rebinding, but closures might require new bindings from the substitution function
    let new_binding = match &*function_def.definition {
        FunctionDefinition::Lambda(lambda)      => FunctionDefinition::Lambda(lambda.clone()),
        FunctionDefinition::Closure(closure)    => FunctionDefinition::Closure(closure.substitute_frame_references(substitute))
    };

    // Create two copies of the definition for the compiler and the substitution routine
    let function_def        = FunctionBinding {
        reference_type: function_def.reference_type,
        definition:     Arc::new(new_binding)
    };
    let substitute_function = function_def.clone();
    let reference_type      = function_def.reference_type;

    // Create the new syntax for this function
    SyntaxCompiler::custom(
        move || { compile_function(&function_def) },
        move |map_cell| { subtitute_function_references(&substitute_function, map_cell) },
        reference_type
    )
}

///
/// The possible ways a function can be defined 
///
enum FunctionDefinition {
    /// Function that does not capture its environment
    Lambda(Lambda<Vec<Action>>),

    /// Function that captures its environment
    Closure(Closure<Vec<Action>>)
}

///
/// Represents a binding of a function
///
#[derive(Clone)]
struct FunctionBinding {
    /// The type of reference represented by the function (ReturnsMonad or Value)
    reference_type: ReferenceType,

    /// The definition of this function
    definition: Arc<FunctionDefinition>
}

struct FunBinder;

impl BindingMonad for FunBinder {
    type Binding=Option<FunctionBinding>;

    fn description(&self) -> String { "##fun##".to_string() }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        (bindings, None)
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
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

        // Pre-bind the statements
        for statement in statements.iter() {
            let (new_bindings, _) = pre_bind_statement(statement.clone(), inner_bindings);
            inner_bindings = new_bindings;
        }

        // Compile the statements
        let mut actions             = vec![];
        let mut monadic_function    = false;

        for statement in statements {
            // Bind the statement
            let bound_statement = bind_statement(statement, inner_bindings)
                .and_then(|(bound, next_bindings)| {
                    let is_monad = bound.reference_type() == ReferenceType::Monad;
                    match compile_statement(bound) {
                        Ok(actions) => Ok(((actions, is_monad), next_bindings)),
                        Err(err)    => Err((err.into(), next_bindings))
                    }
                });

            let (statement_actions, next_binding) = match bound_statement {
                Ok((statement_actions, next_binding))   => (statement_actions, next_binding),
                Err((error, next_binding))              => return (next_binding.pop().0, Err(error))
            };

            // If any action has a monad type, then this function will be flagged as a 'monad' function, and will return a monad type via flat_map
            if let (_, true) = statement_actions { monadic_function = true; }

            // Add these actions to our own
            actions.push(statement_actions);

            inner_bindings = next_binding;
        }

        // If this is a 'monadic' function then add the extra steps needed to bind the final result to the actions
        if monadic_function && actions.len() > 1 {
            for action_num in 0..(actions.len()) {
                let (actions, is_monad) = &mut actions[action_num];

                // Wrap the value in a monad if the value is not itself a monad
                if !*is_monad {
                    actions.push(Action::Wrap)
                }

                // Push the monad on the first action, and flat_map it on any others
                if action_num == 0 {
                    actions.push(Action::Push);
                } else {
                    actions.push(Action::Next);
                }
            }

            // Final action is to return the monad value
            actions.push((smallvec![Action::Pop].into(), true));
        }

        // Collapse the actions into a single set of actions
        let actions             = actions.into_iter()
            .fold(CompiledActions::empty(), |mut collected, (actions, _)| { collected.extend(actions); collected } );

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
                    SafasCell::FrameReference(our_cell_id, 0, _cell_type) => {
                        // Cell from this frame
                        cell_imports.push((*our_cell_id, import_into_cell_id));
                    },

                    SafasCell::FrameReference(their_cell_id, frame_count, cell_type) => {
                        // Import from a parent frame
                        let our_cell_id = bindings.alloc_cell();
                        bindings.import(SafasCell::FrameReference(*their_cell_id, *frame_count, *cell_type).into(), our_cell_id);
                        cell_imports.push((our_cell_id, import_into_cell_id));
                    },

                    _ => panic!("Don't know how to import this type of symbol")
                }
            }

            // Return the closure
            let closure         = Closure::new(actions.to_actions().collect::<Vec<_>>(), cell_imports, num_cells, num_args, monadic_function);
            if monadic_function {
                let closure     = FunctionBinding { 
                    reference_type: ReferenceType::ReturnsMonad,
                    definition:     Arc::new(FunctionDefinition::Closure(closure))
                };

                // Closure needs to be called to create the actual function
                (bindings, Ok(Some(closure)))
            } else {
                let closure     = FunctionBinding { 
                    reference_type: ReferenceType::Value,
                    definition:     Arc::new(FunctionDefinition::Closure(closure))
                };

                // Closure needs to be called to create the actual function
                (bindings, Ok(Some(closure)))
            }
        } else {
            // No imports, so return a straight lambda
            let lambda          = Lambda::new(actions.to_actions().collect::<Vec<_>>(), num_cells, num_args);
            if monadic_function {
                let lambda      = FunctionBinding { 
                    reference_type: ReferenceType::ReturnsMonad,
                    definition:     Arc::new(FunctionDefinition::Lambda(lambda))
                };

                // Lambda can just be executed directly
                (bindings, Ok(Some(lambda)))
            } else {
                let lambda      = FunctionBinding { 
                    reference_type: ReferenceType::Value,
                    definition:     Arc::new(FunctionDefinition::Lambda(lambda))
                };

                // Lambda can just be executed directly
                (bindings, Ok(Some(lambda)))
            }
        }
    }

    fn reference_type(&self, bound_value: CellRef) -> ReferenceType {
        let bound_value: Result<ListTuple<(AtomId, AtomId, CellRef)>, _>   = bound_value.try_into();
        match bound_value {
            Ok(ListTuple((monad_type, _, _)))                   => { if monad_type == AtomId(*MONAD_ATOM) { ReferenceType::ReturnsMonad } else { ReferenceType::Value } },
            Err(_)                                              => ReferenceType::Value
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
            ).unwrap().to_string();
        assert!(val == "42".to_string());
    }


    #[test]
    fn define_and_call_function_with_no_args() {
        let val = eval(
            "(def a (fun () 42))\
            (a)"
            ).unwrap().to_string();
        assert!(val == "42".to_string());
    }

    #[test]
    fn call_function_directly() {
        let val = eval(
            "((fun (x) x) 42)"
            ).unwrap().to_string();
        assert!(val == "42".to_string());
    }

    #[test]
    fn define_and_call_function_with_closure() {
        let val = eval(
                "(def a (fun (x) x)) \
                (def b (fun (x) (a x))) \
                (b 42)"
            ).unwrap().to_string();
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
            ).unwrap().to_string();
        assert!(val == "42".to_string());
    }
}
