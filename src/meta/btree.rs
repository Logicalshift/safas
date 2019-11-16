use super::cell::*;

use crate::exec::*;

use std::cmp::{Ordering};

/// The number of values to store per b-tree node
const BTREE_ORDER: usize = 5;

/// The value to use in a full node as the median
const BTREE_MEDIAN: usize = 2;

///
/// Creates a new empty B-Tree cell
///
pub fn btree_new() -> CellRef {
    SafasCell::BTree(vec![], vec![]).into()
}

///
/// Searches a B-Tree for the value associated with a particular cell. Returns nil if the key is not present in the BTree
///
pub fn btree_search(btree: CellRef, key: CellRef) -> Result<CellRef, RuntimeError> {
    match btree_search_internal(&btree, key) {
        BTreeSearchResult::Found(key_value) => Ok(key_value),
        BTreeSearchResult::NotFound         => Ok(NIL.clone()),
        BTreeSearchResult::Err(err)         => Err(err)
    }
}

///
/// Attempts to insert a value in a B-Tree cell
///
pub fn btree_insert(btree: CellRef, key_value: (CellRef, CellRef)) -> Result<CellRef, RuntimeError> {
    match btree_insert_internal(&btree, key_value.0, key_value.1) {
        BTreeInsertResult::NewBTree(result)                 => Ok(result),
        BTreeInsertResult::Split(left, right, median_key)   => Ok(SafasCell::BTree(vec![median_key], vec![left, right]).into()),
        BTreeInsertResult::Err(err)                         => Err(err)
    }
}

///
/// Result from a btree insert operation (may indicate how the parents should be rewritten)
///
enum BTreeInsertResult {
    /// The value at this position was replaced by the specified BTree
    NewBTree(CellRef),

    /// The value was split into a left and right b-tree at the specified node
    Split(CellRef, CellRef, (CellRef, CellRef)),

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

                                BTreeInsertResult::Split(left_btree, right_btree, median) => {
                                    return btree_insert_new_key_at_subtree(key_values, child_nodes, median, left_btree, right_btree);
                                },

                                // Pass errors along
                                BTreeInsertResult::Err(err) => { return BTreeInsertResult::Err(err) }
                            }
                        } else {
                            return btree_insert_new_key_at_leaf(key_values, child_nodes, key, value);
                        }
                    },
                    None                    => { return BTreeInsertResult::Err(RuntimeError::CannotCompare(key_values[idx].0.clone(), key)); }
                }
            }

            // Can't reach here as we'll eventually hit the Ordering::Greater case
            unreachable!()
        },

        // Just replace the nil value with a b-tree with a single value
        SafasCell::Nil => BTreeInsertResult::NewBTree(SafasCell::BTree(vec![(key, value)], vec![]).into()),

        // Other values are not b-trees
        _ => BTreeInsertResult::Err(RuntimeError::NotImplemented)
    }
}

///
/// Performs the operations required to insert a new key at a leaf node
///
fn btree_insert_new_key_at_leaf(key_values: &Vec<(CellRef, CellRef)>, child_nodes: &Vec<CellRef>, key: CellRef, value: CellRef) -> BTreeInsertResult {
    // Need to insert at this leaf node
    if key_values.len() < BTREE_ORDER {
        // Node isn't full: just add to the list (sort here is concise but maybe not as fast as scanning the list of values)
        let mut new_key_values = key_values.clone();
        new_key_values.push((key, value));
        new_key_values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

        return BTreeInsertResult::NewBTree(SafasCell::BTree(new_key_values, child_nodes.clone()).into());
    } else {
        // Node is full: need to split at the median value
        let median_key      = key_values[BTREE_MEDIAN].clone();
        let mut left_cells  = key_values[0..BTREE_MEDIAN].iter().cloned().collect::<Vec<_>>();
        let mut right_cells = key_values[(BTREE_MEDIAN+1)..key_values.len()].iter().cloned().collect::<Vec<_>>();

        // Insert into the left or right depending on the ordering of the median value with the key
        match (&*median_key.0).partial_cmp(&*key) {
            Some(Ordering::Greater) => { left_cells.push((key, value)); left_cells.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal)); },
            Some(Ordering::Less)    => { right_cells.push((key, value)); right_cells.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal)); },
            Some(Ordering::Equal)   => unreachable!(),
            None                    => return BTreeInsertResult::Err(RuntimeError::CannotCompare(median_key.0.clone(), key))
        }

        // We're at the leaf node here so there are no child nodes for the two new trees
        let left_btree  = SafasCell::BTree(left_cells, vec![]);
        let right_btree = SafasCell::BTree(right_cells, vec![]);
        
        return BTreeInsertResult::Split(left_btree.into(), right_btree.into(), median_key);
    }
}

