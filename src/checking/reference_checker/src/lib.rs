/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    hash::Hash,
};

use java_class::{
    classinfo::ClassInfo,
    java_class::{Class, ConstPoolEntry},
};
use log::{debug, trace};
use rayon::prelude::*;

struct ClassRequirements<'a> {
    name: &'a str,
    classes: Vec<&'a str>,
    methods: Vec<(&'a str, String)>,
}

trait Consumer<'a> {
    fn get_consumed(&'a self) -> Result<ClassRequirements<'a>, String>;
}

trait Provider {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
        java_classes: &HashMap<&str, ClassInfo>,
    ) -> Result<(&str, HashSet<String>), String>;
}

impl<'a> Consumer<'a> for Class {
    fn get_consumed(&'a self) -> Result<ClassRequirements<'a>, String> {
        static PRIMITIVES: [&str; 8] = ["B", "C", "D", "F", "I", "J", "S", "Z"];

        let mut class_imports = vec![];
        let mut required_methods = vec![];
        let this_name = self.get_name()?;
        for cp_info in &self.const_pool {
            if let (_, ConstPoolEntry::Class { name_index }) = cp_info {
                //remove array stuff around class definition
                let trimmed = self
                    .get_utf8(name_index)?
                    .trim_start_matches('[')
                    .trim_start_matches('L')
                    .trim_end_matches(';');
                if !PRIMITIVES.contains(&trimmed) {
                    class_imports.push(trimmed);
                }
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
                    return Err(format!("Not a class info entry at idx {}!", class_index));
                };
                let class_name = self.get_utf8(name_index)?;
                let ConstPoolEntry::NameAndType {
                    name_index: method_name_index,
                    descriptor_index,
                } = &self.const_pool[name_type_index]
                else {
                    println!("Not a NameAndType entry at idx {}!", name_type_index);
                    continue;
                };
                let method_name = self.get_utf8(method_name_index)?;
                let method_descriptor = self.get_utf8(descriptor_index)?;
                if method_name == "clone" && method_descriptor == "()Ljava/lang/Object;" {
                    continue;
                }
                required_methods
                    .push((class_name, format!("{}{}", method_name, method_descriptor)));
            }
        }
        Ok(ClassRequirements {
            name: this_name,
            classes: class_imports,
            methods: required_methods,
        })
    }
}

impl Provider for Class {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
        java_classes: &HashMap<&str, ClassInfo>,
    ) -> Result<(&str, HashSet<String>), String> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            let class_name = self.get_utf8(&name_index)?;
            if class_name != "module-info" {
                trace!("Processing class {}", class_name);
                for method_signature in collect_methods(class_name, classes, java_classes)? {
                    result.insert(method_signature);
                }
                let result = (class_name, result);
                return Ok(result);
            }
            trace!("Skipping module-info.class");
            return Ok(("module-info", HashSet::new()));
        }
        Err("This-class index is invalid!".to_owned())
    }
}

fn collect_methods(
    class_name: &str,
    classes: &HashMap<String, Class>,
    java_classes: &HashMap<&str, ClassInfo>,
) -> Result<HashSet<String>, String> {
    let mut result = HashSet::new();
    if let Some(current_class) = classes.get(class_name) {
        trace!("Class {}", class_name);
        for method_signature in current_class.get_methods()? {
            result.insert(method_signature);
        }
        if let ConstPoolEntry::Class { name_index } =
            current_class.const_pool[&current_class.super_class_idx]
        {
            let super_class_name = current_class.get_utf8(&name_index)?;
            result.extend(collect_methods(super_class_name, classes, java_classes)?);
            for iface_index in &current_class.iface_indexes {
                let ConstPoolEntry::Class { name_index } = current_class.const_pool[iface_index]
                else {
                    continue;
                };
                result.extend(collect_methods(
                    current_class.get_utf8(&name_index)?,
                    classes,
                    java_classes,
                )?);
            }
        }
    } else if let Some(super_class) = java_classes.get(class_name) {
        trace!("Java Class {}", class_name);
        trace!("Java Class Methods: {:?}", super_class.methods);
        result.extend(super_class.methods.iter().map(|&s| s.to_owned()));
        if let Some(super_class) = super_class.super_class {
            result.extend(collect_methods(super_class, classes, java_classes)?);
        }
    }
    Ok(result)
}

#[derive(Debug, Eq, Clone)]
pub struct ClassDependencies<'a> {
    name: &'a str,
    classes: HashSet<&'a str>,
    methods: HashMap<&'a str, HashSet<String>>,
}

impl PartialOrd for ClassDependencies<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        self.name.lt(other.name)
    }

    fn le(&self, other: &Self) -> bool {
        self.name.le(other.name)
    }

    fn gt(&self, other: &Self) -> bool {
        self.name.gt(other.name)
    }

    fn ge(&self, other: &Self) -> bool {
        self.name.ge(other.name)
    }
}

impl Ord for ClassDependencies<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(other.name)
    }
}

impl Hash for ClassDependencies<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.name.as_bytes());
    }
}

