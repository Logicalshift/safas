use crate::bind::*;
use crate::meta::*;
use crate::exec::*;
use crate::parse::*;

use std::path::{Path, Component};
use std::fs;

///
/// Reads a value retrieved using look_up from a set of symbol bindings as a series of strings
///
fn read_strings(value: Option<(CellRef, u32)>, frame: &Frame) -> Vec<String> {
    value.map(|(import_path, depth)| {
            match (&*import_path, depth) {
                (SafasCell::FrameReference(cell_id, 0, _), 0) => {
                    // Fetch from the frame (note that external frames aren't supported here: any syntax should try to pull in import_paths itself)
                    let import_path = frame.cells[*cell_id].clone();
                    import_path.to_vec().unwrap_or_else(|| vec![])
                        .into_iter()
                        .filter_map(|cell| match &*cell {
                            SafasCell::String(path) => Some(path.clone()),
                            _                       => None
                        })
                        .collect()
                },

                (SafasCell::List(_, _), _) => {
                    import_path.to_vec().unwrap_or_else(|| vec![])
                        .into_iter()
                        .filter_map(|cell| match &*cell {
                            SafasCell::String(path) => Some(path.clone()),
                            _                       => None
                        })
                        .collect()
                }

                _ => vec![]
            }
        }).unwrap_or_else(|| vec![])
}

///
/// Imports a file using the specified symbol bindings as the environment (for non-absolute paths, the `import_path` atom can be defined
/// to a list of places to look. `built_ins` can be used to supply a set of built-in files as strings that are used if the file can't
/// be found on the import path)
///
pub fn import_file(filename: &str, bindings: SymbolBindings, frame: Frame) -> (CellRef, SymbolBindings, Frame) {
    // The import_path atom can be defined to a list of paths to try to read imported files from
    let import_path = get_id_for_atom_with_name("import_path");
    let built_ins   = get_id_for_atom_with_name("built_ins");

    let import_path = bindings.look_up(import_path);
    let built_ins   = bindings.look_up(built_ins);

    // Convert the import path into a list of strings
    let import_path = read_strings(import_path, &frame);

    // Try to open the file by searching the input paths
    let file_path = Path::new(filename);

    let file_path = if file_path.is_absolute() || file_path.components().nth(0) == Some(Component::CurDir) {
        // Absolute paths are not searched for: we'll just return it as existing if 
        if file_path.is_file() {
            Some(file_path.to_path_buf())
        } else {
            None
        }
    } else {
        // Relative paths are searched via the import paths
        let mut found = None;
        for import_prefix in import_path.iter() {
            let import_prefix = Path::new(import_prefix);

            // To be a valid import path, the path must contain
            if import_prefix.is_absolute() || import_prefix.components().nth(0) == Some(Component::CurDir) {
                let try_path = import_prefix.join(file_path);
                if try_path.is_file() {
                    found = Some(try_path);
                }
            }
        }

        found
    };

    // Read the file contents
    let (file_content, file_path) = if let Some(file_path) = file_path {
        let content = fs::read_to_string(file_path.as_path());
        let content = match content { Ok(content) => content, Err(_err) => { return (RuntimeError::IOError.into(), bindings, frame); } };

        (content, file_path)
    } else {
        // File not found (TODO: search through the builtins)
        return (RuntimeError::FileNotFound.into(), bindings, frame);
    };

    // Parse the file
    let file_content = parse_safas(&mut TokenReadBuffer::new(file_content.chars()), FileLocation::new(&file_path.to_string_lossy()));
    let file_content = match file_content { Ok(content) => content, Err(err) => { return (RuntimeError::ParseError(err).into(), bindings, frame); } };

    // Evaluate the file
    eval_statements(file_content, NIL.clone(), bindings, frame)
}
