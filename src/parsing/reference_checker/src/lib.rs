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

fn get_utf8<'a>(class: &'a Class, index: &u16) -> Option<&'a str> {
    if let ConstPoolEntry::Utf8 { value } = &class.const_pool[index] {
        Some(value.as_str())
    } else {
        println!("Not a UTF8 entry at idx {}!", index);
        None
    }
}

impl Consumer for Class {
    fn get_consumed(&self) -> HashSet<String> {
        let mut imports = HashSet::new();
        for cp_info in &self.const_pool {
            if let (_, ConstPoolEntry::Utf8 { value }) = cp_info {
                imports.extend(get_references(value));
            }
            if let (
                _,
                ConstPoolEntry::MethodRef {
                    class_index,
                    name_type_index,
                },
            ) = cp_info
            {
                let ConstPoolEntry::Class { name_index } = &self.const_pool[class_index] else {
                    println!("Not a class info entry at idx {}!", class_index);
                    continue;
                };
                let Some(class_name) = get_utf8(self, name_index) else {
                    continue;
                };
                let ConstPoolEntry::NameAndType {
                    name_index: method_name_index,
                    descriptor_index,
                } = &self.const_pool[name_type_index]
                else {
                    println!("Not a NameAndType entry at idx {}!", name_type_index);
                    continue;
                };
                let Some(method_name) = get_utf8(self, method_name_index) else {
                    continue;
                };
                let Some(method_descriptor) = get_utf8(self, descriptor_index) else {
                    continue;
                };
                imports.insert(format!(
                    "{}#{}{}",
                    class_name, method_name, method_descriptor,
                ));
            }
        }
        imports
    }
}

impl Provider for Class {
    fn get_provided(&self) -> HashSet<String> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            let Some(class_name) = get_utf8(self, &name_index) else {
                return result;
            };
            result.insert(class_name.to_owned());
            for method_info in &self.methods {
                let Some(method_name) = get_utf8(self, &method_info.name_index) else {
                    continue;
                };
                let Some(method_descriptor) = get_utf8(self, &method_info.descriptor_index) else {
                    continue;
                };
                result.insert(format!(
                    "{}#{}{}",
                    class_name, method_name, method_descriptor,
                ));
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
