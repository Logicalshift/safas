use super::symbol_bindings::*;
use super::bind_error::*;
use super::binding_monad::*;
use super::binding_monad_sugar::*;
use super::syntax_compiler::*;
use super::compile_statement::*;

use crate::meta::*;
use crate::exec::*;
use crate::syntax::*;

use std::sync::*;
use std::result::{Result};

lazy_static! {
    static ref WRAP_KEYWORD: CellRef = SafasCell::Syntax(wrap_keyword(), NIL.clone()).into();
}

///
/// Binds a statement to the value used for compiling
/// 
/// Binding is the process of swapping the symbolic values of the items in a statement with their 'actual'
/// values from a binding environment. A binding environment is represented by the `SymbolBindings` structure.
/// 
/// The binding environment is set up by the `pre_bind_statement` call for a given set of statements.
/// 
/// Most of the types of symbol are constants: ie, the number `1` is just bound to itself. Atoms are a bit more
/// interesting: they are looked up in the environment and bound to whatever value is found there, for example
/// to a cell in the current frame.
/// 
/// Lists are processed as function calls or can be processed in a custom way if their first value is bound to
/// a `Syntax` item.
/// 
/// Atoms that are bound to `Syntax` values can provide custom binding behaviour. They can either appear alone
/// or in a list, and can return any value for their binding. They bind to a new list starting with the syntax
/// item they're bound to (and followed by whatever their binding function returns). The compiler uses this to
/// invoke their `generate_actions` funciton.
/// 
/// Atoms that are bound to `Monad` values or functions that evaluate to `Monad` values (whose `reference_type` is
/// `ReturnsMonad`) are further treated specially. Rather than evaluating them as straight values, their wrapped
/// value is obtained by rewriting the statement so that their `flat_map` method is called. This allows them to
/// be treated as if they are normal values (this is equivalent to `do` syntax in other languages). 
/// 
/// For example: `(wrap 1)` creates a monad wrapping the value `1`. If we use this as a parameter to a function
/// call - for instance `(list (wrap 1) 2)` - the result is not a list containing the monad and the number 2
/// but instead is a monad containing a list `(1 2)`. This allows for a very natural code style when building
/// assembler programs as well as a way to build similar code structures.
/// 
/// SAFAS is typeless, so there is only one `wrap` function needed.
///
pub fn bind_statement(source: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    use self::SafasCell::*;

    match &*source {
        // Lists are processed according to their first value
        List(car, cdr)  => { bind_list_statement(Arc::clone(car), Arc::clone(cdr), bindings) }

        // Atoms bind to their atom value
        Atom(atom_id)   => {
            // Look up the value for this symbol
            let symbol_value = bindings.look_up(*atom_id);

            if let Some((symbol_value, symbol_level)) = symbol_value {
                use self::SafasCell::*;

                match &*symbol_value {
                    Nil                                         |
                    Any(_)                                      |
                    Number(_)                                   |
                    Atom(_)                                     |
                    String(_)                                   |
                    BitCode(_)                                  |
                    Char(_)                                     |
                    List(_, _)                                  |
                    Monad(_, _)                                 |
                    Error(_)                                    |
                    FrameMonad(_)                               => Ok((symbol_value, bindings)),

                    FrameReference(cell_num, frame, cell_type)  => {
                        let (cell_num, frame) = (*cell_num, *frame);
                        if frame == 0 {
                            // Local symbol
                            Ok((symbol_value, bindings))
                        } else {
                            // Import from a parent frame
                            let mut bindings    = bindings;
                            let local_cell_id   = bindings.alloc_cell();
                            bindings.import(SafasCell::FrameReference(cell_num, frame, *cell_type).into(), local_cell_id);

                            Ok((SafasCell::FrameReference(local_cell_id, 0, *cell_type).into(), bindings))
                        }
                    },

                    Syntax(syntax_compiler, parameter)     => {
                        // If we're on a different syntax level, try rebinding the monad (the syntax might need to import symbols from an outer frame, for example)
                        let mut bindings = bindings;

                        if symbol_level != 0 {
                            // Try to rebind the syntax from an outer frame
                            let (mut new_bindings, rebound_monad) = syntax_compiler.binding_monad.rebind_from_outer_frame(bindings, symbol_level);

                            // If the compiler rebinds itself...
                            if let Some(rebound_monad) = rebound_monad {
                                // ... create a new syntax using the rebound binding monad
                                let new_syntax = SyntaxCompiler {
                                    binding_monad:      rebound_monad,
                                    generate_actions:   Arc::clone(&syntax_compiler.generate_actions)
                                };

                                // Add to the symbols in the current bindings so we don't need to rebind the syntax multiple times
                                let new_syntax = Syntax(new_syntax, parameter.clone()).into();
                                new_bindings.symbols.insert(*atom_id, new_syntax);
                                new_bindings.export(*atom_id);

                                // Re-evaluate this binding (as we insert the binding at the current level we won't rebind the next time through)
                                return bind_statement(source, new_bindings);
                            }

                            // Update the bindings to apply the effects of the rebinding
                            bindings = new_bindings
                        }

                        let mut bindings        = bindings.push_interior_frame();
                        bindings.args           = None;
                        bindings.depth          = Some(symbol_level);
                        let (bindings, bound)   = syntax_compiler.binding_monad.bind(bindings);
                        let (bindings, imports) = bindings.pop();

                        if imports.len() > 0 { panic!("Should not need to import symbols into an interior frame"); }

                        match bound {
                            Ok(bound)       => Ok((SafasCell::List(symbol_value, bound).into(), bindings)),
                            Err(error)      => Err((error, bindings))
                        }
                    }
                }
            } else {
                // Not a valid symbol
                Err((BindError::UnknownSymbol, bindings))
            }
        }

        // Normal values just get loaded into cell 0
        _other          => { Ok((source, bindings)) }
    }
}

