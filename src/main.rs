/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

mod args;
mod error;
use std::collections::HashMap;

use args::Args;
use clap::Parser;
use env_logger::Env;
use java_class::{
    classinfo::{self, ClassInfo},
    parse_classpath,
};
use log::{debug, info, trace};
use reference_checker::{check_classes, ClassDependencies};

fn main() -> Result<(), error::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    info!("Reading ClassInfo from {}", &args.jdk_classinfo);
    let classinfo_data = std::fs::read_to_string(&args.jdk_classinfo)?;
    let java_classes = read_classinfo(&classinfo_data)?;
    trace!("{:?}", java_classes);
    let classes = parse_classpath(&args.classpath, args.parallel)?;
    let consumed =
        check_classes(&classes, args.parallel, &java_classes).expect("Failed to get result");
    let mut sorted: Vec<ClassDependencies<'_>> = Vec::with_capacity(consumed.capacity());
    sorted.extend(consumed);
    sorted.sort();
    println!(
        "Classpath: {} \n Class count {} \n Consume count: {:?}",
        &args.classpath,
        classes.len(),
        sorted.len()
    );
    debug!("{}", format(sorted));
    Ok(())
}

fn format(dep: Vec<ClassDependencies>) -> String {
    let mut result = String::with_capacity(dep.capacity());
    for d in dep {
        result.push_str(d.format().as_str());
    }
    result
}

fn read_classinfo(data: &str) -> Result<HashMap<&str, ClassInfo>, error::Error> {
    let mut result = HashMap::new();
    let java_classes =
        classinfo::ClassInfo::from_string(data).expect("Failed to read classinfo file!");
    for class_info in java_classes {
        trace!("Converting {}", class_info.name);
        result.insert(class_info.name, class_info);
    }
    Ok(result)
}
