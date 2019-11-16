use super::def_syntax::*;
use super::pattern_match::*;

use crate::bind::*;
use crate::meta::*;

use itertools::*;
use std::sync::*;
use std::convert::{TryFrom};

///
/// `(extend_syntax existing_syntax new_syntax_name (<pattern> <macro> ...) [prelude_statements])`
/// 
/// Takes an existing syntax (anything that binds the `syntax` keyword to a btree) and extends it with a new syntax
///
pub fn extend_syntax_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    get_expression_arguments().and_then(|ListWithTail((new_name, existing_syntax_name, patterns), statements): ListWithTail<(AtomId, AtomId, CellRef), CellRef>| {

        BindingFn::from_binding_fn(move |bindings| {
            
            // Look up the existing syntax
            let mut bindings                = bindings;
            let AtomId(existing_syntax_id)  = existing_syntax_name;
            let existing_syntax             = bindings.look_up(existing_syntax_id);

            let existing_syntax             = match existing_syntax {
                None                                    => return (bindings, Err(BindError::UnknownSymbol(name_for_atom_with_id(existing_syntax_id)))),
                Some((existing_syntax, frame_depth))    => {
                    // Rebind to the current frame if necessary
                    if frame_depth != 0 {
                        match &*existing_syntax {
                            SafasCell::Syntax(old_syntax, params) => {
                                // Rebind the syntax
                                let (new_bindings, new_syntax)  = old_syntax.rebind_from_outer_frame(bindings, params.clone(), frame_depth);
                                bindings                        = new_bindings;

                                // Update the binding if the syntax update
                                if let Some((new_syntax, new_params)) = new_syntax {
                                    SafasCell::Syntax(new_syntax, new_params).into()
                                } else {
                                    existing_syntax
                                }
                            }
                            _ => existing_syntax
                        }
                    } else {
                        existing_syntax
                    }
                }
            };

            // For syntax items, the parameter contains a btree with syntax bindings in it
            let syntax_items = match &*existing_syntax {
                SafasCell::Syntax(_syntax, params) => {
                    match btree_search(params.clone(), SafasCell::atom("syntax")) {
                        Ok(syntax_items)    => syntax_items,
                        Err(_)              => return (bindings, Err(BindError::CannotExtendSyntax(name_for_atom_with_id(existing_syntax_id))))
                    }
                },

                _ => return (bindings, Err(BindError::CannotExtendSyntax(name_for_atom_with_id(existing_syntax_id))))
            };

            if !syntax_items.is_btree() {
                return (bindings, Err(BindError::CannotExtendSyntax(name_for_atom_with_id(existing_syntax_id))))
            }

            // Return the result
            (bindings, Ok((existing_syntax, new_name, patterns.clone(), statements.clone())))
        })

    }).map_result(|(existing_syntax, new_name, patterns, statements)| {

        // Parse the arguments to the expression

        // Process the patterns (each is of the form <pattern> <macro>)
        let mut current_pattern = patterns;
        let mut macros          = vec![];
        while !current_pattern.is_nil() {
            // Each pattern is two cells, the pattern definition and the macro definition
            // Format is `(<symbol> . <pattern>) <macro>`
            let pattern_def: ListWithTail<(ListWithTail<(AtomId, ), CellRef>, CellRef), CellRef>    = ListWithTail::try_from(current_pattern)?;
            let ListWithTail((ListWithTail((symbol_name, ), pattern_def), macro_def), next_pattern) = pattern_def;

            // Compile the pattern
            let pattern_def = PatternMatch::from_pattern_as_cells(pattern_def)?;

            // Add to the macros
            macros.push((symbol_name, pattern_def, macro_def));

            // Move to the next pattern
            current_pattern = next_pattern;
        }

        // Group by symbol, so we a vec of each symbol we want to match and the corresponding macro definition
        let macros = macros.into_iter().group_by(|(AtomId(symbol_name), _pattern_def, _macro_def)| *symbol_name);
        let macros = macros.into_iter()
            .map(|(symbol, values)| {
                let values = values.into_iter().map(|(_symbol, pattern_def, macro_def)| (Arc::new(pattern_def), macro_def));
                (symbol, values.collect::<Vec<_>>())
            })
            .collect::<Vec<_>>();

        // Result of the first stage is the list of patterns
        Ok((existing_syntax, new_name, Arc::new(macros), statements))

    }).and_then(|args| {

        // Bind each of the macros and generate the syntax closure

        BindingFn::from_binding_fn(move |bindings| {

            // Fetch the values computed by the previous step
            let (existing_syntax, name, macros, statements)  = &args;

            // Bind the syntax closure
            let (mut bindings, syntax_closure)  = syntax_closure_from_macro_definitions(bindings, macros, Some(existing_syntax.clone()));
            let syntax_closure                  = match syntax_closure { Ok(syntax_closure) => syntax_closure, Err(err) => return (bindings, Err(err)) };

            // Generate a btree with the 'syntax' entry in it
            let mut btree                       = btree_new();
            btree                               = btree_insert(btree, (SafasCell::atom("syntax"), syntax_closure.syntax_btree())).unwrap();

            // Bind to the name
            let AtomId(name_id) = name;

            let syntax          = SafasCell::Syntax(Box::new(syntax_closure.syntax()), btree);
            bindings.symbols.insert(*name_id, syntax.into());
            bindings.export(*name_id);

            (bindings, Ok(NIL.clone()))

        })
    }).map_result(|_| {
        Ok(SyntaxCompiler::default())
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;
    use crate::meta::*;

    #[test]
    fn evaluate_extension_macro() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            (extend_syntax more_syntax some_syntax ((ldx #<x>) (x)))
            (more_syntax (ldx #42))"
            ).unwrap().to_string();

        assert!(val == "42");
    }

    #[test]
    fn evaluate_original_macro() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            (extend_syntax more_syntax some_syntax ((ldx #<x>) (x)))
            (more_syntax (lda #42))"
            ).unwrap().to_string();

        assert!(val == "42");
    }

    #[test]
    fn evaluate_extra_rule() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            (extend_syntax more_syntax some_syntax ((lda (<x>)) ((+ x 1))))
            (more_syntax (lda (42)))"
            ).unwrap().to_string();

        assert!(val == "43");
    }

    #[test]
    fn evaluate_original_rule() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            (extend_syntax more_syntax some_syntax ((lda (<x>)) ((+ x 1))))
            (more_syntax (lda #42))"
            ).unwrap().to_string();

        assert!(val == "42");
    }

    #[test]
    fn fetch_syntax() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            (extend_syntax more_syntax some_syntax ((ldx #<x>) (x)))
            (more_syntax syntax)"
            ).unwrap();

        let mut iter = btree_iterate(val);
        assert!(iter.next().unwrap().0 == SafasCell::atom("lda"));
        assert!(iter.next().unwrap().0 == SafasCell::atom("ldx"));
        assert!(iter.next() == None);
    }
}
