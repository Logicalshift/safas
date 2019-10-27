use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use std::sync::*;

///
/// Implements the `(export foo)` symbol, which lifts a symbol out of the current compilation context into the parent context
/// 
/// Most commonly used in files intended to be loaded by `import`, where this can be used to specify the symbols that are
/// available outside of that file.
///
pub fn export_keyword() -> SyntaxCompiler {
    let bind = get_expression_arguments().and_then(|ListTuple((atom, )): ListTuple<(AtomId, )>| {
        // All the binding does is add to the export list for the current bindings
        let AtomId(atom_id) = atom;

        BindingFn::from_binding_fn(move |bindings| {
            let mut bindings = bindings;
            bindings.export(atom_id);

            (bindings, Ok(NIL.clone()))
        })

    });

    // No actions for the compilation
    let compile = |_| Ok(CompiledActions::empty());

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compile)
    }
}
