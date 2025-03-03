use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use java_class::{Class, ConstPoolEntry};
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use zip::ZipArchive;

pub mod error;
pub mod java_class;

type Result<T> = std::result::Result<T, error::Error>;

fn read_zip_archive(path: &Path) -> Result<HashMap<String, Class>> {
    info!("Processing file {}...", path.to_str().unwrap());
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

pub fn parse_classpath(cp: &str, parallel: bool) -> Result<HashMap<String, Class>> {
    let split = cp.split(';');
    let expanded = split
        .map(|el| shellexpand::full(el).unwrap_or_else(|_| panic!("Failed to expand path {}", el)));
    let globbed = expanded
        .clone()
        .filter_map(|el| {
            if el.contains('*') {
                Some(glob::glob(el.as_ref()).unwrap().map(|p| p.unwrap()))
            } else {
                None
            }
        })
        .flatten();
    let concrete = expanded.filter_map(|el| {
        if !el.contains('*') {
            let mut p = PathBuf::new();
            p.push(el.as_ref());
            Some(p)
        } else {
            None
        }
    });
    let chained: Vec<PathBuf> = globbed.chain(concrete).collect();
    let result = if parallel {
        chained
            .par_iter()
            .map(|pb| read_zip_archive(pb.as_path()).unwrap())
            .reduce(HashMap::new, |a, mut b| {
                a.into_iter().for_each(|(k, v)| {
                    b.insert(k, v);
                });
                b
            })
    } else {
        chained
            .iter()
            .map(|pb| read_zip_archive(pb.as_path()).unwrap())
            .reduce(|a, mut b| {
                a.into_iter().for_each(|(k, v)| {
                    b.insert(k, v);
                });
                b
            })
            .unwrap_or(HashMap::new())
    };

    Ok(result)
}
