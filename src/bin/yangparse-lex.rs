//
// Binary for lexing a YANG file for testing purposes
//

use std::io::{stdout, Write};

use yangparse::lexing::{HumanReadableTokensExt, Scanner};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let filepath = args.get(1).expect("Expected file path as first argument");
    let buffer = std::fs::read(filepath).expect("Failed to read input file");

    // io::stdin().lock().read_to_end(&mut buffer).unwrap();

    let scanner = Scanner::new(&buffer);

    let mut lock = stdout().lock();

    for token in scanner.iter() {
        write!(lock, "{}", token.human_readable_string()).expect("Failed to write to STDOUT");
    }
}