///
/// Binds a list statement, like `(cons 1 2)`
///
fn bind_list_statement(car: CellRef, cdr: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    use self::SafasCell::*;

    // Action depends on the type of car
    match &*car {
        // Atoms can call a function or act as syntax in this context
        Atom(atom_id)   => {
            use self::SafasCell::*;
            let symbol_value = bindings.look_up(*atom_id);

            if let Some((symbol_value, symbol_level)) = symbol_value {
                match &*symbol_value {
                    // Constant values just load that value and call it
                    Nil                                         |
                    Any(_)                                      |
                    Number(_)                                   |
                    Atom(_)                                     |
                    String(_)                                   |
                    BitCode(_)                                  |
                    Char(_)                                     |
                    Monad(_, _)                                 |
                    Error(_)                                    |
                    FrameMonad(_)                               => { bind_call(symbol_value, cdr, bindings) },

                    // Lists bind themselves before calling
                    List(_, _)                                  => { let (bound_symbol, bindings) = bind_statement(symbol_value, bindings)?; bind_call(bound_symbol, cdr, bindings) }

                    // Frame references load the value from the frame and call that
                    FrameReference(_cell_num, _frame, _type)    => { let (actions, bindings) = bind_statement(car, bindings)?; bind_call(actions, cdr, bindings) }
                    
                    // Syntax items apply the actions specified in their binding monad
                    Syntax(syntax_compiler, parameter)     => {
                        // If we're on a different syntax level, try rebinding the monad (the syntax might need to import symbols from an outer frame, for example)
                        let mut bindings = bindings;

                        if symbol_level != 0 {
                            // Try to rebind the syntax from an outer frame
                            let (mut new_bindings, rebound_monad) = syntax_compiler.binding_monad.rebind_from_outer_frame(bindings, symbol_level);

                            // If the compiler rebinds itself...
                            if let Some(rebound_monad) = rebound_monad {
                                // ... create a new syntax using the rebound binding monad
                                let new_syntax = SyntaxCompiler {
                                    binding_monad:      rebound_monad,
                                    generate_actions:   Arc::clone(&syntax_compiler.generate_actions)
                                };

                                // Add the bound syntax to the symbols in the current bindings so we don't need to rebind the syntax multiple times
                                let new_syntax = Syntax(new_syntax, parameter.clone()).into();
                                new_bindings.symbols.insert(*atom_id, new_syntax);
                                new_bindings.export(*atom_id);

                                // Re-evaluate this binding (as we insert the binding at the current level we won't rebind the next time through)
                                return bind_list_statement(car, cdr, new_bindings);
                            }

                            // Update the bindings to apply the effects of the rebinding
                            bindings = new_bindings
                        }

                        let mut bindings        = bindings.push_interior_frame();
                        bindings.args           = Some(cdr);
                        bindings.depth          = Some(symbol_level);
                        let (bindings, bound)   = syntax_compiler.binding_monad.bind(bindings);
                        let (bindings, imports) = bindings.pop();

                        if imports.len() > 0 { panic!("Should not need to import symbols into an interior frame"); }

                        match bound {
                            Ok(bound)       => Ok((SafasCell::List(symbol_value, bound).into(), bindings)),
                            Err(error)      => Err((error, bindings))
                        }
                    }
                } 
            } else {
                return Err((BindError::UnknownSymbol, bindings));
            }
        },

        // Default action is to evaluate the first item as a statement and call it
        _other          => {
            let (actions, bindings) = bind_statement(car, bindings)?;
            bind_call(actions, cdr, bindings)
        }
    }
}

