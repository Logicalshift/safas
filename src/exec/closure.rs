use super::frame::*;
use super::lambda::*;
use super::frame_monad::*;
use super::runtime_error::*;

use crate::meta::*;

use std::sync::*;

///
/// A closure is a monad that generates a lambda monad cell
/// 
/// That is, given a function that needs to read values from the current frame to work, this will return a function
/// with those values bound.
///
#[derive(Clone)]
pub struct Closure<Action: FrameMonad> {
    /// The action that will be performed by the closure
    action: Arc<Action>,

    /// The cells that should be imported from the frame this closure is resolved in
    /// 
    /// The two values in the tuple are the location in the source frame and the location in the target frame
    import_cells: Vec<(usize, usize)>,

    /// Cells bound to values other than frame references
    bound_cells: Vec<(usize, CellRef)>,

    /// The number of cells to allocate on the frame for this function
    num_cells: usize,

    /// The number of cells to fill with arguments for this function (loaded in to cells 1-args)
    arg_count: usize,
}

impl<Action: 'static+FrameMonad> Closure<Action> {
    ///
    /// Creates a new closure monad
    /// 
    /// The cell IDs are in the form of (source, target)
    ///
    pub fn new<CellIter: IntoIterator<Item=(usize, usize)>>(action: Action, import_cells: CellIter, num_cells: usize, arg_count: usize) -> Closure<Action> {
        Closure {
            action:         Arc::new(action),
            import_cells:   import_cells.into_iter().collect(),
            bound_cells:    vec![],
            num_cells:      num_cells,
            arg_count:      arg_count
        }
    }
}

impl<Action: 'static+FrameMonad+Clone> Closure<Action> {
    ///
    /// Changes the source references according to the results of the substitution function
    ///
    pub fn substitute_frame_references(&self, substitute: &mut dyn FnMut(FrameReference) -> Option<CellRef>) -> Closure<Action> {
        // Rebind the imported cells for this closure
        let mut new_import_cells    = vec![];
        let mut new_bound_cells     = self.bound_cells.clone();

        // Substitute the imported cells
        for (src_cell_id, tgt_cell_id) in self.import_cells.iter() {
            if let Some(substitution) = substitute(FrameReference(*src_cell_id, 0, ReferenceType::Value)) {
                // Bind to an absolute value or a different cell
                match &*substitution {
                    SafasCell::FrameReference(new_src_cell_id, 0, _)    => new_import_cells.push((*new_src_cell_id, *tgt_cell_id)),
                    _                                                   => new_bound_cells.push((*tgt_cell_id, substitution))
                }
            } else {
                // Import as before
                new_import_cells.push((*src_cell_id, *tgt_cell_id));
            }
        }

        // Generate the substituted closure
        Closure {
            action:         Arc::clone(&self.action),
            import_cells:   new_import_cells,
            bound_cells:    new_bound_cells,
            num_cells:      self.num_cells,
            arg_count:      self.arg_count
        }
    }
}


impl<Action: 'static+FrameMonad<Binding=RuntimeResult>> FrameMonad for Closure<Action> {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        let args = (0..self.arg_count).into_iter().map(|_| "_").collect::<Vec<_>>().join(" ");

        format!("(closure ({}) {})", args, self.action.description())
    }

    fn execute(&self, frame: Frame) -> (Frame, RuntimeResult) {
        // Read the cells from the current frame
        let cells   = self.import_cells.iter()
            .map(|(src_idx, tgt_idx)| (*tgt_idx, Arc::clone(&frame.cells[*src_idx])))
            .chain(self.bound_cells.iter()
                .map(|(tgt_idx, cell_ref)| (*tgt_idx, Arc::clone(cell_ref))))
            .collect();

        // Create a closure body
        let body    = ClosureBody { action: Arc::clone(&self.action), cells: cells };

        // Resulting value is a lambda
        let lambda  = Lambda::new(body, self.num_cells, self.arg_count);
        let lambda  = SafasCell::FrameMonad(Box::new(lambda));
        let lambda  = Arc::new(lambda);

        (frame, Ok(lambda))
    }
}

///
/// A stack closure is a monad that generates a lambda monad cell
/// 
/// That is, given a function that needs to read values from the current frame to work, this will return a function
/// with those values bound. Unlike Closure it pops the imported values from the stack instead of loading them from cells
///
pub struct StackClosure<Action: FrameMonad> {
    /// The action that will be performed by the closure
    action: Arc<Action>,

    /// The cells that should be imported from the frame this closure is resolved in
    /// 
    /// The two values in the tuple are the location in the source frame and the location in the target frame
    import_cells: Vec<usize>,

    /// The number of cells to allocate on the frame for this function
    num_cells: usize,

    /// The number of cells to fill with arguments for this function (loaded in to cells 1-args)
    arg_count: usize,
}

impl<Action: 'static+FrameMonad> StackClosure<Action> {
    ///
    /// Creates a new closure monad
    /// 
    /// The cell IDs are in the form of (source, target)
    ///
    pub fn new<CellIter: IntoIterator<Item=(usize)>>(action: Action, import_cells: CellIter, num_cells: usize, arg_count: usize) -> StackClosure<Action> {
        StackClosure {
            action:         Arc::new(action),
            import_cells:   import_cells.into_iter().collect(),
            num_cells:      num_cells,
            arg_count:      arg_count
        }
    }
}

impl<Action: 'static+FrameMonad<Binding=RuntimeResult>> FrameMonad for StackClosure<Action> {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        let args            = (0..self.arg_count).into_iter().map(|_| "_").collect::<Vec<_>>().join(" ");
        let import_cells    = self.import_cells.iter().map(|cell_id| cell_id.to_string()).collect::<Vec<_>>().join(", ");

        format!("(closure ({}) ({}) {})", args, import_cells, self.action.description())
    }

    fn execute(&self, frame: Frame) -> (Frame, RuntimeResult) {
        // Read the cells from the current frame
        let mut cells   = vec![];
        let mut frame   = frame;
        for tgt_idx in self.import_cells.iter() {
            let value   = frame.stack.pop();
            let value   = match value { Some(value) => value, None => return (frame, Err(RuntimeError::StackIsEmpty)) };
            cells.push((*tgt_idx, value));
        }

        // Create a closure body
        let body        = ClosureBody { action: Arc::clone(&self.action), cells: cells };

        // Resulting value is a lambda
        let lambda      = Lambda::new(body, self.num_cells, self.arg_count);
        let lambda      = SafasCell::FrameMonad(Box::new(lambda));
        let lambda      = Arc::new(lambda);

        (frame, Ok(lambda))
    }
}

///
/// The closure body 
///
struct ClosureBody<Action: FrameMonad> {
    /// The action the closure will perform once the cells are loaded
    action: Arc<Action>,

    /// The values to load into cells for this closure
    cells: Vec<(usize, CellRef)>
}

impl<Action: FrameMonad> FrameMonad for ClosureBody<Action> {
    type Binding = Action::Binding;

    fn description(&self) -> String {
        format!("(closure_body ({}) {})", self.cells.iter().map(|(_index, value)| value.to_string()).collect::<Vec<_>>().join(" "), self.action.description())
    }

    fn execute(&self, frame: Frame) -> (Frame, Action::Binding) {
        let mut frame = frame;

        // Store the values of the cells
        for (cell_index, cell_value) in self.cells.iter() {
            frame.cells[*cell_index] = Arc::clone(cell_value);
        }

        // Resolve the action
        self.action.execute(frame)
    }
}
