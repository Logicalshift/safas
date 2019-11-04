use super::bitcode_monad::*;
use super::bitcode_functions::*;

use crate::bind::*;
use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::convert::*;
use std::iter;

lazy_static! {
    /// The alloc_label bitcode monad
    static ref ALLOC_LABEL: CellRef = alloc_label();

    /// The wrap_value flat_map function (reads a value from a monad and stores it)
    static ref WRAP_VALUE: CellRef = wrap_value();

    /// The read_label_value flat_map function
    static ref READ_LABEL_VALUE: CellRef = read_label_value();

    /// Function that creates a set_label_value flat_map function given the label
    static ref CREATE_SET_LABEL_VALUE: CellRef = create_set_label_value();

    /// The read_bit_pos flat_map function
    static ref READ_BIT_POS: CellRef = read_bit_pos();

    /// The ID of the atom containing the standard bit-position to label value function
    static ref LABEL_VALUE_FUNCTION: u64 = get_id_for_atom_with_name("label_value");

    /// A function that takes a value mapping function and returns a bitcode flat_map function
    static ref MAP_TO_FLAT_MAP: CellRef = map_to_flat_map_fn();
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
    // Called on a monad that will return a label ID cell
    let read_label_value = FnMonad::from(|(label_id, ): (CellRef, )| {
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
/// Creates a 'read bit position' flat_map function
///
fn read_bit_pos() -> CellRef {
    let read_bit_pos = FnMonad::from(|(_, ): (CellRef, )| {
        let bit_pos     = BitCodeMonad::read_bit_pos();
        let bit_pos     = SafasCell::Any(Box::new(bit_pos)).into();

        let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

        SafasCell::Monad(bit_pos, monad_type).into()
    });
    let read_bit_pos    = ReturnsMonad(read_bit_pos);
    let read_bit_pos    = SafasCell::FrameMonad(Box::new(read_bit_pos));

    read_bit_pos.into()
}

///
/// Creates a 'set label value' closure function (this is a function that receives a label ID and returns a flat_map function that sets that label)
///
fn create_set_label_value() -> CellRef {
    // Called on a monad that will return a label ID cell
    let create_set_label_flat_map = FnMonad::from(|(label_id_monad, ): (CellRef, )| {
        let label_id = BitCodeMonad::from_cell(&label_id_monad).expect("Label ID is not a monad");

        // Create the flat_map function
        let set_label_value = FnMonad::from(move |(label_value, ): (CellRef, )| {
            // Flat_map to store the position read by the bit_pos monad
            let set_label_value = label_id.clone().flat_map(move |label_id| { 
                Ok(BitCodeMonad::set_label_value(label_id, label_value.clone())) 
            }).expect("Failed to map set_label_value");
            let set_label_value = SafasCell::Any(Box::new(set_label_value)).into();

            let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

            SafasCell::Monad(set_label_value, monad_type).into()
        });
        let set_label_value = ReturnsMonad(set_label_value);

        // Return as a framemonad
        SafasCell::FrameMonad(Box::new(set_label_value)).into()
    });

    let create_set_label_flat_map = SafasCell::FrameMonad(Box::new(create_set_label_flat_map));

    create_set_label_flat_map.into()
}

///
/// Creates the 'wrap_value' function as a cell
///
fn wrap_value() -> CellRef {
    // Creates a with_value bitcode monad from any value
    let wrap_value = FnMonad::from(|(value, ): (CellRef, )| {
        let wrapped     = BitCodeMonad::with_value(value);
        let wrapped     = SafasCell::Any(Box::new(wrapped)).into();
        let monad_type  = MonadType::new(BITCODE_FLAT_MAP.clone());

        SafasCell::Monad(wrapped, monad_type).into()
    });
    let wrap_value = ReturnsMonad(wrap_value);
    let wrap_value = SafasCell::FrameMonad(Box::new(wrap_value));

    wrap_value.into()
}

struct MapAndWrap(pub CellRef);

impl FrameMonad for MapAndWrap {
    type Binding = RuntimeResult;

    fn execute(&self, frame: Frame) -> (Frame, RuntimeResult) {
        // Call the mapping function (we're called with a value parameter, which will be passed on to the mapping function here)
        let map_fn          = &self.0;
        let (frame, result) = match &**map_fn {
            SafasCell::FrameMonad(map_fn)   => map_fn.execute(frame),
            _                               => (frame, Err(RuntimeError::NotAFunction(map_fn.clone())))
        };

        // Wrap as a bitcode monad value
        let result          = match result { Ok(result) => result, Err(err) => return (frame, Err(err)) };
        let result          = BitCodeMonad::with_value(result);
        let result          = SafasCell::Any(Box::new(result)).into();
        let monad_type      = MonadType::new(BITCODE_FLAT_MAP.clone());

        (frame, Ok(SafasCell::Monad(result, monad_type).into()))
    }

    fn description(&self) -> String { format!("(map_and_wrap {})", self.0.to_string()) }

    fn returns_monad(&self) -> bool { true }
}

///
/// Creates a `map_to_flat_map` function, which receives a function `a -> b` and returns a function `a -> BitCodeMonad b`
///
fn map_to_flat_map_fn() -> CellRef {
    let map_to_flat_map_fn = FnMonad::from(|(map_fn, ): (CellRef, )| {
        let map_and_wrap = MapAndWrap(map_fn);
        SafasCell::FrameMonad(Box::new(map_and_wrap)).into()
    });

    SafasCell::FrameMonad(Box::new(map_to_flat_map_fn)).into()
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
fn label_binding(label_cell: FrameReference) -> impl BindingMonad<Binding=SyntaxCompiler> {
    // The label binding, which specifies which cell the compiler should load from
    LabelBinding(label_cell).map(|args| {
        let args = args.clone();

        // Compiler receives the label reference as an argument and flat_maps it
        let compile = |args: CellRef| {
            // Args should just be a frame reference generated by the binding operation
            let args                                    = FrameReference::try_from(args.clone())?;
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

        SyntaxCompiler::with_compiler_and_reftype(compile, args, ReferenceType::Monad)
    })
}

///
/// The `label` keyword creates a bitcode monad that specifies a label
/// 
/// Label values are available everywhere in the same context (and may be passed outside 
/// of that context as separate values if necessary): note that 'forward declaration' of
/// labels are specifically allowed via the pre-binding mechanism.
///
pub fn label_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    // Binding function. Labels are pre-bound so they're available throughout the current context
    let bind = get_expression_arguments()
        .and_then(|args: ListTuple<(AtomId, )>| {
            // Parse out the arguments
            let ListTuple((AtomId(atom_id), )) = args;

            BindingFn(move |mut bindings| {
                // Fetch the label value function
                let label_value_fn = if let Some(label_value) = bindings.look_up_and_import(*LABEL_VALUE_FUNCTION) {
                    label_value
                } else {
                    NIL.clone()
                };

                // Fetch the value assigned to the atom that represents the label
                let reference = bindings.look_up(atom_id);
                let reference = match reference { Some((reference, 0)) => reference.clone(), _ => return (bindings, Err(BindError::UnknownSymbol(name_for_atom_with_id(atom_id)))) };

                // TODO: check that we've got the reference we allocated in the pre-binding (if it's been rebound the label is invalid)

                let result = SafasCell::list_with_cells(vec![reference.into(), label_value_fn]);
                (bindings, Ok(result))
            },

            move |mut bindings| {
                // TODO: the label can only be pre-bound once: check that the value has not already been bound

                // Labels are bound to their own syntax item, which reads the label value when used
                let label_cell      = bindings.alloc_cell();
                let label_reference = FrameReference(label_cell, 0, ReferenceType::Monad);
                let label_action    = label_binding(label_reference);
                bindings.symbols.insert(atom_id, SafasCell::Syntax(Box::new(label_action), label_reference.into()).into());
                bindings.export(atom_id);

                // Result is just the atom as for the main binding function
                let result = SafasCell::list_with_cells(iter::once(SafasCell::Atom(atom_id).into()));
                (bindings, result)
            })
        }).map(|value| {
            let value = value.clone();

            // Compiling function: labels bind themselves to a monad that allocates/retrieves the label value at the start of the code block and just bind to the label value later on 
            let compiler = |value: CellRef| -> Result<_, BindError> {
                // Results of the bindings is the cell reference
                let ListTuple((label_action, calculate_value)): ListTuple<(CellRef, CellRef)> = value.clone().try_into()?;

                // The label should be bound to a syntax item, with the frame cell as the parameter
                let cell_reference = match &*label_action { SafasCell::Syntax(_, cell_reference) => Ok(cell_reference.clone()), _ => Err(BindError::MissingArgument) }?;

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

                // Evaluate the value calculation
                let mut value_reference_type = calculate_value.reference_type();
                if calculate_value.is_nil() {
                    // Read the current bit position
                    value_reference_type = ReferenceType::ReturnsMonad;
                    actions.actions.extend(vec![
                        Action::Value(READ_BIT_POS.clone()),
                    ])
                } else {
                    // Calculate a value
                    if value_reference_type == ReferenceType::Value {
                        // Create the bit_pos monad on the stack
                        actions.actions.extend(vec![
                            // Bit_pos monad
                            Action::PushValue(READ_BIT_POS.clone()),
                            Action::PushValue(NIL.clone()),
                            Action::PopCall(1),
                            Action::Push,

                            // Function conversion routine (we call this to generate a flatmap method, then call flatmap on the monad we just created)
                            Action::PushValue(MAP_TO_FLAT_MAP.clone())
                        ])
                    }

                    // Get the value of the label_value function
                    actions.extend(compile_statement(calculate_value)?);
                }

                // Map the value reference type
                match value_reference_type {
                    ReferenceType::Value        => actions.actions.extend(vec![Action::Push, Action::PopCall(1), Action::FlatMap]),
                    ReferenceType::ReturnsMonad => actions.actions.extend(vec![Action::Push, Action::PushValue(NIL.clone()), Action::PopCall(1)]),
                    ReferenceType::Monad        => { }
                }

                // To evaluate the label syntax itself, we fetch the label and flat_map via SET_LABEL_VALUE
                actions.actions.extend(vec![
                    // Push the monad containing the label value
                    Action::Push,

                    // Call the closure that generates the set_label_value function from the cell value monad
                    Action::PushValue(CREATE_SET_LABEL_VALUE.clone()),
                    Action::PushCell(cell_id),
                    Action::PopCall(1),

                    // FlatMap the result with the label value monad we pushed earlier
                    Action::FlatMap
                ]);

                Ok(actions)
            };

            SyntaxCompiler::with_compiler_and_reftype(compiler, value, ReferenceType::Monad)
        });

    WithReferenceType(bind, ReferenceType::Monad)
}

#[cfg(test)]
mod test {
    use crate::interactive::*;
    use crate::bitcode::*;
    use crate::meta::*;
    use crate::bind::*;
    use crate::exec::*;
    use crate::syntax::*;
    use crate::parse::*;
    use crate::functions::*;

    use std::sync::*;

    fn bind_expr(expr: &str) -> Result<CellRef, RuntimeError> {
        // Create the execution frame
        let bindings                = SymbolBindings::new();

        // Apply the standard bindings
        let syntax                  = standard_syntax();
        let functions               = standard_functions();
        let (bindings, _actions)    = syntax.bind(bindings);
        let (bindings, _fn_actions) = functions.bind(bindings);

        let mut bindings            = bindings;

        // Parse the expression
        let expr = parse_safas(&mut TokenReadBuffer::new(expr.chars()), FileLocation::new("<expr>"))?;

        // Pre-bind the statements
        let mut statement   = Arc::clone(&expr);
        while let SafasCell::List(car, cdr) = &*statement {
            let (new_bindings, _)   = pre_bind_statement(Arc::clone(&car), bindings);
            bindings                = new_bindings;
            statement               = Arc::clone(&cdr);
        }

        // Bind the statements (last one is the result)
        let mut statement   = Arc::clone(&expr);
        let mut result      = NIL.clone();

        while let SafasCell::List(car, cdr) = &*statement {
            let (bound, new_bindings)   = match bind_statement(Arc::clone(&car), bindings) { Ok((bound, new_bindings)) => (bound, new_bindings), Err((err, _new_bindings)) => return Err(err.into()) };

            result                      = bound;
            bindings                    = new_bindings;

            statement                   = Arc::clone(&cdr);
        }
        
        Ok(result)
    }

    #[test]
    fn data_expr_is_a_monad() {
        let data_expr = bind_expr("(d 1)").unwrap();
        assert!(data_expr.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn label_is_a_monad() {
        let label_expr = bind_expr("(label foo)").unwrap();
        assert!(label_expr.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn label_value_is_a_monad() {
        let label_expr = bind_expr("(label foo) foo").unwrap();
        assert!(label_expr.reference_type() == ReferenceType::Monad);
    }

    #[test]
    fn define_basic_label() {
        let result          = eval("(label foo) foo").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();

        assert!(val.to_string() == "$0u64".to_string());
    }

    #[test]
    fn define_basic_label_in_list() {
        let result          = eval("(label foo) (list foo)").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();

        assert!(val.to_string() == "($0u64)".to_string());
    }

    #[test]
    fn label_reads_bit_position() {
        let result          = eval("(d 5u8) (label foo) foo").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();

        assert!(val.to_string() == "$8u64".to_string());
    }

    #[test]
    fn label_uses_label_value_returnsmonad_function() {
        let result          = eval("
            (def label_value 
                (fun (_) 
                    (* (bit_pos) 8)
                )
            )

            (d 5u8) (label foo) foo"
        ).unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();
        println!("{:?}", val.to_string());

        assert!(val.to_string() == "$40u64".to_string());
    }

    #[test]
    fn label_uses_label_value_monad() {
        let result          = eval("
            (def ip (* (bit_pos) 8))
            (def label_value ip)

            (d 5u8) (label foo) foo"
        ).unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();
        println!("{:?}", val.to_string());

        assert!(val.to_string() == "$40u64".to_string());
    }

    #[test]
    fn label_uses_label_value_value_function() {
        let result          = eval("
            (def label_value 
                (fun (cur_bit_pos) 
                    (* cur_bit_pos 8)
                )
            )

            (d 5u8) (label foo) foo"
        ).unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();
        println!("{:?}", val.to_string());

        assert!(val.to_string() == "$40u64".to_string());
    }

    #[test]
    fn label_requiring_multiple_passes_1() {
        let result          = eval("(d foo) (label foo) foo").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();

        // Labels are 64-bits so we should end up with a label position of 64 here
        assert!(val.to_string() == "$40u64".to_string());
    }

    #[test]
    fn label_requiring_multiple_passes_2() {
        let result          = eval("(d (bits 32 foo)) (label foo) foo").unwrap();
        let monad           = BitCodeMonad::from_cell(&result).unwrap();

        let (val, _bitcode) = assemble(&monad).unwrap();

        // Cut down to 32 bits, so we end up with a label position of 32
        assert!(val.to_string() == "$20u64".to_string());
    }
}
