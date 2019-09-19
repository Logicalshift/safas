use super::symbol_value::*;

use crate::meta::*;

use std::sync::*;
use std::collections::{HashMap};

///
/// Represents a set of bindings from atoms to symbols
///
#[derive(Clone)]
pub struct SymbolBindings {
    /// When binding on a macro or similar, the arguments that were supplied to the macro
    pub args: Option<Arc<SafasCell>>,

    /// The symbols in this binding
    pub symbols: HashMap<u64, SymbolValue>,

    /// The symbol bindings in the 'parent' of the current frame
    pub parent: Option<Box<SymbolBindings>>,

    /// The number of cells to allocate in the current frame (there's always one, which we use as the accumulator)
    pub num_cells: usize,

    /// True if this is an 'interior' binding (shares its cells with its parent)
    pub is_interior: bool
}

impl SymbolBindings {
    ///
    /// Creates a new set of symbol bindings
    ///
    pub fn new() -> SymbolBindings {
        SymbolBindings {
            args:           None,
            symbols:        HashMap::new(),
            parent:         None,
            num_cells:      1,
            is_interior:    false
        }
    }

    ///
    /// Looks up the value for a symbol in this binding
    ///
    pub fn look_up(&self, symbol: u64) -> Option<SymbolValue> {
        self.symbols.get(&symbol).cloned()
    }

    ///
    /// Pushes a new symbol binding, as if it were a new frame (eg, due to a function call)
    ///
    pub fn push_new_frame(self) -> SymbolBindings {
        SymbolBindings {
            args:           None,
            symbols:        HashMap::new(),
            parent:         Some(Box::new(self)),
            num_cells:      1,
            is_interior:    false
        }
    }

    ///
    /// Pushes a symbol binding that works as an interior frame (eg, when binding in a macro)
    ///
    pub fn push_interior_frame(self) -> SymbolBindings {
        SymbolBindings {
            args:           None,
            symbols:        HashMap::new(),
            num_cells:      self.num_cells,
            parent:         Some(Box::new(self)),
            is_interior:    true
        }
    }

    ///
    /// Pops a set of symbol bindings
    ///
    pub fn pop(mut self) -> SymbolBindings {
        // Take the parent binding
        let parent = self.parent.take().expect("Parent binding missing");

        // Unbox it
        let mut parent = *parent;

        // Make sure the number of cells is updated on the parent if it's interior
        if self.is_interior {
            parent.num_cells = self.num_cells.max(parent.num_cells);
        }

        // The parent binding is the result
        parent
    }
}
