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
use log::{debug, info, trace};
use rayon::prelude::*;

struct ClassRequirements<'a> {
    name: &'a str,
    classes: Vec<&'a str>,
    class_methods: Vec<(&'a str, String)>,
    iface_methods: Vec<(&'a str, String)>,
}

trait Consumer<'a> {
    fn get_consumed(&'a self) -> Result<ClassRequirements<'a>, String>;
}

trait Provider {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
        java_classes: &HashMap<&str, ClassInfo>,
    ) -> Result<Option<MethodProvider>, String>;
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
        Ok(ClassRequirements {
            name: this_name,
            classes: class_imports,
            class_methods: required_class_methods,
            iface_methods: required_iface_methods,
        })
    }
}

fn process_method<'a>(
    class_index: &u16,
    name_type_index: &u16,
    class: &'a Class,
) -> Result<Option<(&'a str, String)>, String> {
    let ConstPoolEntry::Class { name_index } = &class.const_pool[class_index] else {
        return Err(format!("Not a class info entry at idx {}!", class_index));
    };
    let class_name = class.get_utf8(name_index)?;
    let ConstPoolEntry::NameAndType {
        name_index: method_name_index,
        descriptor_index,
    } = &class.const_pool[name_type_index]
    else {
        return Err(format!(
            "Not a NameAndType entry at idx {}!",
            name_type_index
        ));
    };
    let method_name = class.get_utf8(method_name_index)?;
    let method_descriptor = class.get_utf8(descriptor_index)?;
    if method_name == "clone" && method_descriptor == "()Ljava/lang/Object;" {
        return Ok(None);
    }
    Ok(Some((
        class_name,
        format!("{}{}", method_name, method_descriptor),
    )))
}

struct MethodProvider<'a> {
    name: &'a str,
    methods: HashSet<String>,
    interface: bool,
}

