use super::symbol_value::*;

use std::collections::{HashMap};

///
/// Represents a set of bindings from atoms to symbols
///
#[derive(Clone)]
pub struct SymbolBindings {
    /// The symbols in this binding
    pub symbols: HashMap<u64, SymbolValue>,

    /// The symbol bindings in the 'parent' of the current frame
    pub parent: Option<Box<SymbolBindings>>,

    /// The number of cells to allocate in the current frame (there's always one, which we use as the accumulator)
    pub num_cells: usize
}

impl SymbolBindings {
    ///
    /// Creates a new set of symbol bindings
    ///
    pub fn new() -> SymbolBindings {
        SymbolBindings {
            symbols:    HashMap::new(),
            parent:     None,
            num_cells:  1
        }
    }

    ///
    /// Looks up the value for a symbol in this binding
    ///
    pub fn look_up(&self, symbol: u64) -> Option<SymbolValue> {
        self.symbols.get(&symbol).cloned()
    }
}
