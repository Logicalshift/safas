use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::convert::*;

///
/// The monad for the 'quote' syntax (quote literal)
/// 
/// (quote (1 2 3)) evaluates to exactly (1 2 3)
///
pub struct QuoteKeyword {
}

impl QuoteKeyword {
    ///
    /// Creates the `(quote thing)` syntax
    ///
    pub fn new() -> SyntaxCompiler {
        unimplemented!()
    }
}

impl BindingMonad for QuoteKeyword {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##quote##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        let args                    = bindings.args.clone().unwrap_or_else(|| SafasCell::Nil.into());
        let SafasList(car, _cdr)    = SafasList::try_from(args).unwrap_or(SafasList::nil());

        (bindings, Ok(smallvec![Action::Value(car)]))
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