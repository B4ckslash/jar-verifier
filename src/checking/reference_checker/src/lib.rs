/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use std::{collections::hash_map::Entry, hash::Hash};

use ahash::{AHashMap, AHashSet};
use java_class::{
    classinfo::{ClassInfo, Method},
    java_class::{Class, ConstPoolEntry},
};
use log::{debug, info, trace};
use rayon::prelude::*;

type HashMap<K, V> = AHashMap<K, V>;
type HashSet<E> = AHashSet<E>;

#[derive(Debug, Eq)]
pub struct ClassRequirements<'a> {
    name: &'a str,
    dependencies: HashMap<&'a str, Dependency>,
}

impl PartialOrd for ClassRequirements<'_> {
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

impl Ord for ClassRequirements<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(other.name)
    }
}

impl Hash for ClassRequirements<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.name.as_bytes());
    }
}

impl PartialEq for ClassRequirements<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<'a> ClassRequirements<'a> {
    #[allow(dead_code)]
    fn remove_class(&mut self, class: &str) {
        trace!("Removing class {} from {}", class, self.name);
        self.dependencies.remove(class);
    }

    fn clear_empty_deps(&mut self) {
        self.dependencies.retain(|_, dep| !dep.methods.is_empty());
    }

    fn remove_methods<'b>(
        &mut self,
        class: &'a str,
        methods: &'b HashMap<String, Method>,
    ) where
        'a: 'b,
    {
        trace!(
            "Removing methods {:?} of class {} from {}",
            methods, class, self.name
        );
        let entry = self.dependencies.entry(class);

        if let Entry::Occupied(mut e) = entry {
            let value = &mut e.get_mut().methods;
            for sig in methods.keys() {
                trace!("Trying to remove {}#{}", class, sig.as_str());
                if value.remove(sig) {
                    trace!("Removed {}#{}", class, sig.as_str());
                    trace!("Remaining methods for {}: {:?}", class, value);
                }
            }
        }
        self.clear_empty_deps();
    }

    fn remove_java_classes_and_methods(&mut self, java_classes: &HashMap<&str, ClassInfo>) {
        self.dependencies
            .retain(|name, _| !java_classes.contains_key(name));
        for (class_name, dep) in self.dependencies.iter_mut() {
            dep.methods
                .retain(|method| !Self::provides_method(class_name, method, java_classes));
        }
        self.clear_empty_deps();
    }

    fn provides_method(class: &str, method: &str, java_classes: &HashMap<&str, ClassInfo>) -> bool {
        if let Some(class_info) = java_classes.get("java/lang/Object")
            && !method.contains("<init>")
            && class_info.methods.contains_key(method)
        {
            return true;
        }
        if let Some(class_info) = java_classes.get(class) {
            if class_info.methods.contains_key(method) {
                return true;
            }
            if class_info.methods.values().any(|m| m.polymorphic_signature) {
                let Some((method_name, _)) = method.split_once("(") else {
                    panic!("Illegal method signature! {}", method)
                };
                for class_method in class_info.methods.values().filter_map(|m| {
                    if m.polymorphic_signature {
                        let Some((name, _)) = method.split_once("(") else {
                            panic!("Illegal method signature! {}", method)
                        };
                        return Some(name);
                    }
                    None
                }) {
                    if method_name == class_method {
                        return true;
                    }
                }
            }
            if let Some(super_class) = class_info.super_class
                && Self::provides_method(super_class, method, java_classes)
            {
                return true;
            }
            for super_iface in &class_info.interfaces {
                if Self::provides_method(super_iface, method, java_classes) {
                    return true;
                }
            }
        }
        false
    }

    fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }

    pub fn format(&'a self) -> String {
        let mut result = self.name.to_owned();
        result.push('\n');
        let mut sorted: Vec<(&'a str, &Dependency)> = self
            .dependencies
            .iter()
            .map(|(name, dep)| (*name, dep))
            .collect();
        sorted.sort_by_key(|&(name, _)| name);
        for entry in sorted {
            result.push('\t');
            result.push_str(format!("Class {}", entry.0).as_str());
            result.push('\n');
            let mut sorted: Vec<&'a str> = entry.1.methods.iter().map(|s| s.as_str()).collect();
            sorted.sort();
            for method in sorted {
                result.push_str("\t\t");
                result.push_str(
                    format!(
                        "{}Method {}",
                        if entry.1.is_interface {
                            "Iface"
                        } else {
                            "Class"
                        },
                        method
                    )
                    .as_str(),
                );
                result.push('\n');
            }
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Dependency {
    methods: HashSet<String>,
    is_interface: bool,
}

impl Dependency {
    fn add(&mut self, method: String) {
        self.methods.insert(method);
    }
}

trait Consumer<'a> {
    fn get_consumed(&'a self) -> Result<ClassRequirements<'a>, String>;
}

trait Provider {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
        java_classes: &HashMap<&str, ClassInfo>,
    ) -> Result<Option<MethodProvider<'_>>, String>;
}

impl<'a> Consumer<'a> for Class {
    fn get_consumed(&'a self) -> Result<ClassRequirements<'a>, String> {
        static PRIMITIVES: [&str; 8] = ["B", "C", "D", "F", "I", "J", "S", "Z"];

        let mut class_imports = vec![];
        let mut required_class_methods = vec![];
        let mut required_iface_methods = vec![];
        let this_name = self.get_name()?;
        for cp_info in &self.const_pool {
            if let (idx, ConstPoolEntry::Class { name_index }) = cp_info {
                if !self.is_class_entry_used(idx) {
                    continue;
                }
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
                match process_method(class_index, name_type_index, self)? {
                    Some(res) => required_class_methods.push(res),
                    None => continue,
                }
            }
            if let (
                _,
                ConstPoolEntry::IfaceMethodRef {
                    class_index,
                    name_type_index,
                },
            ) = cp_info
            {
                match process_method(class_index, name_type_index, self)? {
                    Some(res) => required_iface_methods.push(res),
                    None => continue,
                }
            }
        }

        let mut deps = HashMap::new();
        for (cls, method) in required_class_methods {
            deps.entry(cls)
                .or_insert(Dependency {
                    methods: HashSet::new(),
                    is_interface: false,
                })
                .add(method);
        }
        for (cls, method) in required_iface_methods {
            deps.entry(cls)
                .or_insert(Dependency {
                    methods: HashSet::new(),
                    is_interface: true,
                })
                .add(method);
        }
        Ok(ClassRequirements {
            name: this_name,
            dependencies: deps,
        })
    }
}

fn process_method<'a>(
    class_index: &u16,
    name_type_index: &u16,
    class: &'a Class,
) -> Result<Option<(&'a str, String)>, String> {
    let ConstPoolEntry::Class { name_index } = &class.const_pool[class_index] else {
        return Err(format!("Not a class info entry at idx {class_index}!"));
    };
    let class_name = class.get_utf8(name_index)?;
    let ConstPoolEntry::NameAndType {
        name_index: method_name_index,
        descriptor_index,
    } = &class.const_pool[name_type_index]
    else {
        return Err(format!("Not a NameAndType entry at idx {name_type_index}!"));
    };
    let method_name = class.get_utf8(method_name_index)?;
    let method_descriptor = class.get_utf8(descriptor_index)?;
    if method_name == "clone" && method_descriptor == "()Ljava/lang/Object;" {
        return Ok(None);
    }
    Ok(Some((
        class_name,
        format!("{method_name}{method_descriptor}"),
    )))
}

struct MethodProvider<'a> {
    name: &'a str,
    methods: HashMap<String, Method>,
}

impl Provider for Class {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
        java_classes: &HashMap<&str, ClassInfo>,
    ) -> Result<Option<MethodProvider<'_>>, String> {
        let mut result = HashMap::default();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            let class_name = self.get_utf8(&name_index)?;
            if !self.is_module() {
                trace!("Processing class {}", class_name);
                for (signature, method) in collect_methods(class_name, classes, java_classes)? {
                    result.insert(signature, method);
                }
                return Ok(Some(MethodProvider {
                    name: class_name,
                    methods: result,
                }));
            }
            trace!("Skipping module-info.class");
            return Ok(None);
        }
        Err("This-class index is invalid!".to_owned())
    }
}

