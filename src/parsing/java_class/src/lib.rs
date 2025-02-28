use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

use java_class::{Class, ConstPoolEntry};
use zip::ZipArchive;

pub mod error;
pub mod java_class;

type Result<T> = std::result::Result<T, error::Error>;

fn read_zip_archive(path: &Path) -> Result<HashMap<String, Class>> {
    println!("Processing file {}...", path.to_str().unwrap());
    let file = File::options()
        .read(true)
        .write(false)
        .create_new(false)
        .open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut classes = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if let Some(path) = file.enclosed_name() {
            if let Some(ext) = path.extension() {
                if ext.eq("class") {
                    let mut file_inmem: Vec<u8> = vec![];
                    if file.read_to_end(&mut file_inmem).is_err() {
                        continue;
                    }
                    let class_parsed = Class::from(&mut Cursor::new(file_inmem));
                    let ConstPoolEntry::Class { name_index } =
                        &class_parsed.const_pool[&class_parsed.this_class_idx]
                    else {
                        continue;
                    };
                    let Some(class_name) = class_parsed.get_utf8(name_index) else {
                        continue;
                    };
                    classes.insert(class_name.to_owned(), class_parsed);
                }
            }
        }
    }
    Ok(classes)
}

pub fn parse_classpath(cp: &str) -> Result<HashMap<String, Class>> {
    let mut result = HashMap::new();
    for element in cp.split(';') {
        let element = shellexpand::full(element)
            .unwrap_or_else(|_| panic!("Failed to expand path {}", element));
        if element.contains('*') {
            for entry in glob::glob(element.as_ref())? {
                let path = entry?;
                result.extend(read_zip_archive(path.as_path())?);
            }
        } else {
            result.extend(read_zip_archive(Path::new(element.as_ref()))?);
        }
    }
    Ok(result)
}
