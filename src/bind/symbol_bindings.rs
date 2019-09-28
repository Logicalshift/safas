use crate::meta::*;

use smallvec::*;
use std::sync::*;
use std::collections::{HashMap};

///
/// Represents a set of bindings from atoms to symbols
///
#[derive(Clone)]
pub struct SymbolBindings {
    /// When binding on a macro or other syntax item, the arguments that were supplied to the macro
    pub args: Option<CellRef>,

    /// When binding a macro or other syntax item, the depth (number of frames) that the item was found at
    pub depth: Option<u32>,

    /// The symbols in this binding
    pub symbols: HashMap<u64, CellRef>,

    /// The symbols to export to the parent context
    pub export_symbols: SmallVec<[u64; 2]>,

    /// A list of the symbols to import from the parent, along with the cell they should be stored in
    pub import_symbols: SmallVec<[(CellRef, usize); 2]>,

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
            depth:          None,
            symbols:        HashMap::new(),
            export_symbols: smallvec![],
            import_symbols: smallvec![],
            parent:         None,
            num_cells:      1,
            is_interior:    false
        }
    }

    ///
    /// Looks up the value for a symbol in this binding. If the symbol is found the return value is the
    /// cell bound to this symbol and the number of frames 'above' the current stack frame that the
    /// match was made.
    /// 
    /// For frame references, the FrameReference is returned updated with the level where the symbol
    /// could be found.
    ///
    pub fn look_up(&self, symbol: u64) -> Option<(CellRef, u32)> {
        let mut binding = Some(self);
        let mut level   = 0;

        while let Some(current_binding) = binding {
            // Look up in the current binding
            if let Some(symbol) = current_binding.symbols.get(&symbol) {
                // Return the value if we find it, adjusting the frame level for cells that need to be imported
                if let Some((cell, bound_level)) = symbol.frame_reference() {
                    if level == 0 {
                        return Some((Arc::clone(&symbol), level));
                    } else {
                        return Some((SafasCell::FrameReference(cell, bound_level + level).into(), level));
                    }
                } else {
                    return Some((Arc::clone(&symbol), level));
                }
            }

            // Move up a level
            if !current_binding.is_interior {
                level += 1;
            }

            binding = current_binding.parent.as_ref().map(|parent| &**parent);
        }

        None
    }

    ///
    /// Pushes a new symbol binding, as if it were a new frame (eg, due to a function call)
    ///
    pub fn push_new_frame(self) -> SymbolBindings {
        SymbolBindings {
            args:           None,
            depth:          None,
            symbols:        HashMap::new(),
            export_symbols: smallvec![],
            import_symbols: smallvec![],
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
            depth:          None,
            symbols:        HashMap::new(),
            export_symbols: smallvec![],
            import_symbols: smallvec![],
            num_cells:      self.num_cells,
            parent:         Some(Box::new(self)),
            is_interior:    true
        }
    }

    ///
    /// Pops a set of symbol bindings, returning the symbol bindings and the list of values that need to be
    /// loaded from the current frame into the new one
    ///
    pub fn pop(mut self) -> (SymbolBindings, SmallVec<[(CellRef, usize); 2]>) {
        // Take the parent binding
        let parent = self.parent.take().expect("Parent binding missing");

        // Unbox it
        let mut parent = *parent;

        if self.is_interior {
            // Make sure the number of cells is updated on the parent if it's interior
            parent.num_cells = self.num_cells.max(parent.num_cells);

            // For interior frames, imports come straight from the same parent
            parent.import_symbols.extend(self.import_symbols.drain());
        }

        // Move any export symbols into the parent
        for export_id in self.export_symbols.drain() {
            if let Some(value) = self.symbols.get(&export_id) {
                parent.symbols.insert(export_id, value.clone());
            }
        }

        // The parent binding is the result
        (parent, self.import_symbols)
    }

    ///
    /// Allocates a new storage cell that's currently not being used
    ///
    pub fn alloc_cell(&mut self) -> usize {
        let cell_id     = self.num_cells;
        self.num_cells += 1;

        cell_id
    }

    ///
    /// Binds an atom to an unused cell
    ///
    pub fn bind_atom_to_new_cell(&mut self, atom_id: u64) -> usize {
        let cell_id = self.alloc_cell();
        self.symbols.insert(atom_id, SafasCell::FrameReference(cell_id, 0).into());
        cell_id
    }

    ///
    /// Exports the specified atom to the parent bindings
    /// 
    /// This must be an interior binding for this to work. The value of the specified symbol will become visible outside of this binding.
    ///
    pub fn export(&mut self, atom_id: u64) {
        if self.is_interior {
            self.export_symbols.push(atom_id);
        }
    }

    ///
    /// Adds a symbol to be imported from the parent frame of this binding. The symbol should be a frame reference.
    /// 
    /// Any binding that references this symbol will be updated to point to the cell after this call
    ///
    pub fn import(&mut self, symbol: CellRef, cell_id: usize) {
        if let Some((import_from_cell_id, import_from_frame_id)) = symbol.frame_reference() {
            if import_from_frame_id > 0 {
                // Import from the symbol in the parent frame
                self.import_symbols.push((SafasCell::FrameReference(import_from_cell_id, import_from_frame_id-1).into(), cell_id));

                // Update any references to this parent cell to point to the imported cell
                for (_symbol, value) in self.symbols.iter_mut() {
                    if let SafasCell::FrameReference(reference_cell, reference_frame) = &**value {
                        if *reference_cell == import_from_cell_id && *reference_frame == import_from_frame_id {
                            let replacement_symbol  = SafasCell::FrameReference(cell_id, 0);
                            *value                  = replacement_symbol.into();
                        }
                    }
                }
            } else {
                // Symbol can't be imported
                panic!("Cannot import a symbol that is already in the current frame")
            }
        } else {
            panic!("Import symbols must be a reference to a cell in a parent frame")
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_new_frame() {
        let frame = SymbolBindings::new();
        assert!(frame.parent.is_none());

        let frame = frame.push_new_frame();
        assert!(frame.parent.is_some());
    }

    #[test]
    fn look_up_missing_symbol() {
        let frame = SymbolBindings::new();

        let symbol = frame.look_up(0);
        assert!(symbol.is_none());
    }

    #[test]
    fn look_up_symbol_in_current_frame() {
        let mut frame = SymbolBindings::new();
        frame.symbols.insert(0, SafasCell::Number(1.into()).into());

        let symbol = frame.look_up(0);
        assert!(!symbol.is_none());
        let (symbol, level) = symbol.unwrap();
        assert!(symbol.number_value().unwrap() == 1.into());
        assert!(level == 0);
    }

    #[test]
    fn look_up_symbol_in_parent_frame() {
        let mut frame = SymbolBindings::new();
        frame.symbols.insert(0, SafasCell::Number(1.into()).into());
        let frame = frame.push_new_frame();

        let symbol = frame.look_up(0);
        assert!(!symbol.is_none());
        let (symbol, level) = symbol.unwrap();
        assert!(symbol.number_value().unwrap() == 1.into());
        assert!(level == 1);
    }

    #[test]
    fn look_up_replaced_symbol() {
        let mut frame = SymbolBindings::new();
        frame.symbols.insert(0, SafasCell::Number(1.into()).into());
        let mut frame = frame.push_new_frame();
        frame.symbols.insert(0, SafasCell::Number(2.into()).into());

        let symbol = frame.look_up(0);
        assert!(!symbol.is_none());
        let (symbol, level) = symbol.unwrap();
        assert!(symbol.number_value().unwrap() == 2.into());
        assert!(level == 0);
    }

    #[test]
    fn first_cell_allocated_is_cell_1() {
        // Cell 0 is always allocated by default
        let mut frame = SymbolBindings::new();
        assert!(frame.alloc_cell() == 1);
    }

    #[test]
    fn imports_are_popped() {
        let frame       = SymbolBindings::new();
        let mut frame   = frame.push_new_frame();

        frame.import(SafasCell::FrameReference(3, 2).into(), 3);

        let (_frame, imports) = frame.pop();

        // We generate imports in the context of the incoming frame, so the 'frame count' ends up being reduced by 1 here
        assert!(imports.len() == 1);
        assert!(imports[0].0.frame_reference() == Some((3, 1)));
        assert!(imports[0].1 == 3);
    }

    #[test]
    fn imports_are_not_popped_for_interior_frames() {
        let frame       = SymbolBindings::new();
        let mut frame   = frame.push_interior_frame();

        frame.import(SafasCell::FrameReference(3, 2).into(), 3);

        let (_frame, imports) = frame.pop();

        // We generate imports in the context of the incoming frame, so the 'frame count' ends up being reduced by 1 here
        assert!(imports.len() == 0);
    }

    #[test]
    fn imports_are_inherited_from_interior_frames() {
        let frame       = SymbolBindings::new();
        let frame       = frame.push_new_frame();
        let mut frame   = frame.push_interior_frame();

        frame.import(SafasCell::FrameReference(3, 2).into(), 3);

        // Pop the interior and the main frame (imports should pop)
        let (frame, _imports) = frame.pop();
        let (_frame, imports) = frame.pop();

        // We generate imports in the context of the incoming frame, so the 'frame count' ends up being reduced by 1 here
        assert!(imports.len() == 1);
        assert!(imports[0].0.frame_reference() == Some((3, 1)));
        assert!(imports[0].1 == 3);
    }

    #[test]
    fn import_updates_frame_references() {
        let mut frame = SymbolBindings::new();

        // Reference in a parent frame
        frame.symbols.insert(0, SafasCell::FrameReference(2, 2).into());

        // Import into cell 3
        frame.import(SafasCell::FrameReference(2, 2).into(), 3);

        // Symbol we added should now point at the current frame
        let (symbol, level) = frame.look_up(0).unwrap();
        assert!(symbol.frame_reference() == Some((3, 0)));
        assert!(level == 0);
    }
}
