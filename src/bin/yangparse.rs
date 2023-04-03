use yangparse::parsing::parse;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let filepath = args.get(1).expect("Expected file path as first argument");
    let buffer = std::fs::read(filepath).expect("Failed to read input file");

    let tree = parse(&buffer).expect("Failed to parse input");

    println!("{}", tree);
}
