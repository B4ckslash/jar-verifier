mod args;
mod error;
use args::Args;
use clap::Parser;
use env_logger::Env;
use java_class::parse_classpath;
use reference_checker::check_classes;

fn main() -> Result<(), error::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let classes = parse_classpath(&args.classpath, args.parallel)?;
    let consumed = check_classes(&classes, args.parallel).unwrap();
    println!(
        "Classpath: {} \n Class count {} \n Consume count: {:?}",
        &args.classpath,
        classes.len(),
        consumed.len()
    );
    Ok(())
}
