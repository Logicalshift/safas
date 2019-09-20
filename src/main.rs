#[macro_use] extern crate lazy_static;
#[macro_use] extern crate smallvec;

mod meta;
mod bind;
mod exec;
mod parse;
mod interactive;

use self::interactive::*;

fn main() {
    run_interactive();
}
