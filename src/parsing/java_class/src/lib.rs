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
use log::{debug, info, trace, warn};
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

    let mut multi_release_candidates = HashMap::default();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        if let Some(entry_path) = file.enclosed_name() {
            if let Some(ext) = entry_path.extension() {
                if ext.eq("class") {
                    if entry_path.starts_with(MULTI_RELEASE_PREFIX) {
                        let path_str = entry_path.as_os_str().to_string_lossy();
                        let split = &mut path_str[MULTI_RELEASE_PREFIX.len()..].split('/');
                        if let Some(version) = split.next()
                            && let Ok(class_version) = u16::from_str_radix(version, 10)
                        {
                            if class_version > java_version {
                                debug!(
                                    "Skipping {:?}: class is for Java version {}, which is newer than {}",
                                    entry_path, class_version, java_version
                                );
                            } else if let Some(class_name) = split.last() {
                                multi_release_candidates
                                    .entry(class_name.to_string())
                                    .and_modify(|e: &mut (u16, usize)| {
                                        trace!("Replacing MR candidate for {} in version {} with version {}", class_name, e.0, class_version);
                                        if e.0 < class_version {
                                            e.0 = class_version;
                                            e.1 = i;
                                        }
                                    })
                                    .or_insert((class_version, i));
                            }
                            continue;
                        }
                    }
                    let (class_parsed, class_name) = match read_class(path, file) {
                        Some(value) => value,
                        None => continue,
                    };
                    classes.insert(class_name, class_parsed);
                }
            }
        }
    }
    multi_release_candidates
        .iter()
        .filter_map(|(name, (version, index))| {
            debug!("Using version {} for MR class {}", version, name);
            read_class(
                path,
                archive
                    .by_index(*index)
                    .expect("Could not access archive file by index!"),
            )
        })
        .for_each(|(class, class_name)| {
            if let Some(_) = classes.insert(class_name.clone(), class) {
                trace!("Replaced base class {} with MR version", class_name);
            }
        });
    Ok(classes)
}

fn read_class(
    archive_path: &Path,
    mut file: zip::read::ZipFile<'_, File>,
) -> Option<(Class, String)> {
    let mut file_inmem: Vec<u8> = vec![];
    if file.read_to_end(&mut file_inmem).is_err() {
        warn!(
            "Failed to read zip entry {:?} from {:?}!",
            file.enclosed_name()
                .expect("Could not get path of zip entry!")
                .to_str(),
            archive_path.to_str()
        );
        return None;
    }
    let class_parsed = Class::from(&mut Cursor::new(file_inmem));
    let ConstPoolEntry::Class { name_index } =
        &class_parsed.const_pool[&class_parsed.this_class_idx]
    else {
        return None;
    };
    let Ok(class_name) = class_parsed.get_utf8(name_index).map(ToString::to_string) else {
        return None;
    };
    Some((class_parsed, class_name))
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
