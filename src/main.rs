mod args;
mod error;
use std::{collections::HashMap, fs::File, io::BufRead, path::Path};

use args::Args;
use clap::Parser;
use env_logger::Env;
use java_class::parse_classpath;
use log::trace;
use reference_checker::check_classes;

fn main() -> Result<(), error::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let java_classes = read_classinfo(&args.jdk_classinfo);
    let classes = parse_classpath(&args.classpath, args.parallel)?;
    let consumed =
        check_classes(&classes, args.parallel, &java_classes?).expect("Failed to get result");
    println!(
        "Classpath: {} \n Class count {} \n Consume count: {:?}",
        &args.classpath,
        classes.len(),
        consumed.len()
    );
    trace!("{:?}", consumed);
    Ok(())
}

fn read_classinfo(path: &str) -> Result<HashMap<String, Vec<String>>, error::Error> {
    let path = Path::new(path);
    let reader = std::io::BufReader::new(File::open(path)?);
    let mut result = HashMap::new();
    for line in reader.lines() {
        let line = line?;
        if !line.starts_with("--") {
            trace!("Java Class {} from classinfo", line);
            result.insert(line, vec![]);
        } else {
            trace!("Java Class Method {} from classinfo", line);
            result.entry(line.clone()).and_modify(|vec| {
                vec.push(
                    line.strip_prefix("--")
                        .expect("Method lines should be prefixed with '--'!")
                        .to_owned(),
                )
            });
        }
    }
    Ok(result)
}
