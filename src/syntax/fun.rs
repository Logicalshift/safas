use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;

///
/// The fun monad defines the '(fun (x y) (statement) ...)' syntax
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
        let fun_args            = match fun_args { Some(fun_args) => fun_args, None => return (bindings, Err(BindError::LambdaArgumentsNotSupplied)) };

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

        // Create a lambda from our actions
        let lambda              = Lambda::new(actions, num_cells, num_args);
        let lambda              = Arc::new(lambda);
        let lambda              = SafasCell::Monad(lambda);

        // If there are any imports, turn into a closure
        if imports.len() > 0 {
            unimplemented!("Closures not implemented yet")
        } else {
            // No imports, so return a straight lambda
            (bindings, Ok(smallvec![Action::Value(Arc::new(lambda))]))
        }
    }
}
