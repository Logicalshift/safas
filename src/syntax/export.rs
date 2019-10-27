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

            // Look up the symbol value
            let symbol_value = bindings.look_up(atom_id);

            if let Some((symbol_value, depth)) = symbol_value {
                if depth != 0 {
                    // Must be exporting something from the current frame (ie, not require importing)
                    (bindings, Err(BindError::NotInfallible))
                } else {
                    // Add to the 'local' symbols
                    bindings.symbols.insert(atom_id, symbol_value);

                    // Export to the parent binding
                    bindings.export_from_parent(atom_id);

                    (bindings, Ok(NIL.clone()))
                }
            } else {
                (bindings, Err(BindError::UnknownSymbol(name_for_atom_with_id(atom_id))))
            }
        })

    });

    // No actions for the compilation
    let compile = |_| Ok(CompiledActions::empty());

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compile)
    }
}