///
/// Binds a call function, given the value that evaluates to the function
///
fn bind_call(load_fn: CellRef, args: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    let mut bindings = bindings;

    // The function might be generated by a monad
    if load_fn.reference_type() == ReferenceType::Monad {
        return bind_monad(vec![], load_fn, args, bindings);
    }

    // Start by pushing the function value onto the stack (we'll pop it later on to call the function)
    let mut bound       = vec![load_fn];

    // Push the arguments
    let mut next_arg    = args;
    let mut hanging_cdr = false;

    loop {
        match &*next_arg {
            SafasCell::List(car, cdr) => {
                // Evaluate car and push it onto the stack
                let (next_action, next_bindings) = bind_statement(Arc::clone(car), bindings)?;

                if next_action.reference_type() == ReferenceType::Monad {
                    // Convert to a monad
                    return bind_monad(bound, next_action, Arc::clone(cdr), next_bindings);
                }

                bound.push(next_action);

                bindings    = next_bindings;

                // cdr contains the next argument
                next_arg    = Arc::clone(cdr);
            }

            SafasCell::Nil => {
                // Got a complete list
                break;
            }

            _other => {
                // Incomplete list: evaluate the CDR value
                let (next_action, next_bindings) = bind_statement(next_arg, bindings)?;
                bound.push(next_action);

                bindings    = next_bindings;
                hanging_cdr = true;
                break;
            }
        }
    }

    // If there was a 'hanging' CDR, then generate a result with the same format, otherwise generate a well-formed list
    if hanging_cdr {
        let cdr = bound.pop();
        Ok((SafasCell::list_with_cells_and_cdr(bound, cdr.unwrap()).into(), bindings))
    } else {
        Ok((SafasCell::list_with_cells(bound).into(), bindings))
    }
}

