use crate::bind::*;
use crate::meta::*;
use crate::exec::*;
use crate::parse::*;

use std::path::{Path, PathBuf, Component};
use std::convert::{TryFrom};
use std::fs;

const DEFAULT_EXTENSION: &str = "sf";

///
/// Represents where an imported file can be loaded from
///
pub enum ImportFile {
    /// Import file was not found
    NotFound,

    /// File to be loaded from a particular path
    FromPath(PathBuf),

    // File found in the builtins specified in the bindings (two strings are the path name and the file data)
    BuiltIn(String, String)
}

impl Default for ImportFile {
    fn default() -> Self {
        ImportFile::NotFound
    }
}

///
/// Reads a value retrieved using look_up from a set of symbol bindings as a series of strings
///
fn read_strings(value: Option<(CellRef, u32)>) -> Vec<String> {
    value.map(|(import_path, depth)| {
            match (&*import_path, depth) {
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
/// Attempts to locate where a particular file to be imported can be loaded from
/// 
/// The `filename` is the name provided by the user/program for the file to import. The `bindings` are used to
/// retrieve the environment for the import: in particular, the `import_path` and `built_ins` atoms should be
/// defined to be the list of paths to search for imports and the list of built-in definitions respectively.
/// Search order follows the import paths first then looks at the builtins.
///
pub fn locate_import_file(filename: &str, bindings: &SymbolBindings, allow_relative: bool) -> ImportFile {
    // The import_path atom can be defined to a list of paths to try to read imported files from
    let import_path = get_id_for_atom_with_name("import_path");
    let built_ins   = get_id_for_atom_with_name("built_ins");

    let import_path = bindings.look_up(import_path);
    let built_ins   = bindings.look_up(built_ins);

    // Convert the import path into a list of strings
    let import_path = read_strings(import_path);

    // Try to open the file by searching the input paths
    let file_path = Path::new(filename);

    let file_path = if allow_relative && !file_path.is_absolute() && file_path.is_file() {
        // If the current directory is allowed then try the current directory
        Some(file_path.to_path_buf())
    } else if allow_relative && !file_path.is_absolute() && file_path.extension().is_none() && file_path.with_extension(DEFAULT_EXTENSION).is_file() {
        // We also add the '.sf' extension if it's not already set
        Some(file_path.with_extension(DEFAULT_EXTENSION))
    } else if file_path.is_absolute() || file_path.components().nth(0) == Some(Component::CurDir) || file_path.components().nth(0) == Some(Component::ParentDir) {
        // Absolute paths are not searched for: we'll just return it as existing if an absolute path or a path starting at the current directory is used
        if file_path.is_file() {
            Some(file_path.to_path_buf())
        } else if file_path.extension().is_none() && file_path.with_extension(DEFAULT_EXTENSION).is_file() {
            Some(file_path.with_extension(DEFAULT_EXTENSION))
        } else {
            None
        }
    } else {
        // Relative paths are searched via the import paths
        let mut found = None;
        for import_prefix in import_path.iter() {
            let import_prefix = Path::new(import_prefix);

            // To be a valid import path, the path must indicate exactly where it's located
            if import_prefix.is_absolute() || import_prefix.components().nth(0) == Some(Component::CurDir) || import_prefix.components().nth(0) == Some(Component::ParentDir) {
                let try_path = import_prefix.join(file_path);
                if try_path.is_file() {
                    found = Some(try_path);
                } else if try_path.extension().is_none() && try_path.with_extension(DEFAULT_EXTENSION).is_file() {
                    found = Some(try_path.with_extension(DEFAULT_EXTENSION));
                }
            }
        }

        found
    };

    if let Some(file_path) = file_path {
        // Found as an actual file
        ImportFile::FromPath(file_path)
    } else if let Some((built_ins, _)) = built_ins {
        // Search the built-ins from the bindings (try with an extra .sf suffix if necessary)
        let built_in = btree_search(built_ins.clone(), CellRef::new(SafasCell::String(filename.to_string())));
        let built_in = if let Ok(built_in) = built_in {
            if built_in.is_nil() {
                // Try with a .sf extension
                btree_search(built_ins, CellRef::new(SafasCell::String(format!("{}.sf", filename))))
            } else {
                // Already found
                Ok(built_in)
            }
        } else {
            // Error
            built_in
        };

        // If a match was found in the b-tree then use that as the file to load
        match built_in {
            Ok(definition) => {
                if let SafasCell::String(definition) = &*definition {
                    ImportFile::BuiltIn(filename.to_string(), definition.clone())
                } else {
                    ImportFile::NotFound
                }
            },

            Err(_) => {
                ImportFile::NotFound
            }
        }
    } else {
        ImportFile::NotFound
    }
}

///
/// Imports a file using the specified symbol bindings as the environment (for non-absolute paths, the `import_path` atom can be defined
/// to a list of places to look. `built_ins` can be used to supply a set of built-in files as strings that are used if the file can't
/// be found on the import path)
///
pub fn import_file(filename: &str, bindings: SymbolBindings, frame: Frame, allow_relative: bool) -> (CellRef, SymbolBindings, Frame) {
    let file_path = locate_import_file(filename, &bindings, allow_relative);

    // Read the file contents
    let (file_content, file_path) = match file_path {
        ImportFile::FromPath(file_path) => {
            let content = fs::read_to_string(file_path.as_path());
            let content = match content { Ok(content) => content, Err(_err) => { return (RuntimeError::IOError.into(), bindings, frame); } };

            (content, String::from(file_path.to_string_lossy()))
        },

        ImportFile::BuiltIn(file_path, content) => {
            (content, file_path)
        },

        ImportFile::NotFound => {
            // File not found
            return (RuntimeError::FileNotFound(filename.to_string()).into(), bindings, frame);
        }
    };

    // Parse the file
    let file_content = parse_safas(&mut TokenReadBuffer::new(file_content.chars()), FileLocation::new(&file_path));
    let file_content = match file_content { Ok(content) => content, Err(err) => { return (RuntimeError::ParseError(err).into(), bindings, frame); } };

    // Evaluate the file
    let bindings                    = bindings.push_interior_frame();
    let (result, bindings, frame)   = eval_statements(file_content, NIL.clone(), bindings, frame);

    let (bindings, _imports)        = bindings.pop();
    (result, bindings, frame)
}

struct LocateImportFile;

impl BindingMonad for LocateImportFile {
    type Binding = (String, ImportFile);

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        // Fetch the arguments to this expression
        let args = match bindings.args.as_ref() { Some(args) => args.clone(), None => return (bindings, Err(BindError::MissingArgument)) };
        let args = ListTuple::<(CellValue<String>, )>::try_from(args);
        let args = match args { Ok(args) => args, Err(err) => return (bindings, Err(err.into())) };

        let ListTuple((CellValue(filename), )) = args;

        // Locate the file. Implicit relative paths are not allowed when using the (import) syntax
        let file_path = locate_import_file(&filename, &bindings, false);

        match file_path {
            // Import file could not be found
            ImportFile::NotFound    => (bindings, Err(BindError::FileNotFound(filename))),

            // Return the file location
            _                       => (bindings, Ok((filename, file_path)))
        }
    }

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        // No pre-binding is performed with the import files
        (bindings, ("".to_string(), ImportFile::NotFound))
    }
}

///
/// Creates the compiler for the import keyword
/// 
/// `(import "foo")` attempts to import the file `foo.sf` from the current set of search paths.
///
pub fn import_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    LocateImportFile.map_result(|(filename, file_path)| {
        // Read the file content
        let file_content = match file_path {
            ImportFile::NotFound                => "".to_string(),
            ImportFile::FromPath(path)          => fs::read_to_string(path.as_path()).map_err(|_io| BindError::IOError)?,
            ImportFile::BuiltIn(_name, data)    => data
        };

        // Parse the file content
        let parsed = parse_safas(&mut TokenReadBuffer::new(file_content.chars()), FileLocation::new(&filename))?;

        // Pass through the parsed content
        Ok((filename, parsed))
    }).and_then(|(_filename, parsed_input)| {

        // Bind the result
        BindingFn::from_binding_fn(move |bindings| {
            // The input is a list of statements
            let mut bindings    = bindings;
            let mut pos         = &*parsed_input;

            // Pre-bind each of the statements
            while let SafasCell::List(statement, next) = pos {
                let (next_bindings, _result)    = pre_bind_statement(statement.clone(), bindings);
                bindings                        = next_bindings;

                pos                             = &*next;
            }

            // Bind each statement in turn
            let mut bound_statements    = vec![];
            let mut pos                 = &*parsed_input;

            while let SafasCell::List(statement, next) = pos {
                match bind_statement(statement.clone(), bindings) {
                    Ok((result, new_bindings))  => { bindings = new_bindings; bound_statements.push(result); }
                    Err((err, new_bindings))    => { return (new_bindings, Err(err)); }
                }

                pos                             = &*next;
            }

            // Result is the list of bound cells
            (bindings, Ok(SafasCell::list_with_cells(bound_statements).into()))
        })

    }).map(|binding: CellRef| {
        // Create a copy of binidng for ourselves
        let binding = binding.clone();

        let compile = |binding: CellRef| {
            let binding = binding.clone();

            // Start with some empty actions for the import
            let mut actions = CompiledActions::empty();

            // The binding is the list of statements from the import
            let mut pos = &*binding;

            while let SafasCell::List(statement, next) = pos {
                // Compile the next statement
                let compiled = compile_statement(statement.clone())?;

                // Add the actions to the compiled result
                actions.extend(compiled);

                // Move to the next statement
                pos = &*next;
            }

            // Return the final result
            Ok(actions)
        };

        SyntaxCompiler::with_compiler(compile, binding)
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn load_builtin_library() {
        eval(
            "(import \"standard/default.sf\")"
        ).unwrap();
    }

    #[test]
    fn load_6502() {
        eval(
            "(import \"standard/default.sf\")
            (import \"cpu/6502\")"
        ).unwrap();
    }

    #[test]
    fn load_65c02() {
        eval(
            "(import \"standard/default.sf\")
            (import \"cpu/65c02\")"
        ).unwrap();
    }
}
