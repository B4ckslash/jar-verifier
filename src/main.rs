use std::env;

mod error;
use java_class::parse_classpath;
use reference_checker::check_classes;

fn main() -> Result<(), error::Error> {
    let args: Vec<String> = env::args().collect();
    let classes = parse_classpath(&args[1])?;
    let consumed = check_classes(&classes).unwrap();
    println!(
        "Classpath: {} \n Class count {} \n Consumes: {:?}",
        &args[1],
        classes.len(),
        consumed
    );
    Ok(())
}
