use std::collections::{HashMap, HashSet};

use java_class::java_class::{Class, ConstPoolEntry};
use log::{debug, trace};
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;

trait Consumer {
    fn get_consumed(&self, classes: &HashMap<String, Class>) -> Result<HashSet<String>, String>;
}

trait Provider {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
    ) -> Result<(&str, HashSet<String>), String>;
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
    fn get_consumed(&self, classes: &HashMap<String, Class>) -> Result<HashSet<String>, String> {
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
        Ok(imports)
    }
}

impl Provider for Class {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
    ) -> Result<(&str, HashSet<String>), String> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            let class_name = self
                .get_utf8(&name_index)
                .ok_or("Class name index is invalid!".to_owned())?;
            if class_name != "module-info" {
                debug!("Processing class {}", class_name);
                for method_signature in collect_methods(class_name, classes)? {
                    result.insert(method_signature);
                }
                let result = (class_name, result);
                trace!("{:?}", result);
                return Ok(result);
            }
            debug!("Skipping module-info.class");
            return Ok(("module-info", HashSet::new()));
        }
        Err("This-class index is invalid!".to_owned())
    }
}

fn collect_methods(
    super_class_name: &str,
    classes: &HashMap<String, Class>,
) -> Result<HashSet<String>, String> {
    let mut result = HashSet::new();
    if let Some(super_class) = classes.get(super_class_name) {
        trace!("Super class: {}", super_class_name);
        for method_signature in super_class.get_methods()? {
            result.insert(method_signature);
        }
        if let ConstPoolEntry::Class { name_index } =
            super_class.const_pool[&super_class.super_class_idx]
        {
            let super_class_name = super_class
                .get_utf8(&name_index)
                .ok_or("Class name index is invalid!".to_owned())?;
            result.extend(collect_methods(super_class_name, classes)?)
        }
    }
    Ok(result)
}

pub fn check_classes(classes: &HashMap<String, Class>, parallel: bool) -> Option<HashSet<String>> {
    Some(
        get_provided(classes, parallel)
            .into_keys()
            .map(|s| s.to_owned())
            .collect(),
    )
}

fn get_provided(
    classes: &HashMap<String, Class>,
    parallel: bool,
) -> HashMap<&str, HashSet<String>> {
    if parallel {
        classes
            .par_iter()
            .map(|(_, class)| class.get_provided(classes).unwrap())
            .fold(HashMap::new, |mut a, b| {
                a.insert(b.0, b.1);
                a
            })
            .reduce(HashMap::new, |mut a, b| {
                b.into_iter().for_each(|(k, v)| {
                    a.insert(k, v);
                });
                a
            })
    } else {
        classes
            .iter()
            .map(|(_, class)| class.get_provided(classes).unwrap())
            .fold(HashMap::new(), |mut a, b| {
                a.insert(b.0, b.1);
                a
            })
    }
}
