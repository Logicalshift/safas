use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;

///
/// The monad for the 'quote' syntax (quote literal)
/// 
/// (quote (1 2 3)) evaluates to exactly (1 2 3)
///
pub fn quote_keyword() -> SyntaxCompiler {
    // The binding just extracts the literal from the expression
    let bind = get_expression_arguments().and_then_ok(|args: ListTuple<(CellRef, )>| {
        let ListTuple((literal, )) = args;
        wrap_binding(Ok(literal))
    });

    // The compiler just loads the literal
    let compiler = |literal: CellRef| {
        Ok(smallvec![Action::Value(literal.clone())])
    };

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compiler)
    }
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn simple_quote() {
        let val = eval(
                "(quote (1 2 3))"
            ).unwrap().0.to_string();
        assert!(val == "(1 2 3)".to_string());
    }
}