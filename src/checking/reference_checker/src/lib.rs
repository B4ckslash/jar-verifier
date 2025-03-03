use std::collections::{HashMap, HashSet};

use java_class::java_class::{Class, ConstPoolEntry};
use log::{debug, trace};
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use regex::Regex;

#[derive(Debug)]
struct ClassProvides<'a> {
    class_name: &'a str,
    methods: HashSet<String>,
}

impl<'a> From<ClassProvides<'a>> for HashSet<String> {
    fn from(value: ClassProvides<'a>) -> Self {
        let mut result = HashSet::with_capacity(value.methods.len() + 1);
        result.insert(value.class_name.to_owned());
        for method in value.methods {
            result.insert(format!("{}#{}", value.class_name, method));
        }
        result
    }
}

trait Consumer {
    fn get_consumed(&self, classes: &HashMap<String, Class>) -> Result<HashSet<String>, String>;
}

trait Provider {
    fn get_provided<'a>(
        &'a self,
        classes: &HashMap<String, Class>,
    ) -> Result<ClassProvides<'a>, String>;
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
    fn get_provided<'a>(
        &'a self,
        classes: &HashMap<String, Class>,
    ) -> Result<ClassProvides<'a>, String> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            let class_name = self
                .get_utf8(&name_index)
                .ok_or("Class name index is invalid!".to_owned())?;
            if class_name != "module-info" {
                debug!("Processing class {}", class_name);
                for method_signature in collect_methods(self)? {
                    result.insert(method_signature);
                }
                if let &ConstPoolEntry::Class { name_index } =
                    &self.const_pool[&self.super_class_idx]
                {
                    let super_class_name = self
                        .get_utf8(&name_index)
                        .ok_or("Class name index is invalid!".to_owned())?;
                    for super_method_signature in collect_super_methods(super_class_name, classes)?
                    {
                        result.insert(super_method_signature);
                    }
                }
                let result = ClassProvides {
                    class_name,
                    methods: result,
                };
                trace!("{:?}", result);
                return Ok(result);
            }
            debug!("Skipping module-info.class");
            return Ok(ClassProvides {
                class_name: "module-info",
                methods: HashSet::new(),
            });
        }
        Err("This-class index is invalid!".to_owned())
    }
}

fn collect_methods(class: &Class) -> Result<HashSet<String>, String> {
    let mut result = HashSet::new();
    for method_info in &class.methods {
        let method_name = class
            .get_utf8(&method_info.name_index)
            .ok_or("Method name index is invalid!".to_owned())?;
        let method_descriptor = class
            .get_utf8(&method_info.descriptor_index)
            .ok_or("Method descriptor index is invalid!".to_owned())?;
        result.insert(format!("{}{}", method_name, method_descriptor,));
    }
    Ok(result)
}

fn collect_super_methods(
    super_class_name: &str,
    classes: &HashMap<String, Class>,
) -> Result<HashSet<String>, String> {
    let mut result = HashSet::new();
    if let Some(super_class) = classes.get(super_class_name) {
        trace!("Super class: {}", super_class_name);
        for method_signature in collect_methods(super_class)? {
            result.insert(method_signature);
        }
        if let ConstPoolEntry::Class { name_index } =
            super_class.const_pool[&super_class.super_class_idx]
        {
            let super_class_name = super_class
                .get_utf8(&name_index)
                .ok_or("Class name index is invalid!".to_owned())?;
            result.extend(collect_super_methods(super_class_name, classes)?)
        }
    }
    Ok(result)
}

pub fn check_classes(classes: &HashMap<String, Class>, parallel: bool) -> Option<HashSet<String>> {
    if parallel {
        let result = classes
            .par_iter()
            .flat_map(|(_, class)| {
                HashSet::from(class.get_provided(classes).unwrap()).into_par_iter()
            })
            .fold(HashSet::new, |mut a, b| {
                a.insert(b);
                a
            })
            .reduce(HashSet::new, |mut a, b| {
                a.extend(b);
                a
            });
        return Some(result);
    }
    let result = classes
        .iter()
        .flat_map(|(_, class)| HashSet::from(class.get_provided(classes).unwrap()).into_iter())
        .fold(HashSet::new(), |mut a, b| {
            a.insert(b);
            a
        });
    Some(result)
}
