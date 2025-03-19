/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use clap::Parser;

#[derive(Parser, Debug)]
#[cfg_attr(
    feature = "embedded_classinfo",
    command(
        version,
        about,
        long_about = env!("CARGO_PKG_DESCRIPTION").to_owned() + "\nCompiled with embedded class information"
    )
)]
#[cfg_attr(not(feature = "embedded_classinfo"), command(version, about))]
pub struct Args {
    ///Classpath of JARs to be checked.
    pub classpath: String,
    ///A file listing the available classes and methods of the relevant JDK.
    #[cfg(not(feature = "embedded_classinfo"))]
    pub jdk_classinfo: String,
    ///The number of threads to use.
    #[arg(short, long, default_value_t = 1usize)]
    pub threads: usize,
    ///The output file path. Prints to stdout if not set.
    #[arg(short, long)]
    pub output_file: Option<String>,
}
