use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;

///
/// The lambda monad defines the '(lambda (x y) (statement) ...)' syntax
///
pub struct LambdaKeyword {
}

impl LambdaKeyword {
    pub fn new() -> LambdaKeyword {
        LambdaKeyword { }
    }
}

impl BindingMonad for LambdaKeyword {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##lambda##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Arguments are the argument list and the statements
        let args = bindings.args.clone();
        let args = args.and_then(|args| args.to_vec());
        let args = match args { Some(args) => args, None => return (bindings, Err(BindError::ArgumentsWereNotSupplied)) };

        // Syntax is (args) statements ...
        if args.len() < 2 { return (bindings, Err(BindError::MissingArgument)); }

        // First argument should be a list of atoms, specifying the variables in the lambda
        let mut args            = args;
        let lambda_args         = args.remove(0);
        let statements          = args;

        let lambda_args         = lambda_args.to_vec();
        let lambda_args         = match lambda_args { Some(lambda_args) => lambda_args, None => return (bindings, Err(BindError::LambdaArgumentsNotSupplied)) };

        // Map the args to atom IDs
        let lambda_args         = lambda_args.into_iter()
            .map(|arg| arg.to_atom_id())
            .collect::<Option<Vec<_>>>();
        let lambda_args         = match lambda_args { Some(lambda_args) => lambda_args, None => return (bindings, Err(BindError::VariablesMustBeAtoms)) };

        // Define the initial lambda frame binding
        let num_args            = lambda_args.len();
        let mut inner_bindings  = bindings.push_new_frame();

        for lambda_arg_atom in lambda_args {
            // Create a cell ID for this atom
            let cell_id = inner_bindings.num_cells;
            inner_bindings.num_cells += 1;
            inner_bindings.symbols.insert(lambda_arg_atom, SymbolValue::FrameReference(cell_id, 0));
        }

        // Compile the statements
        let mut actions             = smallvec![];

        for statement in statements {
            // bind the statement
            let (statement_actions, next_binding) = match bind_statement(statement, inner_bindings) {
                Ok((statement_actions, next_binding))   => (statement_actions, next_binding),
                Err((error, next_binding))              => return (next_binding.pop(), Err(error))
            };

            // Add these actions to our own
            actions.extend(statement_actions);

            inner_bindings = next_binding;
        }

        // Capture the number of cells required for the lambda
        let num_cells       = inner_bindings.num_cells;

        // Pop the bindings to return to the parent context
        let bindings        = inner_bindings.pop();

        // Create a lambda from our actions
        let lambda          = Lambda::new(actions, num_cells, num_args);
        let lambda          = Arc::new(lambda);
        let lambda          = SafasCell::Monad(lambda);

        (bindings, Ok(smallvec![Action::Value(Arc::new(lambda))]))
    }
}
