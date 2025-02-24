use std::{env, path::Path};

use reference_checker::read_zip_archive;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = Path::new(&args[1]);
    let (provided, required) = read_zip_archive(path);
    println!(
        "File: {} \n Provided: {:?} \n Required: {:?} \n --------- \n Missing: {:?}",
        path.to_str().unwrap(),
        provided,
        required,
        required.difference(&provided)
    )
}
