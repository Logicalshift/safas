use super::cell::*;

use crate::exec::*;

use std::cmp::{Ordering};
use smallvec::*;

/// The number of values to store per b-tree node
const BTREE_ORDER: usize = 5;

/// The value to use in a full node as the median
const BTREE_MEDIAN: usize = 3;

///
/// Creates a new empty B-Tree cell
///
pub fn new_btree() -> CellRef {
    SafasCell::BTree(smallvec![], smallvec![]).into()
}

///
/// Searches a B-Tree for the value associated with a particular cell. Returns nil if the key is not present in the BTree
///
pub fn btree_search(btree: CellRef, key: CellRef) -> Result<CellRef, RuntimeError> {
    Err(RuntimeError::NotImplemented)
}

///
/// Attempts to insert a value in a B-Tree cell
///
pub fn btree_insert(btree: CellRef, key_value: (CellRef, CellRef)) -> Result<CellRef, RuntimeError> {
    Err(RuntimeError::NotImplemented)
}

///
/// Result from a btree insert operation (may indicate how the parents should be rewritten)
///
enum BTreeInsertResult {
    /// The value at this position was replaced by the specified BTree
    NewBTree(CellRef),

    /// Error while searching
    Err(RuntimeError)
}

///
/// Result from a BTree search operation
///
enum BTreeSearchResult {
    Found(CellRef),
    NotFound,
    Err(RuntimeError)
}

///
/// Implementation of the btree search operation
///
fn btree_search_internal(btree: &CellRef, key: CellRef) -> BTreeSearchResult {
    match &**btree {
        SafasCell::BTree(key_values, child_nodes)   => {
            // Search the key values
            for idx in 0..key_values.len() {
                // Values are ordered by key
                match (&*key_values[idx].0).partial_cmp(&*key) {
                    Some(Ordering::Equal)   => { return BTreeSearchResult::Found(key_values[idx].1.clone()) },
                    Some(Ordering::Less)    => { },
                    Some(Ordering::Greater) => { 
                        // If there are any child nodes, they contain the items that are between this value and the previous value
                        if child_nodes.len() > 0 {
                            return btree_search_internal(&child_nodes[idx], key);
                        } else {
                            return BTreeSearchResult::NotFound;
                        }
                    },
                    None                    => { return BTreeSearchResult::Err(RuntimeError::CannotCompare(key_values[idx].0.clone(), key)); }
                }
            }

            // Value may be in the final child node
            if child_nodes.len() > 0 {
                return btree_search_internal(&child_nodes[child_nodes.len()-1], key);
            } else {
                return BTreeSearchResult::NotFound;
            }
        },

        SafasCell::Nil                              => BTreeSearchResult::NotFound,
        _                                           => BTreeSearchResult::Err(RuntimeError::NotABTree(btree.clone()))
    }
}

///
/// Implementation of the btree insert operation. This will insert or replace the key with the specified value
///
fn btree_insert_internal(btree: &CellRef, key: CellRef, value: CellRef) -> BTreeInsertResult {
    match &**btree {
        // Search b-tree nodes
        SafasCell::BTree(key_values, child_nodes) => {
            for idx in 0..=key_values.len() {
                // Values are ordered by key. We add an extra test for the final value to avoid a double implementation of the match rules
                let ordering = if idx < key_values.len() { (&*key_values[idx].0).partial_cmp(&*key) } else { Some(Ordering::Greater) };

                match ordering {
                    Some(Ordering::Less)    => { },

                    Some(Ordering::Equal)   => {
                        // Found the key: just replace it here
                        let mut new_key_values  = key_values.clone();
                        new_key_values[idx]     = (key, value);

                        return BTreeInsertResult::NewBTree(SafasCell::BTree(new_key_values, child_nodes.clone()).into());
                    },

                    Some(Ordering::Greater) => { 
                        if child_nodes.len() > 0 {
                            // Need to insert in this child node
                            match btree_insert_internal(&child_nodes[idx], key, value) {
                                BTreeInsertResult::NewBTree(new_btree) => {
                                    // Replace the child node with the new one
                                    let mut new_child_nodes = child_nodes.clone();
                                    new_child_nodes[idx] = new_btree;

                                    return BTreeInsertResult::NewBTree(SafasCell::BTree(key_values.clone(), new_child_nodes).into())
                                },

                                // Pass errors along
                                BTreeInsertResult::Err(err) => { return BTreeInsertResult::Err(err) }
                            }
                        } else {
                            // Need to insert at this leaf node
                            if key_values.len() < BTREE_ORDER {
                                // Node isn't full: just add to the list
                                let mut new_key_values = key_values.clone();
                                new_key_values.push((key, value));
                                new_key_values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

                                return BTreeInsertResult::NewBTree(SafasCell::BTree(new_key_values, child_nodes.clone()).into());
                            } else {
                                // Node is full: need to split at the median value
                                /*
                                let median_key  = key_values[BTREE_MEDIAN].clone();
                                let left_cells  = key_values[0..BTREE_MEDIAN].iter().collect::<SmallVec<_>>();
                                let right_cells = key_values[(BTREE_MEDIAN+1)..key_values.len()].iter().collect::<SmallVec<_>>();
                                */
                                
                                unimplemented!()
                            }
                        }
                    },
                    None                    => { return BTreeInsertResult::Err(RuntimeError::CannotCompare(key_values[idx].0.clone(), key)); }
                }
            }

            // Can't reach here as we'll eventually hit the Ordering::Greater case
            unreachable!()
        },

        // Just replace the nil value with a b-tree with a single value
        SafasCell::Nil => BTreeInsertResult::NewBTree(SafasCell::BTree(smallvec![(key, value)], smallvec![]).into()),

        // Other values are not b-trees
        _ => BTreeInsertResult::Err(RuntimeError::NotImplemented)
    }
}

#[cfg(test)]
mod test {

}