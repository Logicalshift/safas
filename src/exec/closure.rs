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
pub struct Closure<Action: FrameMonad> {
    /// The action that will be performed by the closure
    action: Arc<Action>,

    /// The cells that should be imported from the frame this closure is resolved in
    /// 
    /// The two values in the tuple are the location in the source frame and the location in the target frame
    import_cells: Vec<(usize, usize)>,

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
            num_cells:      num_cells,
            arg_count:      arg_count
        }
    }
}

impl<Action: 'static+FrameMonad<Binding=RuntimeResult>> FrameMonad for Closure<Action> {
    type Binding = RuntimeResult;

    fn description(&self) -> String {
        let args = (0..self.arg_count).into_iter().map(|_| "_").collect::<Vec<_>>().join(" ");

        format!("(closure ({}) {})", args, self.action.description())
    }

    fn resolve(&self, frame: Frame) -> (Frame, RuntimeResult) {
        // Read the cells from the current frame
        let cells   = self.import_cells.iter().map(|(src_idx, tgt_idx)| (*tgt_idx, Arc::clone(&frame.cells[*src_idx]))).collect();

        // Create a closure body
        let body    = ClosureBody { action: Arc::clone(&self.action), cells: cells };

        // Resulting value is a lambda
        let lambda  = Lambda::new(body, self.num_cells, self.arg_count);
        let lambda  = SafasCell::Monad(Arc::new(lambda));
        let lambda  = Arc::new(lambda);

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

    fn resolve(&self, frame: Frame) -> (Frame, Action::Binding) {
        let mut frame = frame;

        // Store the values of the cells
        for (cell_index, cell_value) in self.cells.iter() {
            frame.cells[*cell_index] = Arc::clone(cell_value);
        }

        // Resolve the action
        self.action.resolve(frame)
    }
}
