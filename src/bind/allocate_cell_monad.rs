use super::binding_monad::*;
use super::symbol_bindings::*;

///
/// Binding monad that allocates a cell on the current frame
///
struct AllocateCellMonad;

impl BindingMonad for AllocateCellMonad {
    type Binding = usize;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, usize) {
        let mut bindings    = bindings;
        let cell            = bindings.alloc_cell();

        (bindings, cell)
    }
}

///
/// Creates a binding monad that will allocate a new cell in the current frame, and returns it
/// 
/// For example: `allocate_cell().and_then(|cell_id| { /* Do something with the cell */ })`
///
pub fn allocate_cell() -> impl BindingMonad<Binding=usize> {
    AllocateCellMonad
}
