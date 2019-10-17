use crate::meta::*;
use crate::parse::*;
use crate::bind::*;
use crate::exec::*;
use crate::syntax::*;
use crate::functions::*;

use std::io;
use std::io::{Write};
use std::sync::*;

///
/// Evaluates a single line in an isolated SAFAS instance and returns the result
///
pub fn eval(expr: &str) -> Result<CellRef, RuntimeError> {
    // Create the execution frame
    let mut frame               = Frame::new(1, None);
    let bindings                = SymbolBindings::new();

    // Apply the standard bindings
    let syntax                  = standard_syntax();
    let functions               = standard_functions();
    let (bindings, actions)     = syntax.bind(bindings);
    let (bindings, fn_actions)  = functions.bind(bindings);
    frame.allocate_for_bindings(&bindings);
    let (frame, _)              = actions.unwrap().execute(frame);
    let (frame, _)              = fn_actions.unwrap().execute(frame);

    let mut frame               = frame;
    let mut bindings            = bindings;

    // Parse the expression
    let expr = parse_safas(&mut TokenReadBuffer::new(expr.chars()), FileLocation::new("<expr>"))?;

    // Pre-bind the statements
    let mut statement   = Arc::clone(&expr);
    while let SafasCell::List(car, cdr) = &*statement {
        let (new_bindings, _)   = pre_bind_statement(Arc::clone(&car), bindings);
        bindings                = new_bindings;
        statement               = Arc::clone(&cdr);
    }

    // Run the statements in the current frame
    let mut statement   = Arc::clone(&expr);
    let mut result      = NIL.clone();
    while let SafasCell::List(car, cdr) = &*statement {
        // Bind this statement
        let (bound, new_bindings)   = match bind_statement(Arc::clone(&car), bindings) { Ok((bound, new_bindings)) => (bound, new_bindings), Err((err, _new_bindings)) => return Err(err.into()) };
        let actions                 = compile_statement(bound)?;
        let monad                   = actions.to_actions().collect::<Vec<_>>();
        bindings                    = new_bindings;

        // Evaluate the monad
        frame.allocate_for_bindings(&bindings);
        let expr_result = monad.execute(frame);
        match expr_result {
            (new_frame, Ok(expr_result))    => { frame = new_frame; result = expr_result; }
            (_new_frame, Err(error))        => { return Err(error); }
        }

        // Move on to the next statement
        statement = Arc::clone(&cdr);
    }

    Ok(result)
}

///
/// Runs the parser and interpreter in interactive mode, displaying the results to the user
///
pub fn run_interactive() {
    println!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("Interactive interpreter");

    // Create the execution frame
    let mut frame               = Frame::new(1, None);
    let bindings                = SymbolBindings::new();

    // Apply the standard bindings
    let syntax                  = standard_syntax();
    let functions               = standard_functions();
    let (bindings, actions)     = syntax.bind(bindings);
    let (bindings, fn_actions)  = functions.bind(bindings);
    frame.allocate_for_bindings(&bindings);
    let (frame, _)              = actions.unwrap().execute(frame);
    let (frame, _)              = fn_actions.unwrap().execute(frame);

    let mut frame               = frame;
    let mut bindings            = bindings;

    loop {
        // Read a line
        let mut input = String::new();
        println!();
        print!("-> ");
        io::stdout().flush().unwrap();
        let num_bytes = io::stdin().read_line(&mut input).unwrap();

        // EOF if num_bytes = 0
        if num_bytes == 0 {
            break;
        }

        // Parse the input
        let input = parse_safas(&mut TokenReadBuffer::new(input.chars()), FileLocation::new("<stdin>"));
        let input = match input {
            Ok(input)   => input,
            Err(error)  => {
                println!("!! Parse error: {:?}", error);
                continue;
            }
        };

        // Pre-bind the statements
        let mut statement   = Arc::clone(&input);
        while let SafasCell::List(car, cdr) = &*statement {
            let (new_bindings, _)   = pre_bind_statement(Arc::clone(&car), bindings);
            bindings                = new_bindings;
            statement               = Arc::clone(&cdr);
        }

        // Run the statements in the current frame
        let mut statement = Arc::clone(&input);
        while let SafasCell::List(car, cdr) = &*statement {
            // Bind this statement
            let bind_result = bind_statement(Arc::clone(&car), bindings)
                .and_then(|(bound, new_bindings)| match compile_statement(bound) {
                    Ok(actions) => Ok((actions, new_bindings)),
                    Err(err)    => Err((err, new_bindings))
                });
            let monad       = match bind_result {
                Ok((actions, new_bindings))   => { bindings = new_bindings; actions.to_actions().collect::<Vec<_>>() },
                Err((error, new_bindings))    => { 
                    bindings = new_bindings;
                    println!("!! Binding error: {:?}", error);
                    break;
                }
            };

            // Evaluate the monad
            frame.allocate_for_bindings(&bindings);
            let result      = monad.execute(frame);
            match result {
                (new_frame, Ok(result)) => { frame = new_frame; println!("{}", result.to_string()); }
                (new_frame, Err(error)) => { frame = new_frame; println!("!! Error: {:?}", error); }    
            }

            // Move on to the next statement
            statement = Arc::clone(&cdr);
        }
    }
}