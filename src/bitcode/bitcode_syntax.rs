use crate::bind::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;
use std::iter;

///
/// The `label` keyword creates a bitcode monad that specifies a label
/// 
/// Label values are available everywhere in the same context (and may be passed outside 
/// of that context as separate values if necessary): note that 'forward declaration' of
/// labels are specifically allowed via the pre-binding mechanism.
///
pub fn label_keyword() -> SyntaxCompiler {
    // Binding function. Labels are pre-bound so they're available throughout the current context
    let bind = get_expression_arguments()
        .and_then_ok(|args: ListTuple<(AtomId, )>| {
            // Parse out the arguments
            let ListTuple((AtomId(atom_id), )) = args;

            BindingFn(move |bindings| {
                // Binding function (just the atom that's assigned to this label)
                let reference = bindings.look_up(atom_id);
                let reference = match reference { Some((reference, 0)) => reference.clone(), _ => return (bindings, Err(BindError::UnknownSymbol)) };

                let result = SafasCell::list_with_cells(iter::once(reference.into()));
                (bindings, Ok(result))
            },

            move |mut bindings| {
                // Pre-binding function (bind the atom to a proto-label)
                let label_cell = bindings.alloc_cell();
                bindings.symbols.insert(atom_id, SafasCell::FrameReference(label_cell, 0, ReferenceType::Monad).into());
                bindings.export(atom_id);

                // Result is just the atom as for the main binding function
                let result = SafasCell::list_with_cells(iter::once(SafasCell::Atom(atom_id).into()));
                (bindings, result)
            })
        });

    // Compiling function: labels bind themselves to a monad that allocates/retrieves the label value at the start of the code block and just bind to the label value later on 
    let compiler = |value: CellRef| -> Result<_, BindError> {
        // TODO
        Ok(smallvec![])
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
    fn define_basic_label() {
        let val = eval("((fun () (label foo) foo))").unwrap();
    }
}
