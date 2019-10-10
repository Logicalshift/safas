use super::bind_error::*;
use super::binding_monad::*;
use super::symbol_bindings::*;

use crate::meta::*;

use std::convert::*;
use std::marker::{PhantomData};

///
/// Monad that returns the arguments to the syntax being bound
/// 
/// Any type that can be converted from a CellRef can be used
///
pub struct BindArgsMonad<TArgs> {
    args: PhantomData<TArgs>
}

impl<TArgs> BindArgsMonad<TArgs>
where   TArgs: TryFrom<CellRef>,
        TArgs: Send+Sync,
        <TArgs as TryFrom<CellRef>>::Error: Into<BindError> {
    /// Creates a new BindArgsMonad that will attempt to load the cell into the specified type
    pub fn new() -> BindArgsMonad<TArgs> {
        BindArgsMonad {
            args: PhantomData
        }
    }
}

impl<TArgs: Default> BindingMonad for BindArgsMonad<TArgs> 
where   TArgs: TryFrom<CellRef>,
        TArgs: Send+Sync,
        <TArgs as TryFrom<CellRef>>::Error: Into<BindError> {
    type Binding = TArgs;

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        if let Some(args) = bindings.args.as_ref() {
            // Try to convert the arguments into the target type
            let args = args.clone();
            let args = TArgs::try_from(args);

            match args {
                Ok(args)    => (bindings, Ok(args)),
                Err(err)    => (bindings, Err(err.into()))
            }
        } else {
            // No arguments were supplied
            (bindings, Err(BindError::ArgumentsWereNotSupplied))
        }
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) { 
        if let Some(args) = bindings.args.as_ref() {
            // Try to convert the arguments into the target type
            let args = args.clone();
            let args = TArgs::try_from(args);

            match args {
                Ok(args)    => (bindings, args),
                Err(_err)   => (bindings, TArgs::default())
            }
        } else {
            // No arguments were supplied
            (bindings, TArgs::default())
        }
    }
}

///
/// Returns a monad that gets the arguments for the expression that is being bound
///
pub fn get_expression_arguments<TArgs: Default>() -> impl BindingMonad<Binding=TArgs>
where   TArgs: TryFrom<CellRef>,
        TArgs: Send+Sync,
        <TArgs as TryFrom<CellRef>>::Error: Into<BindError> {
    BindArgsMonad::new()
}
