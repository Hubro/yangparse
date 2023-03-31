//
// Binary for lexing a YANG file for testing purposes
//

use std::io::{self, BufRead};

use yangparse::lexing::{scan, HumanReadableTokensExt};

fn main() {
    let stdin = io::stdin();
    let mut input_lines: Vec<String> = Vec::new();

    for line in stdin.lock().lines() {
        match line {
            Ok(line) => input_lines.push(line),
            Err(error) => print!("Oh boy, an error: {}", error),
        }
    }

    let input_string = format!("{}\n", input_lines.join("\n"));
    let tokens = scan(&input_string);

    match tokens {
        Err(error) => println!("Scan failed: {}", error),
        Ok(tokens) => {
            print!("{}", tokens.human_readable_string());
        }
    }
}
