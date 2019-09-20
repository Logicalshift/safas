use crate::meta::*;
use crate::parse::*;
use crate::bind::*;
use crate::exec::*;

use std::io;
use std::io::{Write};
use std::sync::*;

///
/// Runs the parser and interpreter in interactive mode, displaying the results to the user
///
pub fn run_interactive() {
    println!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!("Interactive interpreter");

    // Create the execution frame
    let mut frame       = Frame::new(1, None);
    let mut bindings    = SymbolBindings::new();

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

        // Run the statements in the current frame
        let mut statement = Arc::clone(&input);
        while let SafasCell::List(car, cdr) = &*statement {
            // Bind this statement
            let bind_result = bind_statement(Arc::clone(&car), bindings);
            let monad       = match bind_result {
                Ok((actions, new_bindings))   => { bindings = new_bindings; actions.into_iter().collect::<Vec<_>>() },
                Err(error)                  => { 
                    bindings = SymbolBindings::new();
                    println!("!! Binding error: {:?}", error);
                    break;
                }
            };

            // Evaluate the monad
            let result      = monad.resolve(frame);
            match result {
                Ok((new_frame, result)) => { frame = new_frame; println!("{}", result.to_string()); }
                Err(error)              => { frame = Frame::new(1, None); println!("!! Error: {:?}", error); }    
            }

            // Move on to the next statement
            statement = Arc::clone(&cdr);
        }
    }
}