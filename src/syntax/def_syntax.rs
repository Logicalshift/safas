use super::syntax_symbol::*;
use super::syntax_closure::*;
use super::pattern_match::*;

use crate::bind::*;
use crate::meta::*;

use itertools::*;
use std::sync::*;
use std::collections::{HashMap};
use std::convert::*;

///
/// The (def_syntax) keyword, expressed as a binding monad
/// 
/// Syntax is defined using:
/// 
/// ```(def_syntax <name> (<pattern> <macro> ...) [prelude_statements])```
/// 
/// <name> becomes a syntax item in the binding. We can use the new syntax like this:
/// 
/// ```(<name> <statements>)```
/// 
/// Every syntax we define contains a special `syntax` keyword that can be used to retrieve the
/// bindings it contains (so it's possible to extend it). This can be accessed like this: 
/// `(<name> syntax)`
///
pub fn def_syntax_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    get_expression_arguments().map_result(|args: ListWithTail<(AtomId, CellRef), CellRef>| {

        // First step: parse the arguments to the expression

        // Fetch the arguments
        let ListWithTail((name, patterns), statements) = args;

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
        Ok((name, Arc::new(macros), statements))

    }).and_then(|args| {

        // Second step: bind each of the macros and generate the syntax item

        BindingFn::from_binding_fn(move |bindings| {

            // Fetch the values computed by the previous step
            let (name, macros, statements)  = &args;

            // Bind the macros in an inner frame
            let mut evaluation_bindings     = bindings.push_new_frame();
            let mut symbol_syntax           = vec![];

            // Macros can reference each other. Only back-references are allowed so we can bind them properly
            // Initially all symbols generate errors
            for (symbol_id, _) in macros.iter() {
                // Symbols are intially bound to some syntax that generates an error
                let error = BindingFn::from_binding_fn(|bindings| -> (SymbolBindings, Result<CellRef, BindError>) { (bindings, Err(BindError::ForwardReferencesNotAllowed)) })
                    .map(|_| SyntaxCompiler::with_compiler(|_| Err(BindError::ForwardReferencesNotAllowed), NIL.clone()));

                evaluation_bindings.symbols.insert(*symbol_id, SafasCell::Syntax(Box::new(error), NIL.clone()).into());
            }

            for (symbol_id, symbol_patterns) in macros.iter() {
                // bound_patterns will store the patterns that will be bound by this syntax
                let mut bound_patterns          = vec![];

                for (pattern_def, macro_def) in symbol_patterns.iter() {
                    let pattern_def             = Arc::clone(pattern_def);
                    let macro_def               = Arc::clone(macro_def);

                    // Create an inner frame with the values for this macro
                    let mut macro_bindings      = evaluation_bindings.push_interior_frame();

                    // Bind the arguments for the pattern
                    let mut pattern_cells = vec![];
                    for AtomId(arg_atom_id) in pattern_def.bindings() {
                        // Create a new cell for this atom
                        let arg_cell            = macro_bindings.alloc_cell();
                        let arg_cell: CellRef   = SafasCell::FrameReference(arg_cell, 0, ReferenceType::Value).into();

                        // Add to the bindings and the list of cells for this pattern
                        macro_bindings.symbols.insert(arg_atom_id, arg_cell.clone());
                        pattern_cells.push(arg_cell);
                    }
                    
                    // Bind the macro definition (which is a series of statements)
                    let macro_def               = macro_def.to_vec();
                    let macro_def               = match macro_def { Some(def) => def, None => return (macro_bindings.pop().0.pop().0, Err(BindError::SyntaxExpectingList)) };

                    // Prebind each statement
                    for macro_statement in macro_def.iter() {
                        let (new_bindings, _)   = pre_bind_statement(Arc::clone(macro_statement), macro_bindings);
                        macro_bindings          = new_bindings;
                    }

                    // Finish binding them
                    let mut bind_result         = vec![];
                    for macro_statement in macro_def.into_iter() {
                        // Bind this statement
                        let bound_statement     = bind_statement(macro_statement, macro_bindings);
                        let (new_bindings, bound_statement) = match bound_statement { 
                            Ok((result, macro_bindings))    => ((macro_bindings, result)), 
                            Err((err, macro_bindings))      => { return (macro_bindings.pop().0.pop().0, Err(err)); }
                        };

                        // Store in the result
                        macro_bindings = new_bindings;
                        bind_result.push(bound_statement);
                    }

                    // Store in the results
                    bound_patterns.push((pattern_def, pattern_cells, SafasCell::list_with_cells(bind_result).into()));

                    // Revert the inner frame
                    let (new_bindings, _)       = macro_bindings.pop();
                    evaluation_bindings         = new_bindings;
                }

                // Create a syntax symbol
                let symbol = SyntaxSymbol::new(bound_patterns);
                let symbol = Arc::new(symbol);

                // Define this as our symbol name
                evaluation_bindings.symbols.insert(*symbol_id, SafasCell::Syntax(Box::new(SyntaxSymbol::syntax(symbol.clone())), NIL.clone()).into());
                symbol_syntax.push((AtomId(*symbol_id), symbol))
            }

            // Pop the evaluation frame
            let (mut bindings, imports) = evaluation_bindings.pop();

            // Generate the imported symbol list for the macros
            let mut cell_imports        = HashMap::new();
            for (symbol_value, import_into_cell_id) in imports.into_iter() {
                match &*symbol_value {
                    SafasCell::FrameReference(_our_cell_id, 0, _type) => {
                        // Cell from this frame
                        cell_imports.insert(import_into_cell_id, symbol_value);
                    },

                    SafasCell::FrameReference(their_cell_id, frame_count, their_type) => {
                        // Import from a parent frame
                        let our_cell_id = bindings.alloc_cell();
                        bindings.import(SafasCell::FrameReference(*their_cell_id, *frame_count, *their_type).into(), our_cell_id);
                        cell_imports.insert(import_into_cell_id, SafasCell::FrameReference(our_cell_id, 0, *their_type).into());
                    },

                    _ => panic!("Don't know how to import this type of symbol")
                }
            }

            // Build a syntax closure from the arguments (these are currently bound to the current environment so they
            // can't be passed outside of the current function)
            let syntax_closure  = SyntaxClosure::new(symbol_syntax, Arc::new(cell_imports));

            // Bind to the name
            let AtomId(name_id) = name;
            let syntax          = SafasCell::Syntax(Box::new(syntax_closure.syntax()), NIL.clone());
            bindings.symbols.insert(*name_id, syntax.into());
            bindings.export(*name_id);

            (bindings, Ok(NIL.clone()))

        })
    }).map(|_| SyntaxCompiler::default())
}

