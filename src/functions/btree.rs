use crate::exec::*;
use crate::meta::*;

use std::convert::{TryFrom};

///
/// `(btree (key value) ...) -> btree`
///
pub fn btree_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|VarArgs(key_values)| {
        // Create an empty btree
        let mut result = CellRef::new(SafasCell::BTree(vec![], vec![]));

        // Add any key/value pairs from the argument list
        let mut pos = &*key_values;
        while let SafasCell::List(key_value_pair, next) = pos {
            // Should be a (key, value) list
            let ListTuple((key, value)) = ListTuple::<(CellRef, CellRef)>::try_from(key_value_pair.clone())?;

            // Add to the result
            result = btree_insert(result, (key, value))?;

            pos = &*next;
        }

        Ok(result)
    })
}

///
/// `(btree_insert btree key value) -> btree`
/// 
/// Creates a new btree from an old btree with the specified value added to it
///
pub fn btree_insert_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(btree, key, value): (CellRef, CellRef, CellRef)| {
        btree_insert(btree, (key, value))
    })
}

///
/// `(btree_lookup btree key) -> value`
/// 
/// Looks up a value, returning nil if it could not be found in the btree
///
pub fn btree_lookup_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(btree, key): (CellRef, CellRef)| {
        btree_search(btree, key)
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn make_btree() {
        let val = eval(
                "(btree (quote (a b)) (quote (c d)))"
            ).unwrap();

        assert!(val.to_string() == "btree#(\n  a -> b\n  c -> d\n)".to_string());
    }
}
