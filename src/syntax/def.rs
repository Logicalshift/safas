use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;
use std::convert::*;

///
/// Creates the syntax definition for the 'def' keyword
/// 
/// ```(def <name> <value>)```
/// 
/// Assigns `<value>` to the atom called `<name>`. It is possible to redefine an existing atom. The value
/// will be available to everything in the same frame after this statement.
///
pub fn def_keyword() -> SyntaxCompiler {
    // Create the binding expression
    // 
    //   This retrieves the arguments, binds the value, allocates a cell and associates the name with 
    //   the cell and generates a list containing (cell_binding, value_binding) to pass to the compiler
    //   
    let bind = get_expression_arguments().and_then_ok(|args: ListTuple<(AtomId, CellRef)>| {
        // The arguments are just the name and the value
        let ListTuple((name, value)) = args;

        // Bind the value
        bind(value).and_then_ok(move |value| {
            // Allocate the cell to store the value in
            allocate_cell().and_then(move |cell_id| {
                // Define the symbol to map to this cell
                let value           = value.clone();
                let cell_type       = value.reference_type();
                let cell: CellRef   = SafasCell::FrameReference(cell_id, 0, cell_type).into();

                define_symbol_value(name, cell.clone()).and_then_ok(move |_| {
                    let value   = value.clone();
                    let cell    = cell.clone();

                    // Export the atom into the environment
                    export_symbol(name).and_then(move |_| {
                        // Binding contains the frame reference cell and the bound value
                        wrap_binding(SafasCell::list_with_cells(vec![cell.clone(), value.clone()]))
                    })
                })
            })
        })
    });

    // Create the action compiler (load the value and store in the cell)
    let compiler = |value: CellRef| -> Result<_, BindError> {
        // Fetch the frame reference and the bound value from the value
        let bound_values: ListTuple<(FrameReference, CellRef)>  = value.try_into()?;
        let ListTuple((FrameReference(cell, _, _), value))      = bound_values;

        // Compile the actions to generate the value
        let mut actions                                         = compile_statement(value)?;

        // Store in the cell
        actions.push(Action::StoreCell(cell));

        Ok(actions)
    };

    SyntaxCompiler {
        binding_monad:      Box::new(bind),
        generate_actions:   Arc::new(compiler)
    }
}

///
/// Monad that defines an atom to be a particular value
///
struct DefineSymbol {
    atom_id:    u64,
    value:      CellRef
}

impl BindingMonad for DefineSymbol {
    type Binding=SmallVec<[Action; 8]>;

    fn description(&self) -> String { "##define_symbol##".to_string() }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        (bindings, smallvec![])
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        // Allocate a cell for this binding
        let mut bindings    = bindings;
        let cell_id         = bindings.alloc_cell();
        let cell_type       = self.value.reference_type();
        bindings.symbols.insert(self.atom_id, SafasCell::FrameReference(cell_id, 0, cell_type).into());

        // Actions just load the binding into the cell
        let actions         = smallvec![Action::Value(Arc::clone(&self.value)), Action::StoreCell(cell_id)];

        (bindings, Ok(actions))
    }
}

///
/// Creates a binding monad that defines a symbol to evaluate a particular cell value
///
pub fn define_symbol<Atom: Into<AtomId>, Cell: Into<CellRef>>(atom: Atom, value: Cell) -> impl BindingMonad<Binding=SmallVec<[Action; 8]>> {
    // Retrieve the atom ID
    let atom_id         = atom.into();
    let AtomId(atom_id) = atom_id;

    DefineSymbol {
        atom_id:    atom_id,
        value:      value.into()
    }
}

///
/// Monad that defines an atom to be a particular value
///
struct DefineSymbolValue {
    atom_id:    u64,
    value:      CellRef
}

impl BindingMonad for DefineSymbolValue {
    type Binding=SmallVec<[Action; 8]>;

    fn description(&self) -> String { "##define_symbol##".to_string() }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        (bindings, smallvec![])
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        // Store the value for this symbol
        let mut bindings    = bindings;
        bindings.symbols.insert(self.atom_id, self.value.clone());

        // No actions are performed for this: the symbol is just defined
        let actions         = smallvec![];

        (bindings, Ok(actions))
    }
}

///
/// Creates a binding monad that defines a symbol to evaluate a particular cell value
///
pub fn define_symbol_value<Atom: Into<AtomId>, Cell: Into<CellRef>>(atom: Atom, value: Cell) -> impl BindingMonad<Binding=SmallVec<[Action; 8]>> {
    // Retrieve the atom ID
    let atom_id         = atom.into();
    let AtomId(atom_id) = atom_id;

    DefineSymbolValue {
        atom_id:    atom_id,
        value:      value.into()
    }
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn define_and_read_atom() {
        let val = eval("(def x 1) x").unwrap().0.to_string();
        assert!(val == "1".to_string());
    }

    #[test]
    fn define_multiple_atoms() {
        let val = eval("(def x 1) (def y 2) x").unwrap().0.to_string();
        assert!(val == "1".to_string());

        let val = eval("(def x 1) (def y 2) y").unwrap().0.to_string();
        assert!(val == "2".to_string());
    }
}
