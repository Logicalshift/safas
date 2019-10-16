use super::bitcode_monad::*;
use super::bitcode_functions::*;

use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use std::convert::*;
use std::sync::*;
use std::iter;

lazy_static! {
    /// The wrap_value function
    static ref WRAP_VALUE: CellRef = wrap_value();

    /// The read_label_value function
    static ref READ_LABEL_VALUE: CellRef = read_label_value();
}

///
/// Creates the 'alloc_label' bitcode monad as a cell
///
fn alloc_label(label_id: usize) -> CellRef {
    let alloc_label = BitCodeMonad::alloc_label(label_id);
    let alloc_label = SafasCell::Any(Box::new(alloc_label)).into();     
    let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

    SafasCell::Monad(alloc_label, monad_type).into()
}

///
/// Creates a 'read label value' flat_map function
///
fn read_label_value() -> CellRef {
    let read_label_value = FnMonad::from(|args: BitCodeFlatMapArgs<SafasNumber>| {
        let label_id    = args.value;

        let label_value = BitCodeMonad::read_label_value(label_id.to_usize());
        let label_value = SafasCell::Any(Box::new(label_value)).into();     

        let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

        SafasCell::Monad(label_value, monad_type).into()
    });
    let read_label_value = ReturnsMonad(read_label_value);
    let read_label_value = SafasCell::FrameMonad(Box::new(read_label_value));

    read_label_value.into()
}

///
/// Creates the 'wrap_value' function as a cell
///
fn wrap_value() -> CellRef {
    let wrap_value = FnMonad::from(|args: BitCodeFlatMapArgs<CellRef>| {
        let wrapped     = BitCodeMonad::with_value(args.value);
        let wrapped     = SafasCell::Any(Box::new(wrapped)).into();
        let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

        SafasCell::Monad(wrapped, monad_type).into()
    });
    let wrap_value = ReturnsMonad(wrap_value);
    let wrap_value = SafasCell::FrameMonad(Box::new(wrap_value));

    wrap_value.into()
}

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

                // TODO: check that we've got the reference we allocated in the pre-binding (if it's been rebound the label is invalid)

                let result = SafasCell::list_with_cells(iter::once(reference.into()));
                (bindings, Ok(result))
            },

            move |mut bindings| {
                // TODO: the label can only be pre-bound once: check that the value has not already been bound

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
        // Results of the bindings is the cell reference
        let ListTuple((cell_reference, )): ListTuple<(CellRef, )> = value.try_into()?;

        let (cell_id, frame_num, _) = cell_reference.frame_reference().ok_or(BindError::MissingArgument)?;
        if frame_num != 0 { return Err(BindError::MissingArgument); }

        // Start generating the actions
        let mut actions = CompiledActions::empty();

        // Frame setup allocates the label. We use the cell ID as the label ID for updating it later
        // TODO: and reads its value into the cell (we're just loading the label ID at the moment)
        actions.frame_setup.extend(vec![
            Action::Value(alloc_label(cell_id)),
            Action::Push,
            Action::Value(READ_LABEL_VALUE.clone()),
            Action::FlatMap,
            Action::StoreCell(cell_id)
        ]);

        // Loading the value just loads from the label
        actions.actions.extend(vec![
            Action::CellValue(cell_id)
        ]);

        Ok(actions)
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
