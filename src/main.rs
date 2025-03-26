/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

mod args;
mod error;
use std::{collections::HashMap, fs::File, io::Write};

use args::Args;
use clap::Parser;
use env_logger::Env;
use java_class::{
    classinfo::{self, ClassInfo},
    parse_classpath,
};
use log::{info, trace};
use reference_checker::{check_classes, ClassDependencies};

fn main() -> Result<(), error::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    info!("JAR verifier {}", env!("CARGO_PKG_VERSION"));
    info!("Running with {} threads", args.threads);
    info!("Path {}", args.classpath);
    let parallel = args.threads > 1;
    if parallel {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()?;
    }

    #[cfg(feature = "embedded_classinfo")]
    let classinfo_data = include_str!("../data/17.classinfo").to_owned();
    #[cfg(not(feature = "embedded_classinfo"))]
    let classinfo_data = {
        info!("Reading ClassInfo from {}", &args.jdk_classinfo);
        std::fs::read_to_string(&args.jdk_classinfo)?
    };
    let java_classes = read_classinfo(&classinfo_data)?;
    trace!("{:?}", java_classes);

    info!("Starting processing...");
    let classes = parse_classpath(&args.classpath, parallel)?;
    let consumed = check_classes(&classes, parallel, &java_classes).expect("Failed to get result");
    info!("Finished.");
    info!(
        "Class count {} | Classes with unmet requirements: {}",
        classes.len(),
        consumed.len()
    );

    let mut sorted: Vec<ClassDependencies<'_>> = Vec::with_capacity(consumed.capacity());
    sorted.extend(consumed);
    sorted.sort();
    if let Some(path) = args.output_file {
        write_output(&path, &format(sorted))?;
    } else {
        println!("{}", format(sorted));
    }
    Ok(())
}

fn write_output(path: &str, content: &str) -> Result<(), error::Error> {
    info!("Writing results to {}", path);
    let mut outfile = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    outfile.write_all(content.as_bytes())?;
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
