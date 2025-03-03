use std::env;

mod error;
use env_logger::Env;
use java_class::parse_classpath;
use reference_checker::check_classes;

fn main() -> Result<(), error::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args: Vec<String> = env::args().collect();
    let classes = parse_classpath(&args[1], true)?;
    let consumed = check_classes(&classes, true).unwrap();
    println!(
        "Classpath: {} \n Class count {} \n Consume count: {:?}",
        &args[1],
        classes.len(),
        consumed.len()
    );
    Ok(())
}