///
/// Performs the operations required to insert a new key at a subtree node
///
fn btree_insert_new_key_at_subtree(key_values: &Vec<(CellRef, CellRef)>, child_nodes: &Vec<CellRef>, key_value: (CellRef, CellRef), left: CellRef, right: CellRef) -> BTreeInsertResult {
    // Need to insert at this leaf node
    if key_values.len() < BTREE_ORDER {
        // Node isn't full: just add to the list
        let mut new_key_values  = key_values.clone();
        let mut new_child_nodes = child_nodes.clone();

        btree_insert_new_node_and_children(&mut new_key_values, &mut new_child_nodes, key_value, left, right);

        return BTreeInsertResult::NewBTree(SafasCell::BTree(new_key_values, new_child_nodes).into());
    } else {
        // Node is full: need to split at the median value
        let median_key              = key_values[BTREE_MEDIAN].clone();
        let mut left_cells          = key_values[0..BTREE_MEDIAN].iter().cloned().collect::<Vec<_>>();
        let mut right_cells         = key_values[(BTREE_MEDIAN+1)..key_values.len()].iter().cloned().collect::<Vec<_>>();
        let mut left_children       = child_nodes[0..(BTREE_MEDIAN+1)].iter().cloned().collect::<Vec<_>>();
        let mut right_children      = child_nodes[(BTREE_MEDIAN+1)..(key_values.len()+1)].iter().cloned().collect::<Vec<_>>();

        // Insert into the left or right depending on the ordering of the median value with the key
        match (&*median_key.0).partial_cmp(&*key_value.0) {
            Some(Ordering::Greater) => btree_insert_new_node_and_children(&mut left_cells, &mut left_children, key_value, left, right),
            Some(Ordering::Less)    => btree_insert_new_node_and_children(&mut right_cells, &mut right_children, key_value, left, right),
            Some(Ordering::Equal)   => unreachable!(),
            None                    => return BTreeInsertResult::Err(RuntimeError::CannotCompare(median_key.0.clone(), key_value.0))
        }

        // We're at the leaf node here so there are no child nodes for the two new trees
        let left_btree  = SafasCell::BTree(left_cells, left_children);
        let right_btree = SafasCell::BTree(right_cells, right_children);
        
        return BTreeInsertResult::Split(left_btree.into(), right_btree.into(), median_key);
    }
}

///
/// Inserts a new key/value and left/right split trees into an existing list (without any further splitting)
///
fn btree_insert_new_node_and_children(key_values: &mut Vec<(CellRef, CellRef)>, child_nodes: &mut Vec<CellRef>, key_value: (CellRef, CellRef), left: CellRef, right: CellRef) {
    // Search for the first key that's larger than the one we're adding
    for idx in 0..=key_values.len() {
        if idx == key_values.len() {
            // Add at the end (this is the new largest key)
            key_values.push(key_value);
            child_nodes[idx] = left;
            child_nodes.push(right);

            break;
        } else {
            // Insert before the first key that's larger than this one
            match (&*key_values[idx].0).partial_cmp(&*key_value.0) {
                None | Some(Ordering::Less) | Some(Ordering::Equal) => { }
                Some(Ordering::Greater) => {
                    // Insert before this index
                    key_values.insert(idx, key_value);
                    child_nodes[idx] = right;
                    child_nodes.insert(idx, left);

                    break;
                }
            }
        }
    }
}

///
/// Creates an iterator for a btree
///
pub fn btree_iterate(btree: CellRef) -> impl Iterator<Item=(CellRef, CellRef)> {
    BTreeIterator {
        waiting: vec![BTreeIteratorStackItem::VisitLeft(btree, 0)]
    }
}

enum BTreeIteratorStackItem {
    VisitLeft(CellRef, usize),
    VisitRight(CellRef, usize)
}

///
/// Iterator that visits all of the nodes in a b-tree (in order)
///
struct BTreeIterator {
    waiting: Vec<BTreeIteratorStackItem>
}

impl Iterator for BTreeIterator {
    type Item = (CellRef, CellRef);

