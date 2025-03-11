/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*/

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    ///Classpath of JARs to be checked
    pub classpath: String,
    ///A file listing the available classes and methods of the relevant JDK
    pub jdk_classinfo: String,
    ///Whether the program runs in parallel
    #[arg(short, long, default_value_t = false)]
    pub parallel: bool,
}