fn collect_methods(
    class_name: &str,
    classes: &HashMap<String, Class>,
    java_classes: &HashMap<&str, ClassInfo<'_>>,
) -> Result<HashMap<String, Method>, String> {
    let mut result = HashMap::default();
    if let Some(current_class) = classes.get(class_name) {
        trace!("Class {}", class_name);
        for method_signature in current_class.get_methods()? {
            result.insert(
                method_signature.clone(),
                Method::new(method_signature.clone()),
            );
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
        result.extend(
            super_class
                .methods
                .iter()
                .map(|(sig, m)| (sig.clone(), m.clone())),
        );
        if let Some(super_class) = super_class.super_class {
            result.extend(collect_methods(super_class, classes, java_classes)?);
        }
        for iface in &super_class.interfaces {
            result.extend(collect_methods(iface, classes, java_classes)?);
        }
    }
    Ok(result)
}

pub fn check_classes<'a>(
    classes: &'a HashMap<String, Class>,
    parallel: bool,
    java_classes: &HashMap<&str, ClassInfo>,
) -> Option<HashSet<ClassRequirements<'a>>> {
    info!("Checking class dependencies");
    let provided = get_provided(classes, parallel, java_classes);
    let mut dependencies: Vec<ClassRequirements<'a>> = Vec::new();
    dependencies.extend(get_consumed(classes, parallel));
    for dep in dependencies.iter_mut() {
        dep.remove_java_classes_and_methods(java_classes);
    }
    debug!(
        "Provided size {} | Dependencies count {}",
        provided.capacity(),
        dependencies.capacity()
    );
    if parallel {
        dependencies.par_iter_mut().for_each(|dep| {
            for (class, method_provider) in &provided {
                dep.remove_methods(class, &method_provider.methods);
                // dep.remove_class(class); TODO figure out how to handle standalone missing
                // methods (e.g. Class is present, but has different API)
            }
        });
    } else {
        for dep in dependencies.iter_mut() {
            for (class, method_provider) in &provided {
                dep.remove_methods(class, &method_provider.methods);
                // dep.remove_class(class); TODO figure out how to handle standalone missing
                // methods (e.g. Class is present, but has different API)
            }
        }
    }
    dependencies.retain(|dep| !dep.is_empty());
    let mut result = HashSet::default();
    result.extend(dependencies);
    info!(
        "Finished. Classes with unmet dependencies: {}",
        result.len()
    );
    Some(result)
}