///
/// Given a partially bound function with a monad parameter, rewrites it as a flat_map binding
/// 
/// Say we are evaluating the call (foo x) where 'x' is a monad. This will map this to (flat_map (fun (x) (foo x)) x),
/// returning a new monad as the result of the call. (This is equivalent to 'do' syntax in languages like Haskell but
/// taking account of SAFAS's use of dynamic types instead of static ones)
/// 
/// That is, we can use the monad's `flat_map` function to get the value wrapped in it, so we rewrite the function call
/// such that it occurs inside the `flat_map` call. The function is assumed not to return a monad itself in this case,
/// so the return value will be wrapped into a nil monad (which will get processed by the parent flat_map function)
///
fn bind_monad(args_so_far: Vec<CellRef>, monad: CellRef, remaining_args: CellRef, bindings: SymbolBindings) -> BindResult<CellRef> {
    // The remainder of the function will need to be evaluated in a function
    let mut interior_frame  = bindings.push_new_frame();

    // The first parameter of the flat_map function is the value of the monad argument
    let monad_value_cell    = interior_frame.alloc_cell();

    // Next parameters are bound from the closure and are the arguments so far (including the function, if present)
    let other_arguments     = args_so_far.iter().map(|arg| (interior_frame.alloc_cell(), arg.reference_type())).collect::<Vec<_>>();

    // Generate a partially-bound statement using these arguments (remaining_args are still unbound and go on the end)
    let monad_fn            = SafasCell::List(SafasCell::FrameReference(monad_value_cell, 0, ReferenceType::Value).into(), remaining_args);
    let mut monad_fn        = Arc::new(monad_fn);
    for (cell_id, reference_type) in other_arguments.iter().rev() {
        monad_fn = SafasCell::List(SafasCell::FrameReference(*cell_id, 0, *reference_type).into(), monad_fn).into();
    }

    // Bind this function
    // (Note: the args_so_far are all frame references here so they should bind to themselves, saving us some issues with rebinding)
    let bound_monad_fn                      = bind_statement(monad_fn, interior_frame);
    let (bound_monad_fn, interior_frame)    = match bound_monad_fn { Ok(fun) => fun, Err((err, interior_frame)) => return Err((err, interior_frame.pop().0)) };

    // If the result is not bound to a monad expression, we need to wrap the result
    let is_inner_expression                 = bound_monad_fn.reference_type() != ReferenceType::Monad;

    let bound_monad_fn                      = if is_inner_expression {
        // The inner result of a monadic expression needs to be wrapped so the return value is itself a monad (note we don't bind the bound monad fn itself here)
        let bound_monad_fn                  = SafasCell::List(bound_monad_fn, NIL.clone()).into();
        let bound_monad_fn                  = SafasCell::List(WRAP_KEYWORD.clone(), bound_monad_fn).into();
        bound_monad_fn
    } else {
        bound_monad_fn
    };

    // Compile to a closure (this generates the function passed to FlatMap later on)
    let monad_flat_map                      = compile_statement(bound_monad_fn);
    let monad_flat_map                      = match monad_flat_map { Ok(flat_map) => flat_map, Err(err) => return Err((err, interior_frame.pop().0)) };
    let interior_frame_size                 = interior_frame.num_cells;

    // Pop the interior frame and bring in any imports
    let (bindings, imports)                 = interior_frame.pop();

    // Add any imports to the list of arguments (all the arguments get imported into our closure)
    let mut other_arguments                 = other_arguments.into_iter().map(|(cell_id, _ref_type)| cell_id).collect::<Vec<_>>();
    let mut args_so_far                     = args_so_far;
    let mut bindings                        = bindings;

    for (symbol_value, import_into_cell_id) in imports.into_iter() {
        match &*symbol_value {
            SafasCell::FrameReference(_our_cell_id, 0, _type) => {
                // Cell from this frame
                other_arguments.push(import_into_cell_id);
                args_so_far.push(symbol_value);
            },

            SafasCell::FrameReference(their_cell_id, frame_count, their_type) => {
                // Import from a parent frame
                let our_cell_id = bindings.alloc_cell();
                bindings.import(SafasCell::FrameReference(*their_cell_id, *frame_count, *their_type).into(), our_cell_id);

                other_arguments.push(import_into_cell_id);
                args_so_far.push(SafasCell::FrameReference(our_cell_id, 0, *their_type).into());
            },

            _ => panic!("Don't know how to import this type of symbol")
        }
    }

    // Arguments are loaded in reverse order from the stack, so we need to reverse their order
    other_arguments.reverse();

    // Bind to a closure
    let monad_flat_map                      = monad_flat_map.to_actions().collect::<Vec<_>>();
    let monad_flat_map                      = StackClosure::new(monad_flat_map, other_arguments, interior_frame_size, 1);

    // Convert things to the final result
    let args_so_far                         = SafasCell::list_with_cells(args_so_far);
    let monad_flat_map                      = SafasCell::FrameMonad(Box::new(monad_flat_map)).into();

    // Result is a list starting with the monad
    let result                              = SafasCell::list_with_cells(vec![monad, args_so_far, monad_flat_map]).into();

    Ok((result, bindings))
}

///
/// Monad that performs binding on a statement
///
struct BindMonad {
    source: Vec<CellRef>
}

impl BindingMonad for BindMonad {
    type Binding=Vec<CellRef>;

    fn pre_bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Self::Binding) {
        (bindings, self.source.clone())
    }

    fn bind(&self, bindings: SymbolBindings) -> (SymbolBindings, Result<Self::Binding, BindError>) {
        let mut result      = vec![];
        let mut bindings    = bindings;

        for cell in self.source.iter() {
            match bind_statement(cell.clone(), bindings) {
                Ok((bound, new_bindings))   => { bindings = new_bindings; result.push(bound); }
                Err((err, bindings))        => return (bindings, Err(err))
            }
        }

        (bindings, Ok(result))
    }
}

///
/// Creates a binding monad that will bind the specified source
///
pub fn bind(source: CellRef) -> impl BindingMonad<Binding=CellRef> {
    BindMonad { source: vec![source] }.and_then(|mut results| wrap_binding(results.pop().unwrap()))
}

///
/// Creates a binding monad that will bind many items from the specified source
///
pub fn bind_all<Items: IntoIterator<Item=CellRef>>(source: Items) -> impl BindingMonad<Binding=Vec<CellRef>> {
    BindMonad { source: source.into_iter().collect() }
}
