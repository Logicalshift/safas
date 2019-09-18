use super::symbol_bindings::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;

///
/// Performs binding to generate the actions for a simple statement
///
pub fn bind_simple_statement(source: Arc<SafasCell>, bindings: SymbolBindings) -> SmallVec<[Action; 8]> {
    use self::SafasCell::*;

    match &*source {
        /// Lists generate a list of arguments and a function, which is called
        List(car, cdr)  => { smallvec![] }

        /// Atoms bind to their atom value
        Atom(atom_id)   => { smallvec![] }

        // Normal values just get loaded into cell 0
        other           => { smallvec![] }
    }
}