impl Provider for Class {
    fn get_provided(
        &self,
        classes: &HashMap<String, Class>,
        java_classes: &HashMap<&str, ClassInfo>,
    ) -> Result<Option<MethodProvider>, String> {
        let mut result = HashSet::new();
        if let &ConstPoolEntry::Class { name_index } = &self.const_pool[&self.this_class_idx] {
            let class_name = self.get_utf8(&name_index)?;
            if !self.is_module() {
                trace!("Processing class {}", class_name);
                for method_signature in collect_methods(class_name, classes, java_classes)? {
                    result.insert(method_signature);
                }
                return Ok(Some(MethodProvider {
                    name: class_name,
                    methods: result,
                    interface: self.is_interface(),
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
        for iface in &super_class.interfaces {
            result.extend(collect_methods(iface, classes, java_classes)?);
        }
    }
    Ok(result)
}

#[derive(Debug, Eq, Clone)]
pub struct ClassDependencies<'a> {
    name: &'a str,
    classes: HashSet<&'a str>,
    class_methods: HashMap<&'a str, HashSet<String>>,
    iface_methods: HashMap<&'a str, HashSet<String>>,
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
        let mut class_methods: HashMap<&'a str, HashSet<String>> = HashMap::new();
        let mut iface_methods: HashMap<&'a str, HashSet<String>> = HashMap::new();
        let mut classes = HashSet::new();
        classes.extend(val.classes);
        for (class, method) in val.class_methods {
            match class_methods.entry(class) {
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
        for (class, method) in val.iface_methods {
            match iface_methods.entry(class) {
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
            class_methods,
            iface_methods,
        }
    }
}

impl<'a> ClassDependencies<'a> {
    fn remove_class(&mut self, class: &str) {
        trace!("Removing class {} from {}", class, self.name);
        self.classes.remove(class);
    }

    fn clear_empty_deps(&mut self) {
        self.class_methods.retain(|_, set| !set.is_empty());
        self.iface_methods.retain(|_, set| !set.is_empty());
    }

    fn remove_methods<'b>(&mut self, class: &'a str, methods: &'b HashSet<String>, iface: bool)
    where
        'a: 'b,
    {
        trace!(
            "Removing methods {:?} of class {} from {}",
            methods, class, self.name
        );
        let entry: Entry<'_, _, _> = if iface {
            self.iface_methods.entry(class)
        } else {
            self.class_methods.entry(class)
        };
        if let Entry::Occupied(mut e) = entry {
            let value = e.get_mut();
            for method in methods {
                trace!("Trying to remove {}#{}", class, method.as_str());
                if value.remove(method) {
                    trace!("Removed {}#{}", class, method.as_str());
                    trace!("Remaining methods for {}: {:?}", class, value);
                }
            }
        }
        self.clear_empty_deps();
    }

    fn remove_java_classes_and_methods(&mut self, java_classes: &HashMap<&str, ClassInfo>) {
        self.classes.retain(|name| !java_classes.contains_key(name));
        for (class_name, methods) in self.class_methods.iter_mut() {
            methods.retain(|method| !Self::provides_method(class_name, method, java_classes));
        }
        for (iface_name, methods) in self.iface_methods.iter_mut() {
            methods.retain(|method| !Self::provides_method(iface_name, method, java_classes));
        }
        self.clear_empty_deps();
    }

    fn provides_method(class: &str, method: &str, java_classes: &HashMap<&str, ClassInfo>) -> bool {
        if let Some(class_info) = java_classes.get(class) {
            if class_info.methods.contains(&method) {
                return true;
            }
            if let Some(super_class) = class_info.super_class {
                if Self::provides_method(super_class, method, java_classes) {
                    return true;
                }
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
        self.classes.is_empty() && self.class_methods.is_empty() && self.iface_methods.is_empty()
    }

    pub fn format(&'a self) -> String {
        let mut result = self.name.to_owned();
        result.push('\n');
        let mut sorted: Vec<&&str> = self.classes.iter().collect();
        sorted.sort();
        for class in sorted {
            result.push('\t');
            result.push_str(format!("Class {}", class).as_str());
            result.push('\n');
        }
        let mut sorted: Vec<(&&str, &HashSet<String>)> = self.class_methods.iter().collect();
        sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
        result.push_str(Self::methods_to_str(&sorted, "ClassMethod").as_str());
        sorted = self.iface_methods.iter().collect();
        sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
        result.push_str(Self::methods_to_str(&sorted, "IfaceMethod").as_str());
        result
    }

    fn methods_to_str(sorted_methods: &Vec<(&&str, &'a HashSet<String>)>, prefix: &str) -> String {
        let mut result: String = Default::default();
        for (class, methods) in sorted_methods {
            let mut sorted: Vec<&String> = methods.iter().collect();
            sorted.sort();
            for method in sorted {
                result.push('\t');
                result.push_str(format!("{} {}#{}", prefix, class, method).as_str());
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
    info!("Checking class dependencies");
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
            for (class, method_provider) in &provided {
                if dep.classes.contains(class) {
                    dep.remove_class(class);
                    dep.remove_methods(class, &method_provider.methods, method_provider.interface);
                }
            }
        });
    } else {
        for dep in dependencies.iter_mut() {
            for (class, methods) in &provided {
                if dep.classes.contains(class) {
                    dep.remove_class(class);
                    dep.remove_methods(class, &methods.methods, methods.interface);
                }
            }
        }
    }
    dependencies.retain(|dep| !dep.is_empty());
    let mut result = HashSet::new();
    result.extend(dependencies);
    info!(
        "Finished. Classes with unmet dependencies: {}",
        result.len()
    );
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
) -> HashMap<&'a str, MethodProvider<'a>> {
    if parallel {
        classes
            .par_iter()
            .map(|(_, class)| class.get_provided(classes, java_classes).unwrap())
            .filter(|opt| opt.is_some())
            .map(|opt| opt.unwrap())
            .fold(HashMap::new, |mut a, b| {
                a.insert(b.name, b);
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
            .filter(|opt| opt.is_some())
            .map(|opt| opt.unwrap())
            .fold(HashMap::new(), |mut a, b| {
                a.insert(b.name, b);
                a
            })
    }
}
