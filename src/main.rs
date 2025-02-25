use std::env;

use reference_checker::parse_classpath;

fn main() {
    let args: Vec<String> = env::args().collect();
    let classes = parse_classpath(&args[1]);
    println!("Classpath: {} \n Class count {}", &args[1], classes.len())
}
