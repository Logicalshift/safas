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
pub fn quote_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    // The binding just extracts the literal from the expression
    get_expression_arguments().and_then(|args: ListTuple<(CellRef, )>| {
        let ListTuple((literal, )) = args;
        wrap_binding(literal)
    }).map(|literal| {
        let literal = literal.clone();

        // The compiler just loads the literal
        let compiler = move || {
            Ok(smallvec![Action::Value(literal.clone())].into())
        };

        SyntaxCompiler {
            generate_actions:   Arc::new(compiler),
            reference_type:     ReferenceType::Value
        }
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn simple_quote() {
        let val = eval(
                "(quote (1 2 3))"
            ).unwrap().to_string();
        assert!(val == "(1 2 3)".to_string());
    }
}