fn get_consumed(
    classes: &HashMap<String, Class>,
    parallel: bool,
) -> HashSet<ClassRequirements<'_>> {
    if parallel {
        classes
            .par_iter()
            .map(|(_, class)| class.get_consumed().unwrap())
            .fold(HashSet::default, |mut a, b| {
                a.insert(b);
                a
            })
            .reduce(HashSet::default, |mut a, b| {
                a.extend(b);
                a
            })
    } else {
        classes
            .values()
            .map(|class| class.get_consumed().unwrap())
            .fold(HashSet::default(), |mut a, b| {
                a.insert(b);
                a
            })
    }
}

fn get_provided<'a>(
    classes: &'a HashMap<String, Class>,
    parallel: bool,
    java_classes: &HashMap<&str, ClassInfo>,
) -> HashMap<&'a str, MethodProvider<'a>> {
    if parallel {
        classes
            .par_iter()
            .map(|(_, class)| class.get_provided(classes, java_classes).unwrap())
            .filter(|opt| opt.is_some())
            .map(|opt| opt.unwrap())
            .fold(HashMap::default, |mut a, b| {
                a.insert(b.name, b);
                a
            })
            .reduce(HashMap::default, |mut a, b| {
                b.into_iter().for_each(|(k, v)| {
                    a.insert(k, v);
                });
                a
            })
    } else {
        classes
            .values()
            .filter_map(|class| class.get_provided(classes, java_classes).unwrap())
            .fold(HashMap::default(), |mut a, b| {
                a.insert(b.name, b);
                a
            })
    }
}
