use super::bind_error::*;
use super::binding_monad::*;
use super::symbol_bindings::*;

///
/// Binding monad that allocates a cell on the current frame
///
struct AllocateCellMonad;

impl BindingMonad for AllocateCellMonad {
    type Binding = usize;

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // Note that the cell is not allocated during pre-binding!
        (bindings, 0)
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<usize, BindError>) {
        let mut bindings    = bindings;
        let cell            = bindings.alloc_cell();

        (bindings, Ok(cell))
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
