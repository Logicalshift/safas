use super::binding_monad::*;
use super::symbol_bindings::*;

use crate::meta::*;

///
/// The export symbol monad exports a symbol to the parent environment
///
struct ExportSymbolMonad {
    atom_id: u64
}

impl BindingMonad for ExportSymbolMonad {
    type Binding=AtomId;

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        (bindings, AtomId(self.atom_id))
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, AtomId) {
        // Export the symbol
        let mut bindings = bindings;
        bindings.export(self.atom_id);

        // Result is the atom id
        (bindings, AtomId(self.atom_id))
    }
}

///
/// Creates a monad that will export a particular atom into the parent environment
///
pub fn export_symbol<Atom: Into<AtomId>>(atom: Atom) -> impl BindingMonad<Binding=AtomId> {
    let atom            = atom.into();
    let AtomId(atom_id) = atom;

    ExportSymbolMonad {
        atom_id
    }
}
