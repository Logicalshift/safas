use crate::meta::*;

use std::{str};
use std::ffi::{OsStr};
use include_dir::*;

const LIBRARY: Dir = include_dir!("./library");

///
/// Returns the default value for the built_ins symbol
///
pub fn builtin_library() -> CellRef {
    // The built-in library is a b-tree generated from the .sf files in the library directory
    let mut builtin_library = btree_new();

    // Search the directories
    let mut search_stack    = vec![("".to_string(), &LIBRARY)];
    while let Some((path_prefix, dir)) = search_stack.pop() {
        // Add the .sf files from this directory
        for file in dir.files() {
            // Must be a .sf file to be included
            if file.path().extension() != Some(&OsStr::new("sf")) { continue; }

            // Contents are a utf-8 string
            let content = String::from(str::from_utf8(file.contents()).expect("UTF-8 format files in the library"));

            // Get the path for this file
            let last_component  = file.path().components().last().expect("At least one path component");
            let last_component  = last_component.as_os_str().to_str().expect("String path");
            let builtin_path    = format!("{}/{}", path_prefix, last_component);

            // Add to the builtin library btree
            let builtin_path    = SafasCell::String(builtin_path);
            let content         = SafasCell::String(content);
            builtin_library     = btree_insert(builtin_library, (builtin_path.into(), content.into())).expect("Successful btree insertion");
        }

        // Also iterate through any subdirectories
        for subdir in dir.dirs() {
            let last_component  = subdir.path().components().last().expect("At least one path component");
            let last_component  = last_component.as_os_str().to_str().expect("String path");
            let new_prefix      = if path_prefix == "" { last_component.to_string() } else { format!("{}/{}", path_prefix, last_component) };

            search_stack.push((new_prefix, subdir));
        }
    }

    builtin_library
}