impl PartialEq for ClassDependencies<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<'a> From<ClassRequirements<'a>> for ClassDependencies<'a> {
    fn from(val: ClassRequirements<'a>) -> Self {
        let mut methods: HashMap<&'a str, HashSet<String>> = HashMap::new();
        let mut classes = HashSet::new();
        classes.extend(val.classes);
        for (class, method) in val.methods {
            match methods.entry(class) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().insert(method);
                }
                Entry::Vacant(entry) => {
                    let mut set = HashSet::new();
                    set.insert(method);
                    entry.insert(set);
                }
            };
        }
        ClassDependencies {
            name: val.name,
            classes,
            methods,
        }
    }
}

impl<'a> ClassDependencies<'a> {
    fn remove_class(&mut self, class: &str) {
        trace!("Removing class {} from {}", class, self.name);
        self.classes.remove(class);
    }

    fn remove_methods<'b>(&mut self, class: &'a str, methods: &'b HashSet<String>)
    where
        'a: 'b,
    {
        trace!(
            "Removing methods {:?} of class {} from {}",
            methods, class, self.name
        );
        if let Entry::Occupied(mut e) = self.methods.entry(class) {
            let value = e.get_mut();
            for method in methods {
                trace!("Trying to remove {}#{}", class, method.as_str());
                if value.remove(method) {
                    trace!("Removed {}#{}", class, method.as_str());
                    trace!("Remaining methods for {}: {:?}", class, value);
                }
            }
        }
        self.methods.retain(|_, set| !set.is_empty());
    }

    fn remove_java_classes_and_methods(&mut self, java_classes: &HashMap<&str, ClassInfo>) {
        self.classes.retain(|name| !java_classes.contains_key(name));
        self.methods
            .retain(|&class, _| !java_classes.contains_key(class));
    }

    fn is_empty(&self) -> bool {
        self.classes.is_empty() && self.methods.is_empty()
    }

    pub fn format(&self) -> String {
        let mut result = self.name.to_owned();
        result.push('\n');
        let mut sorted: Vec<&&str> = self.classes.iter().collect();
        sorted.sort();
        for class in sorted {
            result.push('\t');
            result.push_str(format!("Class {}", class).as_str());
            result.push('\n');
        }
        let mut sorted: Vec<(&&str, &HashSet<String>)> = self.methods.iter().collect();
        sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
        for (class, methods) in sorted {
            let mut sorted: Vec<&String> = methods.iter().collect();
            sorted.sort();
            for method in sorted {
                result.push('\t');
                result.push_str(format!("Method {}#{}", class, method).as_str());
                result.push('\n');
            }
        }
        result
    }
}

pub fn check_classes<'a>(
    classes: &'a HashMap<String, Class>,
    parallel: bool,
    java_classes: &HashMap<&str, ClassInfo>,
) -> Option<HashSet<ClassDependencies<'a>>> {
    let provided = get_provided(classes, parallel, java_classes);
    let mut dependencies: Vec<ClassDependencies<'a>> = Vec::new();
    dependencies.extend(get_consumed(classes, parallel));
    let mut dependencies: Vec<ClassDependencies<'a>> = dependencies
        .iter_mut()
        .map(|d| {
            d.remove_java_classes_and_methods(java_classes);
            d.to_owned()
        })
        .collect();
    debug!(
        "Provided size {} | Dependencies count {}",
        provided.capacity(),
        dependencies.capacity()
    );
    if parallel {
        dependencies.par_iter_mut().for_each(|dep| {
            for (class, methods) in &provided {
                if dep.classes.contains(class) {
                    dep.remove_class(class);
                    dep.remove_methods(class, methods);
                }
            }
        });
    } else {
        for dep in dependencies.iter_mut() {
            for (class, methods) in &provided {
                if dep.classes.contains(class) {
                    dep.remove_class(class);
                    dep.remove_methods(class, methods);
                }
            }
        }
    }
    dependencies.retain(|dep| !dep.is_empty());
    let mut result = HashSet::new();
    result.extend(dependencies);
    Some(result)
}

fn get_consumed(classes: &HashMap<String, Class>, parallel: bool) -> HashSet<ClassDependencies> {
    if parallel {
        classes
            .par_iter()
            .map(|(_, class)| Into::<ClassDependencies<'_>>::into(class.get_consumed().unwrap()))
            .fold(HashSet::new, |mut a, b| {
                a.insert(b);
                a
            })
            .reduce(HashSet::new, |mut a, b| {
                a.extend(b);
                a
            })
    } else {
        classes
            .iter()
            .map(|(_, class)| Into::<ClassDependencies<'_>>::into(class.get_consumed().unwrap()))
            .fold(HashSet::new(), |mut a, b| {
                a.insert(b);
                a
            })
    }
}

fn get_provided<'a>(
    classes: &'a HashMap<String, Class>,
    parallel: bool,
    java_classes: &HashMap<&str, ClassInfo>,
) -> HashMap<&'a str, HashSet<String>> {
    if parallel {
        classes
            .par_iter()
            .map(|(_, class)| class.get_provided(classes, java_classes).unwrap())
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
            .map(|(_, class)| class.get_provided(classes, java_classes).unwrap())
            .fold(HashMap::new(), |mut a, b| {
                a.insert(b.0, b.1);
                a
            })
    }
}
