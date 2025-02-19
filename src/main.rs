use binrw::BinReaderExt;
use std::{env, fs::File};

use java_class::java_class::Class;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut classfile = File::open(&args[1]).expect("Failed to open file!");
    let parsed: Class = classfile.read_be().unwrap();
    println!("{:?}", parsed)
}
