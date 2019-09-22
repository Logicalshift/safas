#[macro_use] extern crate lazy_static;

mod meta;
mod bind;
mod exec;
mod parse;
mod syntax;
mod functions;
mod interactive;

use self::interactive::*;

fn main() {
    run_interactive();
}
