use crate::meta::*;
use crate::parse::*;
use crate::bind::*;
use crate::exec::*;
use crate::syntax::*;
use crate::functions::*;

use std::io;
use std::io::{Write};

///
/// Evaluates a single line in an isolated SAFAS instance and returns the result
///
pub fn eval(expr: &str) -> Result<CellRef, RuntimeError> {
    // Create the execution frame
    let frame                   = Frame::new(1, None);
    let bindings                = SymbolBindings::new();

    // Apply the standard bindings
    let (frame, bindings)       = setup_standard_bindings(frame, bindings);

    // Parse the expression
    let expr = parse_safas(&mut TokenReadBuffer::new(expr.chars()), FileLocation::new("<expr>"))?;

    // Evaluate the expression
    let (result, _frame, _bindings) = eval_statements(expr, NIL.clone(), frame, bindings);

    if let SafasCell::Error(err) = &*result {
        Err(err.clone())
    } else {
        Ok(result)
    }
}

///
/// Applies the standard function bindings to the specified frame and bindings (sets these up for interactive mode)
///
pub fn setup_standard_bindings(frame: Frame, bindings: SymbolBindings) -> (Frame, SymbolBindings) {
    // Apply the standard bindings
    let mut frame               = frame;
    let syntax                  = standard_syntax();
    let functions               = standard_functions();
    let (bindings, actions)     = syntax.bind(bindings);
    let (bindings, fn_actions)  = functions.bind(bindings);
    frame.allocate_for_bindings(&bindings);
    let (frame, _)              = actions.unwrap().execute(frame);
    let (frame, _)              = fn_actions.unwrap().execute(frame);

    (frame, bindings)
}

///
/// Runs the parser and interpreter in interactive mode, displaying the results to the user
///
pub fn run_interactive(frame: Frame, bindings: SymbolBindings) {
    println!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("Interactive interpreter");

    let mut frame               = frame;
    let mut bindings            = bindings;
    let mut monad_value         = NIL.clone();

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

        // Evaluate the frame
        let (result, next_frame, next_bindings) = eval_statements(input, monad_value, frame, bindings);

        // Display the next result
        match &*result {
            SafasCell::Error(err)   => println!("!! Error: {:?}", err),
            _                       => println!("{}", result.to_string())
        }

        // Update to the next result
        frame       = next_frame;
        bindings    = next_bindings;
        monad_value = result;
    }
}