#[cfg(test)]
mod test {
    use crate::*;

    #[test]
    fn evaluate_def_syntax() {
        eval("(def_syntax x ((lda #<x>) (d x)))").unwrap().to_string();
    }

    #[test]
    fn evaluate_macro() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "3");
    }

    #[test]
    fn choose_syntax_1() {
        let val = eval(
            "(def_syntax some_syntax ( (lda #<x>) ((list 1 x))   (lda <x>) ((list 2 x)) ))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "(1 3)");
    }

    #[test]
    fn choose_syntax_2() {
        let val = eval(
            "(def_syntax some_syntax ( (lda #<x>) ((list 1 x))   (lda <x>) ((list 2 x)) ))
            (some_syntax (lda 3))"
            ).unwrap().to_string();

        assert!(val == "(2 3)");
    }

    #[test]
    fn choose_syntax_3() {
        let val = eval(
            "(def_syntax some_syntax ( (lda #<x>) ((list 1 x))   (ldx <x>) ((list 2 x)) ))
            (some_syntax (ldx 3))"
            ).unwrap().to_string();

        assert!(val == "(2 3)");
    }

    #[test]
    fn read_external_binding() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn macro_in_macro() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (def_syntax other_syntax ((ld #<x>) ( (some_syntax (lda #x)) )))
            (other_syntax (ld #3))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn macro_in_function() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) (x)))
            ((fun () (some_syntax (lda #3))))"
            ).unwrap().to_string();

        assert!(val == "3");
    }

    #[test]
    fn read_external_binding_in_function() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            ((fun () (some_syntax (lda #3))))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn macro_in_macro_in_function() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (def_syntax other_syntax ((ld #<x>) ( (some_syntax (lda #x)) )))
            ((fun () (other_syntax (ld #3))))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn external_bindings_are_hygenic() {
        let val = eval(
            "(def z 4)
            (def_syntax some_syntax ((lda #<x>) ((list x z))))
            (def z 5)
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "(3 4)");
    }

    #[test]
    fn define_value_in_macro() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) ((def y x) y)))
            (some_syntax (lda #3))"
            ).unwrap().to_string();

        assert!(val == "3");
    }

    #[test]
    fn define_value_in_macro_list() {
        let val = eval(
            "(def_syntax some_syntax ((lda #<x>) ((def y x) y)))
            (some_syntax (list (lda #3) (lda #4) (lda #5)))"
            ).unwrap().to_string();

        assert!(val == "(3 4 5)");
    }

    #[test]
    fn define_function_in_syntax() {
        let val = eval(
            "(def_syntax some_syntax (
                (make_fun <x>) ((fun () x))
            ))
            (some_syntax ((make_fun 2)))"
            ).unwrap().to_string();

        println!("{:?}", val);

        assert!(val == "2");
    }

    #[test]
    fn nested_syntax() {
        let val = eval(
            "(def_syntax some_syntax (
                (lda (<indirect>, X)) ((list indirect))
            ))
            (some_syntax (lda (2, X)))"
            ).unwrap().to_string();

        println!("{:?}", val);

        assert!(val == "(2)");
    }

    #[test]
    fn syntax_monad_1() {
        let val = eval(
            "(def_syntax some_syntax (
                (make_list <x>) ((list 1 x))
            ))
            (some_syntax (make_list (wrap 2)))"
            ).unwrap();

        println!("{:?}", val.to_string());

        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2)))".to_string());
    }

    #[test]
    fn syntax_monad_2() {
        let val = eval(
            "(def_syntax some_syntax (
                (make_list <x> <y>) ((list 1 x y))
            ))
            (some_syntax (make_list (wrap 2) (wrap 3)))"
            ).unwrap();

        println!("{:?}", val.to_string());

        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2 3)))".to_string());
    }

    #[test]
    fn syntax_monad_3() {
        let val = eval(
            "(def_syntax some_syntax (
                (make_list <x> <y>) ((list 1 x y))
            ))
            (some_syntax (make_list 2 (wrap 3)))"
            ).unwrap();

        println!("{:?}", val.to_string());

        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2 3)))".to_string());
    }

    #[test]
    fn syntax_monad_4() {
        let val = eval(
            "(def_syntax some_syntax (
                (make_list <x> <y>) ((list 1 x y))
            ))
            (some_syntax 
                (make_list 1 (wrap 2))
                (make_list 2 (wrap 3))
            )"
            ).unwrap();

        println!("{:?}", val.to_string());

        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2 3)))".to_string());
    }

    #[test]
    fn syntax_monad_5() {
        let val = eval(
            "(def_syntax some_syntax (
                (make_list <x> <y>) ((wrap (list 1 x y)))
            ))
            (some_syntax 
                (make_list 2 (wrap 3))
            )"
            ).unwrap();

        println!("{:?}", val.to_string());

        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2 3)))".to_string());
    }

    #[test]
    fn syntax_monad_6() {
        let val = eval(
            "(def_syntax some_syntax (
                (make_list <x> <y>) ((wrap (list 1 x y)))
            ))
            (some_syntax 
                (make_list 1 (wrap 2))
                (make_list 2 (wrap 3))
            )"
            ).unwrap();

        println!("{:?}", val.to_string());

        assert!(val.reference_type() == ReferenceType::Monad);
        assert!(val.to_string() == "monad#()#(flat_map: ##wrap((1 2 3)))".to_string());
    }

    #[test]
    fn bind_to_value_outside_syntax() {
        let val = eval(
            "(def y 123)
            (def_syntax some_syntax (
                (make_list <x>) ((list y x))
            ))
            (def y 3)
            (some_syntax 
                (make_list 2)
            )"
            ).unwrap().to_string();

        println!("{:?}", val);

        assert!(val == "(123 2)".to_string());
    }

    #[test]
    fn use_syntax_in_closure() {
        let val = eval(
            "(def y 123)
            (def_syntax some_syntax (
                (make_list <x>) ((list y x))
            ))
            (some_syntax 
                ((fun () (make_list 2)))
            )"
            ).unwrap().to_string();

        println!("{:?}", val);

        assert!(val == "(123 2)".to_string());
    }


    #[test]
    fn retrieve_syntax_btree() {
        let val = eval(
            "(def y 123)
            (def_syntax some_syntax (
                (make_list <x>) ((list y x))
            ))
            (some_syntax syntax)"
            ).unwrap().to_string();

        println!("{:?}", val);

        assert!(val == "btree#(\n  make_list -> compile###syntax###()\n)".to_string());
    }
}
