use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;

///
/// The monad for the 'def' syntax (def atom value)
/// 
/// ```(def <name> <value>)```
/// 
/// Assigns `<value>` to the atom called `<name>`. It is possible to redefine an existing atom. The value
/// will be available to everything in the same frame after this statement.
///
pub struct DefKeyword {
}

impl DefKeyword {
    ///
    /// Creates a new syntax compiler for the 'def' syntax
    ///
    pub fn new() -> SyntaxCompiler {
        unimplemented!()
    }
}

pub fn def_keyword() -> SyntaxCompiler {
    let bind = get_expression_arguments()
        .and_then_ok(|args: ListTuple<(AtomId, CellRef)>| {
            // The arguments are just the name and the value
            let ListTuple((name, value)) = args;

            wrap_binding::<Result<CellRef, BindError>>(Err(BindError::RuntimeError))
        });

    unimplemented!()
}

impl BindingMonad for DefKeyword {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##def##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Arguments should be a list containing an atom and a value
        let args = bindings.args.clone();
        let args = args.and_then(|args| args.to_vec());
        let args = match args { Some(args) => args, None => return (bindings, Err(BindError::ArgumentsWereNotSupplied)) };

        if args.len() < 2 { return (bindings, Err(BindError::MissingArgument)); }
        if args.len() > 2 { return (bindings, Err(BindError::TooManyArguments)); }

        let atom    = &args[0];
        let value   = &args[1];

        // Fetch the atom ID
        let atom = match &**atom {
            SafasCell::Atom(atom_id)    => atom_id,
            _                           => return (bindings, Err(BindError::VariablesMustBeAtoms))
        };

        // Evaluate the value
        let statement               = bind_statement(Arc::clone(value), bindings);
        let (statement, bindings)   = match statement {
            Ok((statement, bindings))   => (compile_statement(statement), bindings),
            Err((err, bindings))        => return (bindings, Err(err))
        };
        let statement               = match statement { Ok(statement) => statement, Err(err) => return (bindings, Err(err)) };

        // Allocate a spot for this value
        let mut bindings    = bindings;
        let cell_id         = bindings.alloc_cell();

        // Associate with the atom ID
        bindings.symbols.insert(*atom, SafasCell::FrameReference(cell_id, 0).into());
        bindings.export(*atom);

        // Final actions need to store their value in this cell
        let mut actions     = statement;
        actions.push(Action::StoreCell(cell_id));

        (bindings, Ok(actions))
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
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##define_symbol##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Allocate a cell for this binding
        let mut bindings    = bindings;
        let cell_id         = bindings.num_cells;
        bindings.num_cells  += 1;
        bindings.symbols.insert(self.atom_id, SafasCell::FrameReference(cell_id, 0).into());

        // Actions just load the binding into the cell
        let actions         = smallvec![Action::Value(Arc::clone(&self.value)), Action::StoreCell(cell_id)];

        (bindings, Ok(actions))
    }
}

///
/// Creates a binding monad that defines a symbol to evaluate a particular cell value
///
pub fn define_symbol<Cell: Into<CellRef>>(atom: &str, value: Cell) -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Retrieve the atom ID
    let atom_id = get_id_for_atom_with_name(atom);

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
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##define_symbol##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
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
pub fn define_symbol_value<Cell: Into<CellRef>>(atom: &str, value: Cell) -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Retrieve the atom ID
    let atom_id = get_id_for_atom_with_name(atom);

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
        let val = eval("(def a 1) a").unwrap().0.to_string();
        assert!(val == "1".to_string());
    }

    #[test]
    fn define_multiple_atoms() {
        let val = eval("(def a 1) (def b 2) a").unwrap().0.to_string();
        assert!(val == "1".to_string());

        let val = eval("(def a 1) (def b 2) b").unwrap().0.to_string();
        assert!(val == "2".to_string());
    }
}
