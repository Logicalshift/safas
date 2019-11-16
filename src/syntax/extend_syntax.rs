use super::syntax_symbol::*;
use super::syntax_closure::*;
use super::pattern_match::*;

use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

///
/// `(extend_syntax existing_syntax new_syntax_name (<pattern> <macro> ...) [prelude_statements])`
/// 
/// Takes an existing syntax (anything that binds the `syntax` keyword to a btree) and extends it with a new syntax
///
pub fn extend_syntax_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    get_expression_arguments().and_then(|ListWithTail((existing_syntax_name, new_name, patterns), statements): ListWithTail<(AtomId, AtomId, CellRef), CellRef>| {

        BindingFn::from_binding_fn(move |bindings| {
            
            // Look up the existing syntax
            let AtomId(existing_syntax_id)  = existing_syntax_name;
            let existing_syntax             = CellRef::new(SafasCell::Atom(existing_syntax_id));

            // Attempt to bind the 'syntax' atom
            let syntax_atom                 = CellRef::new(SafasCell::Atom(get_id_for_atom_with_name("syntax")));
            let bind_syntax                 = SafasCell::list_with_cells(vec![existing_syntax, syntax_atom]);

            let (bindings, existing_syntax) = match bind_statement(bind_syntax.into(), bindings) {
                Ok((existing_syntax, bindings)) => (bindings, existing_syntax),
                Err((err, bindings))            => return (bindings, Err(err))
            };
            
            // Return the result
            (bindings, Ok((existing_syntax, new_name, patterns.clone(), statements.clone())))
        })

    })
    .map_result(|_| {
        Ok(SyntaxCompiler::with_compiler(|_| Ok(CompiledActions::empty()), NIL.clone()))
    })
}
