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
use log::{debug, info, trace};
use reference_checker::{ClassDependencies, check_classes};

fn main() -> Result<(), error::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    info!("Version {}", env!("CARGO_PKG_VERSION"));
    #[cfg(feature = "embedded_classinfo")]
    info!("With embedded class information");
    info!("Running with {} threads", args.threads);
    info!("Path {}", args.classpath);
    let parallel = args.threads > 1;
    if parallel {
        rayon::ThreadPoolBuilder::new()
            .num_threads(args.threads)
            .build_global()?;
    }

    #[cfg(feature = "embedded_classinfo")]
    let classinfo_data: HashMap<u16, &'static str> = {
        let mut map = HashMap::new();
        map.insert(11, include_str!("../data/11.classinfo"));
        map.insert(17, include_str!("../data/17.classinfo"));
        map.insert(21, include_str!("../data/21.classinfo"));
        map
    };
    #[cfg(not(feature = "embedded_classinfo"))]
    let classinfo_data = {
        info!("Reading ClassInfo from {}", &args.jdk_classinfo);
        &std::fs::read_to_string(&args.jdk_classinfo)?
    };
    #[cfg(feature = "embedded_classinfo")]
    let classinfo_data = {
        let java_version = &args.java_version.numerical();
        info!("Loading embedded ClassInfo for Java {java_version}");
        classinfo_data
            .get(java_version)
            .expect("Failed to load embedded Class information!")
    };
    let java_classes = read_classinfo(classinfo_data)?;
    trace!("{:?}", java_classes);

    info!("Starting processing...");
    let classes = parse_classpath(&args.classpath, parallel)?;
    let unmet_deps =
        check_classes(&classes, parallel, &java_classes).expect("Failed to get result");
    info!("Done.");

    let mut sorted: Vec<ClassDependencies<'_>> = Vec::with_capacity(unmet_deps.capacity());
    sorted.extend(unmet_deps);
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
    debug!("ClassInfo: {} JDK classes loaded", result.len());
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_java_11() {
        execute_and_compare(11);
    }

    #[test]
    fn test_java_17() {
        execute_and_compare(17);
    }

    #[test]
    fn test_java_21() {
        execute_and_compare(21);
    }

    fn execute_and_compare(version: u16) {
        let pkg_path = env!("CARGO_MANIFEST_DIR");

        let classinfo = load_classinfo(pkg_path, version);
        let java_classes = read_classinfo(&classinfo).unwrap();

        let mut jar_path = pkg_path.to_owned();
        jar_path.push_str("/testdata/test_jar.jar");
        let classes = parse_classpath(jar_path.as_str(), false).unwrap();

        let consumed = check_classes(&classes, false, &java_classes).expect("Failed to get result");

        let mut sorted: Vec<ClassDependencies<'_>> = Vec::with_capacity(consumed.capacity());
        sorted.extend(consumed);
        sorted.sort();

        let formatted = format(sorted);
        let mut compare_path = pkg_path.to_owned();
        compare_path.push_str(format!("/testdata/requirements_{version}.txt").as_str());
        let reference = std::fs::read_to_string(compare_path.as_str()).unwrap();
        assert_eq!(formatted.trim(), reference.trim());
    }

    fn load_classinfo(pkg_path: &str, version: u16) -> String {
        let mut classinfo_path = pkg_path.to_owned();
        classinfo_path.push_str(format!("/data/{version}.classinfo").as_str());
        let classinfo_path = classinfo_path.as_str();
        std::fs::read_to_string(classinfo_path).unwrap()
    }
}
