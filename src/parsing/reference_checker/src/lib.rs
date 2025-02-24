use std::collections::HashSet;

use java_class::java_class::{Class, ConstPoolEntry};
use regex::Regex;

pub trait Consumer {
    fn get_consumed(&self) -> HashSet<&str>;
}

pub trait Provider {
    fn get_provided(&self) -> HashSet<&str>;
}

fn get_references(candidate: &str) -> HashSet<&str> {
    let re = Regex::new(r"^((?:[[:alnum:]\$]+/)+[[:alnum:]\$]+)").expect("Invalid Regex!");
    let mut result = HashSet::new();
    if let Some(caps) = re.captures(candidate) {
        for cap in caps.iter().flatten() {
            let cap = match cap.as_str().strip_prefix('L') {
                Some(trimmed) => trimmed,
                None => cap.as_str(),
            };
            result.insert(cap);
        }
    }
    result
}

impl Consumer for Class {
    fn get_consumed(&self) -> HashSet<&str> {
        let mut imports = HashSet::new();
        for cp_info in &self.const_pool {
            if let (_, ConstPoolEntry::Utf8 { value }) = cp_info {
                imports.extend(get_references(value).iter());
            }
        }
        imports
    }
}

impl Provider for Class {
    fn get_provided(&self) -> HashSet<&str> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            if let ConstPoolEntry::Utf8 { value } = &self.const_pool[&name_index] {
                result.insert(value.as_str());
            }
        }
        result
    }
}
