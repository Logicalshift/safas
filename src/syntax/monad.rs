use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use std::sync::*;
use std::convert::*;

///
/// Generates a syntax compiler for a (wrap value) statement
/// 
/// Unlike the 'wrap' function this uses the 'Wrap' opcode (this doesn't return true for is_monad when binding so we use the
/// wrap function most places)
///
pub fn wrap_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    get_expression_arguments().and_then(|args: ListTuple<(CellRef, )>| {
        let ListTuple((wrap_statement, )) = args;
        bind(wrap_statement)
    }).map(|wrap_statement| {
        let wrap_statement = wrap_statement.clone();
        let compile = move || {
            // Compile the statement as usual
            let ListTuple((args, )) = wrap_statement.clone().try_into()?;
            let mut actions         = compile_statement(args)?;

            // Add a wrap action
            actions.push(Action::Wrap);
            Ok(actions)
        };

        SyntaxCompiler {
            generate_actions:   Arc::new(compile),
            reference_type:     ReferenceType::Monad
        }
    })
}
