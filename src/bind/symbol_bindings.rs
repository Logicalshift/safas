use super::symbol_value::*;

use std::collections::{HashMap};

///
/// Represents a set of bindings from atoms to symbols
///
#[derive(Clone)]
pub struct SymbolBindings {
    pub symbols: HashMap<u64, SymbolValue>
}

impl SymbolBindings {
    ///
    /// Creates a new set of symbol bindings
    ///
    pub fn new() -> SymbolBindings {
        SymbolBindings {
            symbols: HashMap::new()
        }
    }
}
