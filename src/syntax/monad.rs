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
pub fn wrap_keyword() -> SyntaxCompiler {
    let bind = get_expression_arguments().and_then(|args: ListTuple<(CellRef, )>| {
        let ListTuple((wrap_statement, )) = args;
        bind(wrap_statement)
    });

    let compile = |args: CellRef| {
        // Compile the statement as usual
        let ListTuple((args, )) = args.try_into()?;
        let mut actions         = compile_statement(args)?;

        // Add a wrap action
        actions.push(Action::Wrap);
        Ok(actions)
    };

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compile)
    }
}