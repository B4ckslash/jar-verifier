/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use std::{
    fs::File,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use ahash::AHashMap;
use java_class::{Class, ConstPoolEntry};
use log::{debug, info, warn};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use zip::ZipArchive;

pub mod classinfo;
pub mod error;
pub mod java_class;

type HashMap<K, V> = AHashMap<K, V>;
type Result<T> = std::result::Result<T, error::Error>;

fn read_zip_archive(path: &Path, java_version: u16) -> Result<HashMap<String, Class>> {
    static MULTI_RELEASE_PREFIX: &str = "META-INF/versions/";
    debug!("Processing file {}...", path.to_str().unwrap());
    let file = File::options()
        .read(true)
        .write(false)
        .create_new(false)
        .open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut classes = HashMap::default();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if let Some(entry_path) = file.enclosed_name() {
            if let Some(ext) = entry_path.extension() {
                if ext.eq("class") {
                    if entry_path.starts_with(MULTI_RELEASE_PREFIX) {
                        let path_str = entry_path.as_os_str().to_string_lossy();
                        let split = &mut path_str[MULTI_RELEASE_PREFIX.len()..].split('/');
                        if let Some(version) = split.next()
                            && let Ok(version) = u16::from_str_radix(version, 10)
                        {
                            if version > java_version {
                                debug!(
                                    "Skipping {:?}: class is for Java version {}, which is newer than {}",
                                    entry_path, version, java_version
                                );
                                continue;
                            }
                        }
                    }
                    let mut file_inmem: Vec<u8> = vec![];
                    if file.read_to_end(&mut file_inmem).is_err() {
                        warn!(
                            "Failed to read zip entry {:?} from {:?}!",
                            entry_path.to_str(),
                            path.to_str()
                        );
                        continue;
                    }
                    let class_parsed = Class::from(&mut Cursor::new(file_inmem));
                    let ConstPoolEntry::Class { name_index } =
                        &class_parsed.const_pool[&class_parsed.this_class_idx]
                    else {
                        continue;
                    };
                    let Ok(class_name) = class_parsed.get_utf8(name_index) else {
                        continue;
                    };
                    classes.insert(class_name.to_owned(), class_parsed);
                }
            }
        }
    }
    Ok(classes)
}

pub fn parse_classpath(cp: &str, java_version: u16) -> Result<HashMap<String, Class>> {
    info!("Processing class path");
    let split = cp.split(';');
    let expanded = split
        .map(|el| shellexpand::full(el).unwrap_or_else(|_| panic!("Failed to expand path {el}")));
    let globbed = expanded
        .clone()
        .filter_map(|el| {
            if el.contains('*') {
                Some(glob::glob(el.as_ref()).unwrap().map(|p| p.unwrap()))
            } else {
                None
            }
        })
        .flatten();
    let concrete = expanded.filter_map(|el| {
        if !el.contains('*') {
            let mut p = PathBuf::new();
            p.push(el.as_ref());
            Some(p)
        } else {
            None
        }
    });
    let chained: Vec<PathBuf> = globbed.chain(concrete).collect();
    debug!("{} JAR files found.", chained.len());
    let result = chained
        .par_iter()
        .map(|pb| read_zip_archive(pb.as_path(), java_version).unwrap())
        .reduce(HashMap::default, |a, mut b| {
            a.into_iter().for_each(|(k, v)| {
                b.insert(k, v);
            });
            b
        });

    info!("Finished. {} classes found.", result.len());
    Ok(result)
}
