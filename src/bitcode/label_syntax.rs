use super::bitcode_monad::*;
use super::bitcode_functions::*;

use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::convert::*;
use std::sync::*;
use std::iter;

lazy_static! {
    /// The alloc_label bitcode monad
    static ref ALLOC_LABEL: CellRef = alloc_label();

    /// The wrap_value flat_map function (reads a value from a monad and stores it)
    static ref WRAP_VALUE: CellRef = wrap_value();

    /// The read_label_value flat_map function
    static ref READ_LABEL_VALUE: CellRef = read_label_value();

    /// The set_label_value flat_map function
    static ref SET_LABEL_VALUE: CellRef = set_label_value();
}

///
/// Creates the 'alloc_label' bitcode monad as a cell
///
fn alloc_label() -> CellRef {
    // Basic alloc_label monad
    let alloc_label = BitCodeMonad::alloc_label();

    // Stuff into a cell with the any mapping
    let alloc_label = SafasCell::Any(Box::new(alloc_label)).into();

    // Monad type is the flat_map method (which expects the 'Any' cell defined above)
    let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

    SafasCell::Monad(alloc_label, monad_type).into()
}

///
/// Creates a 'read label value' flat_map function
///
fn read_label_value() -> CellRef {
    let read_label_value = FnMonad::from(|args: BitCodeFlatMapArgs<CellRef>| {
        let label_id    = args.value;

        let label_value = BitCodeMonad::read_label_value(label_id);
        let label_value = SafasCell::Any(Box::new(label_value)).into();     

        let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

        SafasCell::Monad(label_value, monad_type).into()
    });
    let read_label_value = ReturnsMonad(read_label_value);
    let read_label_value = SafasCell::FrameMonad(Box::new(read_label_value));

    read_label_value.into()
}

///
/// Creates a 'set label value' flat_map function
///
fn set_label_value() -> CellRef {
    let read_label_value = FnMonad::from(|args: BitCodeFlatMapArgs<CellRef>| {
        let label_id    = args.value;

        let bit_pos         = BitCodeMonad::read_bit_pos();
        let read_and_set    = bit_pos.flat_map(move |bit_pos| Ok(BitCodeMonad::set_label_value(label_id.clone(), bit_pos))).unwrap();
        let read_and_set    = SafasCell::Any(Box::new(read_and_set)).into();

        let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

        SafasCell::Monad(read_and_set, monad_type).into()
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
/// A label binding is attached to labels when they're pre-bound and will evaluate to the label's value
///
struct LabelBinding(FrameReference);

impl BindingMonad for LabelBinding {
    type Binding = CellRef;

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, CellRef) {
        (bindings, NIL.clone())
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<CellRef, BindError>) {
        // This is expected to be used as like a variable
        if !bindings.args.is_none() { return (bindings, Err(BindError::ConstantsCannotBeCalled)); }

        // Binds to the frame reference
        let LabelBinding(reference) = self;
        (bindings, Ok((*reference).into()))
    }

    fn reference_type(&self, _bound_value: CellRef) -> ReferenceType {
        ReferenceType::Monad
    }

    fn rebind_from_outer_frame(&self, bindings: SymbolBindings, frame_depth: u32) -> (SymbolBindings, Option<Box<dyn BindingMonad<Binding=Self::Binding>>>) {
        // Nothing to do if the frame depth is 0
        if frame_depth == 0 { return (bindings, None); }

        // Fetch the current reference (our frame ID will be 0 here but we're being imported from frame_depth)
        let LabelBinding(FrameReference(outer_cell_id, _, _)) = self;

        // Import into a local cell
        let mut bindings    = bindings;
        let local_cell_id   = bindings.alloc_cell();
        let outer_cell      = SafasCell::FrameReference(*outer_cell_id, frame_depth, ReferenceType::Monad).into();
        let inner_cell      = FrameReference(local_cell_id, 0, ReferenceType::Monad);
        bindings.import(outer_cell, local_cell_id);

        // Create a new syntax item
        (bindings, Some(Box::new(LabelBinding(inner_cell))))
    }
}

///
/// Creates the syntax binding for a label name
/// 
/// This generates a monad to load the label value when it's used.
///
fn label_binding(label_cell: FrameReference) -> SyntaxCompiler {
    // The label binding, which specifies which cell the compiler should load from
    let bind = LabelBinding(label_cell);

    // Compiler receives the label reference as an argument and flat_maps it
    let compile = |args: CellRef| {
        // Args should just be a frame reference generated by the binding operation
        let args                                    = FrameReference::try_from(args)?;
        let FrameReference(cell_id, frame_id, _)    = args;

        if frame_id != 0 {
            return Err(BindError::CannotLoadCellInOtherFrame);
        }

        // Compilation reads the frame reference and applies the read_label_value flat map function
        Ok(CompiledActions::from(smallvec![
            Action::CellValue(cell_id),
            Action::Push,
            Action::Value(READ_LABEL_VALUE.clone()),
            Action::FlatMap
        ]))
    };

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compile)
    }
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
        .and_then(|args: ListTuple<(AtomId, )>| {
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

                // Labels are bound to their own syntax item, which reads the label value when used
                let label_cell      = bindings.alloc_cell();
                let label_reference = FrameReference(label_cell, 0, ReferenceType::Monad);
                let label_action    = label_binding(label_reference);
                bindings.symbols.insert(atom_id, SafasCell::ActionMonad(label_action, label_reference.into()).into());
                bindings.export(atom_id);

                // Result is just the atom as for the main binding function
                let result = SafasCell::list_with_cells(iter::once(SafasCell::Atom(atom_id).into()));
                (bindings, result)
            })
        });
    let bind = WithReferenceType(bind, ReferenceType::Monad);

    // Compiling function: labels bind themselves to a monad that allocates/retrieves the label value at the start of the code block and just bind to the label value later on 
    let compiler = |value: CellRef| -> Result<_, BindError> {
        // Results of the bindings is the cell reference
        let ListTuple((label_action, )): ListTuple<(CellRef, )> = value.try_into()?;

        // The label should be bound to an action monad, with the frame cell as the parameter
        let cell_reference = match &*label_action { SafasCell::ActionMonad(_, cell_reference) => Ok(cell_reference.clone()), _ => Err(BindError::MissingArgument) }?;

        // Fetch out the frame reference
        let (cell_id, frame_num, _) = cell_reference.frame_reference().ok_or(BindError::MissingArgument)?;
        if frame_num != 0 { return Err(BindError::CannotLoadCellInOtherFrame); }

        // Start generating the actions
        let mut actions = CompiledActions::empty();

        // Frame setup allocates the label. We use the cell ID as the label ID for updating it later
        actions.frame_setup.extend(vec![
            Action::Value(ALLOC_LABEL.clone()),
            Action::Push,
            Action::Value(WRAP_VALUE.clone()),
            Action::FlatMap,
            Action::StoreCell(cell_id)
        ]);

        // To evaluate the label syntax itself, we fetch the label and flat_map via SET_LABEL_VALUE
        actions.actions.extend(vec![
            Action::CellValue(cell_id),
            Action::Push,
            Action::Value(SET_LABEL_VALUE.clone()),
            Action::FlatMap
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
    use crate::bitcode::*;

    #[test]
    fn define_basic_label() {
        let result          = eval("((fun () (label foo) foo))").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();
        println!("{}", val.to_string());

        assert!(val.to_string() == "$0u64".to_string());
    }
}
