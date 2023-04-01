//
// Binary for lexing a YANG file for testing purposes
//

use std::io::{stdout, Write};

use yangparse::lexing::{scan, HumanReadableTokensExt};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let filepath = args.get(1).expect("Expected file path as first argument");
    let buffer = std::fs::read(filepath).expect("Failed to read input file");

    let mut lock = stdout().lock();

    for token in scan(&buffer) {
        write!(lock, "{}", token.human_readable_string()).expect("Failed to write to STDOUT");
    }
}
