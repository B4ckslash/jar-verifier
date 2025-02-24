use java_class::java_class::{Class, ConstPoolEntry};
use regex::Regex;

pub trait Consumer {
    fn get_consumed(&self) -> Vec<String>;
}

pub trait Provider {
    fn get_provided(&self) -> Vec<String>;
}

fn get_references(candidate: &str) -> Vec<String> {
    let re = Regex::new(r"^((?:[[:alnum:]]+/)+[[:alnum:]]+)").expect("Invalid Regex!");
    let mut result = vec![];
    if let Some(caps) = re.captures(candidate) {
        result.push(caps[0].to_owned());
    }
    result
}

fn get_const_pool_entry(pool: &[ConstPoolEntry], idx: usize) -> &ConstPoolEntry {
    &pool[idx - 1]
}

impl Consumer for Class {
    fn get_consumed(&self) -> Vec<String> {
        let mut imports: Vec<String> = vec![];
        for cp_info in &self.const_pool {
            if let ConstPoolEntry::Utf8 { value } = cp_info {
                imports.append(&mut get_references(value));
            }
        }
        imports
    }
}

impl Provider for Class {
    fn get_provided(&self) -> Vec<String> {
        let mut result = vec![];
        if let &ConstPoolEntry::Class { name_index } =
            get_const_pool_entry(&self.const_pool, self.this_class_idx as usize)
        {
            if let ConstPoolEntry::Utf8 { value } =
                get_const_pool_entry(&self.const_pool, name_index as usize)
            {
                result.push(value.to_owned());
            }
        }
        result
    }
}
