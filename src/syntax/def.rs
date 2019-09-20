use crate::bind::*;
use crate::exec::*;
use crate::meta::*;

use smallvec::*;
use std::sync::*;

///
/// The monad for the 'def' syntax (def atom value)
///
pub struct DefMonad {
}

impl BindingMonad for DefMonad {
    type Binding=Result<Arc<SmallVec<[Action; 8]>>, BindError>;

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
            _                           => return (bindings, Err(BindError::MissingArgument))
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

        // Final actions need to store their value in this cell
        let mut actions     = statement;
        actions.push(Action::StoreCell(cell_id));

        (bindings, Ok(Arc::new(actions)))
    }
}
