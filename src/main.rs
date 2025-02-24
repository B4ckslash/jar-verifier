use binrw::BinReaderExt;
use reference_checker::{Consumer, Provider};
use std::{env, fs::File};

use java_class::java_class::Class;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut classfile = File::open(&args[1]).expect("Failed to open file!");
    let parsed: Class = classfile.read_be().unwrap();
    let parsed_requires = parsed.get_consumed();
    let parsed_provides = parsed.get_provided();
    println!("Consumes: {:?}", parsed_requires);
    println!("Provides: {:?}", parsed_provides);
}
