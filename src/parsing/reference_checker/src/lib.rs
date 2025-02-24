use std::{
    collections::HashSet,
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

use java_class::java_class::{Class, ConstPoolEntry};
use once_cell::sync::Lazy;
use regex::Regex;
use zip::ZipArchive;

trait Consumer {
    fn get_consumed(&self) -> HashSet<String>;
}

trait Provider {
    fn get_provided(&self) -> HashSet<String>;
}

fn get_references(candidate: &str) -> HashSet<String> {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^((?:[[:word:]\$]+/)+[[:word:]\$]+)").expect("Invalid Regex!"));
    let mut result = HashSet::new();
    if let Some(caps) = RE.captures(candidate) {
        for cap in caps.iter().flatten() {
            let cap = match cap.as_str().strip_prefix('L') {
                Some(trimmed) => trimmed,
                None => cap.as_str(),
            };
            result.insert(cap.to_owned());
        }
    }
    result
}

impl Consumer for Class {
    fn get_consumed(&self) -> HashSet<String> {
        let mut imports = HashSet::new();
        for cp_info in &self.const_pool {
            if let (_, ConstPoolEntry::Utf8 { value }) = cp_info {
                imports.extend(get_references(value));
            }
        }
        imports
    }
}

impl Provider for Class {
    fn get_provided(&self) -> HashSet<String> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            if let ConstPoolEntry::Utf8 { value } = &self.const_pool[&name_index] {
                result.insert(value.to_owned());
            }
        }
        result
    }
}

pub fn read_zip_archive(path: &Path) -> (HashSet<String>, HashSet<String>) {
    let file = File::open(path).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    let mut provides = HashSet::new();
    let mut requires = HashSet::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if let Some(path) = file.enclosed_name() {
            if let Some(ext) = path.extension() {
                if ext.to_str().unwrap() == "class" {
                    let mut file_inmem: Vec<u8> = vec![];
                    if file.read_to_end(&mut file_inmem).is_err() {
                        continue;
                    }
                    let class_parsed = Class::from(&mut Cursor::new(file_inmem));
                    provides.extend(class_parsed.get_provided());
                    requires.extend(class_parsed.get_consumed());
                }
            }
        }
    }
    (provides, requires)
}
