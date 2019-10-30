use crate::bind::*;
use crate::meta::*;

///
/// Implements the `(export foo)` symbol, which lifts a symbol out of the current compilation context into the parent context
/// 
/// Most commonly used in files intended to be loaded by `import`, where this can be used to specify the symbols that are
/// available outside of that file.
///
pub fn export_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    get_expression_arguments().and_then(|ListTuple((atom, )): ListTuple<(AtomId, )>| {
        // All the binding does is add to the export list for the current bindings
        let AtomId(atom_id) = atom;

        BindingFn::from_binding_fn(move |bindings| {
            let mut bindings = bindings;

            // Look up the symbol value
            let symbol_value = bindings.look_up(atom_id);

            if let Some((symbol_value, depth)) = symbol_value {
                if depth != 0 {
                    // Must be exporting something from the current frame (ie, not require importing)
                    (bindings, Err(BindError::SymbolNotDefinedLocally(name_for_atom_with_id(atom_id))))
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

    }).map(|_| {
        // 'Export' just adds to the bindings and performs no actual compilation
        SyntaxCompiler::default()
    })
}

///
/// Re-export keyword: takes a statement and exports everything that it exports
/// 
/// Typically used with import: `(re_export (import "some_library"))`
///
pub fn re_export_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    get_expression_arguments().and_then(|ListTuple((expr, )): ListTuple<(CellRef, )>| {

        BindingFn::from_binding_fn(move |bindings| {
            // Create an interior frame
            let inner_bindings                  = bindings.push_interior_frame();

            // Bind our expression to it
            let (bound_expr, inner_bindings)    = match bind_statement(expr.clone(), inner_bindings) {
                Ok((bound_expr, inner_bindings))    => (bound_expr, inner_bindings),
                Err((err, inner_bindings))          => return (inner_bindings.pop().0, Err(err))
            };

            // Export all of the symbols defined in the inner bindings (these will be the ones exported from whatever expression was evaluated)
            let re_exports                  = inner_bindings.symbols.iter()
                .map(|(atom_id, value)| (*atom_id, value.clone()))
                .collect::<Vec<_>>();

            // Pop the interior frame
            let (mut bindings, _imports)    = inner_bindings.pop();

            // Re-export all of the symbols from our inner frame
            for (atom_id, value) in re_exports {
                // Add the symbol to our bindings
                bindings.symbols.insert(atom_id, value);

                // Export from the parent context
                bindings.export_from_parent(atom_id);
            }

            // Result is the expression we just bound
            (bindings, Ok(bound_expr))
        })

    }).map(|expr| {
        let expr = expr.clone();

        // Expression is just compiled as normal
        SyntaxCompiler::with_compiler(|expr| compile_statement(expr), expr)
    })
}
