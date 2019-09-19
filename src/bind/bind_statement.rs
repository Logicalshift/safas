use super::symbol_bindings::*;
use super::symbol_value::*;
use super::bind_error::*;

use crate::meta::*;
use crate::exec::*;

use smallvec::*;
use std::sync::*;
use std::result::{Result};

///
/// Performs binding to generate the actions for a simple statement
///
pub fn bind_statement(source: Arc<SafasCell>, bindings: SymbolBindings) -> Result<(SmallVec<[Action; 8]>, SymbolBindings), BindError> {
    use self::SafasCell::*;

    match &*source {
        /// Lists are processed according to their first value
        List(car, cdr)  => { bind_list_statement(Arc::clone(car), Arc::clone(cdr), bindings) }

        /// Atoms bind to their atom value
        Atom(atom_id)   => {
            // Look up the value for this symbol
            let symbol_value = bindings.look_up(*atom_id);

            if let Some(symbol_value) = symbol_value {
                use self::SymbolValue::*;

                match symbol_value {
                    Constant(value)                 => Ok((smallvec![Action::Value(Arc::clone(&value))], bindings)),
                    Unbound(atom_id)                => Err(BindError::UnboundSymbol),
                    FrameReference(cell_num, frame) => unimplemented!(),
                    FrameMonad(monad)               => Ok((smallvec![Action::Value(Arc::new(SafasCell::Monad(Arc::clone(&monad))))], bindings)),
                    MacroMonad(monad)               => Ok((smallvec![Action::Value(Arc::new(SafasCell::MacroMonad(Arc::clone(&monad))))], bindings)),
                    ActionMonad(monad)              => Ok((smallvec![Action::Value(Arc::new(SafasCell::ActionMonad(Arc::clone(&monad))))], bindings))
                }
            } else {
                // Not a valid symbol
                Err(BindError::UnknownSymbol)
            }
        }

        // Normal values just get loaded into cell 0
        other           => { Ok((smallvec![Action::Value(Arc::clone(&source))], bindings)) }
    }
}

///
/// Binds a list statement, like `(cons 1 2)`
///
pub fn bind_list_statement(car: Arc<SafasCell>, cdr: Arc<SafasCell>, bindings: SymbolBindings) -> Result<(SmallVec<[Action; 8]>, SymbolBindings), BindError> {
    unimplemented!()
}
