#[macro_use] extern crate lazy_static;

mod meta;
mod bind;
mod exec;
mod parse;
mod syntax;
mod bitcode;
mod functions;
mod interactive;

use crate::bind::*;
use crate::exec::*;
use crate::interactive::*;

use clap::{App, Arg};

fn main() {
    // Fetch the parameters
    let params = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Copyright 2019 Andrew Hunter <andrew@logicalshift.io>")
        .about("The Self-Aware Functional Assembler")
        .after_help(concat!("Full source code is available at https://github.com/Logicalshift/safas\n",
            "\n",
            "Licensed under the Apache License, Version 2.0 (the \"License\");\n",
            "you may not use this file except in compliance with the License.\n",
            "You may obtain a copy of the License at\n",
            "\n",
            "http://www.apache.org/licenses/LICENSE-2.0\n\n"))
        .arg(Arg::with_name("interactive")
            .short("i")
            .long("interactive")
            .help("Launches the interactive interpreter"))
        .arg(Arg::with_name("INPUT")
            .help("Sets the input file to read from")
            .index(1))
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .help("Sets the location to write the output to")
            .value_name("OUTPUT"))
        .get_matches();

    // Create the initial execution frame and bindings
    let frame               = Frame::new(1, None);
    let bindings            = SymbolBindings::new();

    // Apply the standard bindings
    let (frame, bindings)   = setup_standard_bindings(frame, bindings);

    // Start in interactive mode if the -i parameter is passed in
    if params.occurrences_of("interactive") > 0 {
        run_interactive(frame, bindings);
        return;
    } else {
        // No valid parameters supplied
        println!("{}", params.usage());
    }
}
