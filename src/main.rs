use std::env;

use reference_checker::{check_classes, parse_classpath};

fn main() -> Result<(), reference_checker::error::CheckError> {
    let args: Vec<String> = env::args().collect();
    let classes = parse_classpath(&args[1])?;
    let consumed = check_classes(&classes)?;
    println!(
        "Classpath: {} \n Class count {} \n Consumes: {:?}",
        &args[1],
        classes.len(),
        consumed
    );
    Ok(())
}
