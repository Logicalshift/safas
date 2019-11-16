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
            let existing_syntax             = bindings.look_up(existing_syntax_id);

            let existing_syntax             = match existing_syntax {
                None                                    => return (bindings, Err(BindError::UnknownSymbol(name_for_atom_with_id(existing_syntax_id)))),
                Some((existing_syntax, frame_depth))    => {
                    // TODO: Rebind if necessary
                    existing_syntax
                }
            };

            // For syntax items, the parameter contains a btree with syntax bindings in it
            let syntax_items = match &*existing_syntax {
                SafasCell::Syntax(_binding, params) => {
                    match btree_search(params.clone(), SafasCell::atom("syntax")) {
                        Ok(syntax_items)    => syntax_items,
                        Err(_)              => return (bindings, Err(BindError::CannotExtendSyntax(name_for_atom_with_id(existing_syntax_id))))
                    }
                },

                _ => return (bindings, Err(BindError::CannotExtendSyntax(name_for_atom_with_id(existing_syntax_id))))
            };

            if syntax_items.is_nil() {
                return (bindings, Err(BindError::CannotExtendSyntax(name_for_atom_with_id(existing_syntax_id))))
            }

            println!("{:?}", syntax_items);

            // Return the result
            (bindings, Ok((syntax_items, new_name, patterns.clone(), statements.clone())))
        })

    })
    .map_result(|_| {
        Ok(SyntaxCompiler::with_compiler(|_| Ok(CompiledActions::empty()), NIL.clone()))
    })
}
