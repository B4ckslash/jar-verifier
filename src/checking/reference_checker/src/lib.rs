use std::collections::{HashMap, HashSet};

use java_class::java_class::{Class, ConstPoolEntry};
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;

trait Consumer {
    fn get_consumed(&self, classes: &HashMap<String, Class>) -> HashSet<String>;
}

trait Provider {
    fn get_provided(&self, classes: &HashMap<String, Class>) -> HashSet<String>;
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
    fn get_consumed(&self, classes: &HashMap<String, Class>) -> HashSet<String> {
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
                let Some(class_name) = self.get_utf8(name_index) else {
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
                let Some(method_name) = self.get_utf8(method_name_index) else {
                    continue;
                };
                let Some(method_descriptor) = self.get_utf8(descriptor_index) else {
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
    fn get_provided(&self, classes: &HashMap<String, Class>) -> HashSet<String> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            let Some(class_name) = self.get_utf8(&name_index) else {
                return result;
            };
            result.insert(class_name.to_owned());
            for method_info in &self.methods {
                let Some(method_name) = self.get_utf8(&method_info.name_index) else {
                    continue;
                };
                let Some(method_descriptor) = self.get_utf8(&method_info.descriptor_index) else {
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

pub fn check_classes(classes: &HashMap<String, Class>) -> Option<HashSet<String>> {
    let result = classes
        .par_iter()
        .map(|(_, class)| (class.get_consumed(classes)))
        .reduce(HashSet::new, |existing, new| {
            existing.union(&new).cloned().collect()
        });
    Some(result)
}
