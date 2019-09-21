use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;

///
/// The monad for the 'def' syntax (def atom value)
///
pub struct DefKeyword {
}

impl DefKeyword {
    ///
    /// Creates a new monad for the 'def' syntax
    ///
    pub fn new() -> DefKeyword {
        DefKeyword { }
    }
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
            Ok((statement, bindings))   => (statement, bindings),
            Err((err, bindings))        => return (bindings, Err(err))
        };

        // Allocate a spot for this value
        let mut bindings    = bindings;
        let cell_id         = bindings.num_cells;
        bindings.num_cells += 1;

        // Associate with the atom ID
        bindings.symbols.insert(*atom, SymbolValue::FrameReference(cell_id, 0));
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
    value:      Arc<SafasCell>
}

impl BindingMonad for DefineSymbol {
    type Binding=Result<SmallVec<[Action; 8]>, BindError>;

    fn description(&self) -> String { "##define_symbol##".to_string() }

    fn resolve(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Allocate a cell for this binding
        let mut bindings    = bindings;
        let cell_id         = bindings.num_cells;
        bindings.num_cells  += 1;
        bindings.symbols.insert(self.atom_id, SymbolValue::FrameReference(cell_id, 0));

        // Actions just load the binding into the cell
        let actions         = smallvec![Action::Value(Arc::clone(&self.value)), Action::StoreCell(cell_id)];

        (bindings, Ok(actions))
    }
}

///
/// Creates a binding monad that defines a symbol to evaluate a particular cell value
///
pub fn define_symbol(atom: &str, value: SafasCell) -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Retrieve the atom ID
    let atom_id = get_id_for_atom_with_name(atom);

    DefineSymbol {
        atom_id:    atom_id,
        value:      Arc::new(value)
    }
}

///
/// Monad that defines an atom to be a particular value
///
struct DefineSymbolValue {
    atom_id:    u64,
    value:      SymbolValue
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
pub fn define_symbol_value(atom: &str, value: SymbolValue) -> impl BindingMonad<Binding=Result<SmallVec<[Action; 8]>, BindError>> {
    // Retrieve the atom ID
    let atom_id = get_id_for_atom_with_name(atom);

    DefineSymbolValue {
        atom_id:    atom_id,
        value:      value
    }
}
