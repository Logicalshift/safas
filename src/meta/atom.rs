use std::sync::*;
use std::collections::{HashMap};

lazy_static! {
    static ref ATOM_IDS:        Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
    static ref ATOM_NAMES:      Mutex<HashMap<u64, String>> = Mutex::new(HashMap::new());
    static ref NEXT_ATOM_ID:    Mutex<u64>                  = Mutex::new(1);
}

///
/// Retrieves the ID for the atom with the specified name
///
pub fn get_id_for_atom_with_name(name: &str) -> u64 {
    let mut atom_ids = ATOM_IDS.lock().unwrap();

    if let Some(id) = atom_ids.get(name) {
        // Atom already has an assigned ID
        *id
    } else {
        // Assign a new atom ID
        let mut atom_names      = ATOM_NAMES.lock().unwrap();
        let mut next_atom_id    = NEXT_ATOM_ID.lock().unwrap();

        let id                  = *next_atom_id;
        (*next_atom_id)         += 1;

        atom_ids.insert(String::from(name), id);
        atom_names.insert(id, String::from(name));

        id
    }
}

/// 
/// Retrieves the name for the atom with the specified ID
/// 
pub fn name_for_atom_with_id(id: u64) -> String {
    let atom_names = ATOM_NAMES.lock().unwrap();

    if let Some(name) = atom_names.get(&id) {
        name.clone()
    } else {
        format!("##a#{}", id)
    }
}