    fn next(&mut self) -> Option<Self::Item> {
        use self::BTreeIteratorStackItem::*;

        loop {
            match self.waiting.pop() {
                None                            => { return None; }

                Some(VisitLeft(btree, idx))     => { 
                    if let SafasCell::BTree(key_values, child_nodes) = &*btree {
                        if idx > key_values.len() {
                            // Overran the end of the nodes
                            continue;
                        } else if child_nodes.len() == 0 {
                            // No child nodes, so just visit our neighbour next
                            self.waiting.push(VisitLeft(btree.clone(), idx+1));

                            if idx < key_values.len() {
                                return Some(key_values[idx].clone());
                            } else {
                                continue;
                            }
                        } else {
                            // Visit the child nodes first, then revisit this node
                            self.waiting.push(VisitRight(btree.clone(), idx));
                            self.waiting.push(VisitLeft(child_nodes[idx].clone(), 0));

                            continue;
                        }
                    } else {
                        // Nothing to do here
                        continue;
                    }
                }

                Some(VisitRight(btree, idx))    => { 
                    if let SafasCell::BTree(key_values, child_nodes) = &*btree {
                        if child_nodes.len() == 0 {
                            // No child nodes, so just visit the main node next
                            return Some(key_values[idx].clone());
                        } else {
                            // Visit the node to the right in the list
                            self.waiting.push(VisitLeft(btree.clone(), idx+1));

                            // Return the parent node
                            if idx < key_values.len() {
                                return Some(key_values[idx].clone());
                            } else {
                                continue;
                            }
                        }
                    } else {
                        // Nothing to do here
                        continue;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::meta::*;

    use rand::prelude::*;

    #[test]
    fn insert_and_search_100_nodes() {
        let mut btree = btree_new();

        for num in 0..100 {
            let key         = SafasCell::Number(SafasNumber::Plain(num));
            let key         = CellRef::new(key);

            let value       = SafasCell::Number(SafasNumber::Plain(num + 100));
            let value       = CellRef::new(value);
            
            btree           = btree_insert(btree, (key.clone(), value.clone())).unwrap();

            let lookup_val  = btree_search(btree.clone(), key.clone()).unwrap();
            assert!(lookup_val == value);
        }
    }

    #[test]
    fn insert_and_iterate_100_nodes() {
        let mut btree = btree_new();

        for num in 0..100 {
            let key         = SafasCell::Number(SafasNumber::Plain(num));
            let key         = CellRef::new(key);

            let value       = SafasCell::Number(SafasNumber::Plain(num + 100));
            let value       = CellRef::new(value);
            
            btree           = btree_insert(btree, (key.clone(), value.clone())).unwrap();
        }

        let mut count = 0;
        for (num, (key, value)) in btree_iterate(btree).enumerate() {
            let num             = num as u128;
            let key_expected    = SafasCell::Number(SafasNumber::Plain(num));
            let key_expected    = CellRef::new(key_expected);

            let value_expected  = SafasCell::Number(SafasNumber::Plain(num + 100));
            let value_expected  = CellRef::new(value_expected);

            assert!(key == key_expected);
            assert!(value == value_expected);
            count += 1;
        }

        assert!(count == 100);
    }

    #[test]
    fn insert_and_search_100_nodes_random_keys() {
        let mut btree   = btree_new();
        let mut rng     = StdRng::seed_from_u64(42);

        for num in 0..100 {
            let key         = SafasCell::Number(SafasNumber::Plain(rng.gen_range(0, 1000)));
            let key         = CellRef::new(key);

            let value       = SafasCell::Number(SafasNumber::Plain(num + 100));
            let value       = CellRef::new(value);
            
            btree           = btree_insert(btree, (key.clone(), value.clone())).unwrap();

            let lookup_val  = btree_search(btree.clone(), key.clone()).unwrap();
            assert!(lookup_val == value);
        }
    }

    #[test]
    fn insert_replace_and_search_100_nodes() {
        let mut btree = btree_new();

        for num in 0..100 {
            let key         = SafasCell::Number(SafasNumber::Plain(num));
            let key         = CellRef::new(key);

            let value       = SafasCell::Number(SafasNumber::Plain(num + 100));
            let value       = CellRef::new(value);
            
            btree           = btree_insert(btree, (key.clone(), value.clone())).unwrap();
        }

        for num in 0..100 {
            let key         = SafasCell::Number(SafasNumber::Plain(num));
            let key         = CellRef::new(key);

            let value       = SafasCell::Number(SafasNumber::Plain(num + 200));
            let value       = CellRef::new(value);

            btree           = btree_insert(btree, (key.clone(), value.clone())).unwrap();

            let lookup_val  = btree_search(btree.clone(), key.clone()).unwrap();
            assert!(lookup_val == value);
        }
    }
}
