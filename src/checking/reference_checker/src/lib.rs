/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use std::collections::{hash_map::Entry, HashMap, HashSet};

use java_class::{
    classinfo::ClassInfo,
    java_class::{Class, ConstPoolEntry},
};
use log::{debug, trace};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

struct ClassRequirements<'a> {
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
        let mut class_imports = vec![];
        let mut required_methods = vec![];
        for cp_info in &self.const_pool {
            if let (_, ConstPoolEntry::Class { name_index }) = cp_info {
                class_imports.push(
                    //remove array stuff around class definition
                    self.get_utf8(name_index)?
                        .trim_start_matches('[')
                        .trim_start_matches('L')
                        .trim_end_matches(';'),
                );
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
                debug!("Processing class {}", class_name);
                for method_signature in collect_methods(class_name, classes, java_classes)? {
                    result.insert(method_signature);
                }
                let result = (class_name, result);
                return Ok(result);
            }
            debug!("Skipping module-info.class");
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

pub fn check_classes(
    classes: &HashMap<String, Class>,
    parallel: bool,
    java_classes: &HashMap<&str, ClassInfo>,
) -> Option<HashSet<String>> {
    let provided = get_provided(classes, parallel, java_classes);
    let (mut required_classes, mut required_methods) = get_consumed(classes, parallel);
    for (class, methods) in provided {
        required_classes.remove(class);
        let Some(required_methods) = required_methods.get_mut(class) else {
            continue;
        };
        methods.iter().for_each(|s| {
            required_methods.remove(s);
        });
    }
    let mut result = HashSet::new();
    for class in required_classes {
        if java_classes.contains_key(class) {
            continue;
        }
        result.insert(class.to_owned());
    }
    for (class, method) in required_methods {
        method.iter().for_each(|m| {
            if !class.starts_with("java") {
                result.insert(format!("{}#{}", class, m));
            }
        });
    }
    Some(result)
}

fn get_consumed(
    classes: &HashMap<String, Class>,
    parallel: bool,
) -> (HashSet<&str>, HashMap<&str, HashSet<String>>) {
    if parallel {
        classes
            .par_iter()
            .map(|(_, class)| class.get_consumed().unwrap())
            .fold(
                || (HashSet::new(), HashMap::new()),
                |mut a, b| {
                    a.0.extend(b.classes);
                    for (c, m) in b.methods {
                        match a.1.entry(c) {
                            Entry::Vacant(e) => {
                                let mut set = HashSet::new();
                                set.insert(m);
                                e.insert(set);
                            }
                            Entry::Occupied(mut e) => {
                                e.get_mut().insert(m);
                            }
                        };
                    }
                    a
                },
            )
            .reduce(
                || (HashSet::new(), HashMap::new()),
                |mut a, b| {
                    a.0.extend(b.0);
                    a.1.extend(b.1);
                    a
                },
            )
    } else {
        classes
            .iter()
            .map(|(_, class)| class.get_consumed().unwrap())
            .fold((HashSet::new(), HashMap::new()), |mut a, b| {
                a.0.extend(b.classes);
                for (c, m) in b.methods {
                    match a.1.entry(c) {
                        Entry::Vacant(e) => {
                            let mut set = HashSet::new();
                            set.insert(m);
                            e.insert(set);
                        }
                        Entry::Occupied(mut e) => {
                            e.get_mut().insert(m);
                        }
                    };
                }
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
