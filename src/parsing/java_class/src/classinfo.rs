/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_till},
    character::complete::{char, line_ending, not_line_ending, usize},
    multi::{many, many1, separated_list0},
    sequence::{preceded, terminated},
};

use log::trace;

#[derive(Debug)]
pub struct ClassInfo<'a> {
    pub name: &'a str,
    pub super_class: Option<&'a str>,
    pub interfaces: Vec<&'a str>,
    pub methods: Vec<&'a str>,
}

impl<'a> ClassInfo<'a> {
    fn parse(data: &'a str) -> IResult<&'a str, Self> {
        trace!(
            "Parsing Classinfo from {}[...]",
            if data.len() >= 100 {
                &data[..99]
            } else {
                &data
            }
        );
        let mut colon_terminated = terminated(take_till(|c| c == ':'), char(':'));
        let (remaining, class_name) = colon_terminated.parse(data)?;
        let (remaining, super_class) = colon_terminated.parse(remaining)?;
        let (remaining, interfaces) = colon_terminated.parse(remaining)?;
        let (remaining, methods_count) = terminated(usize, line_ending).parse(remaining)?;

        trace!("Parsing {} methods", methods_count);
        let (remaining, methods) = many(
            methods_count,
            preceded(tag("--"), terminated(not_line_ending, line_ending)),
        )
        .parse(remaining)?;
        trace!(
            "Parsed Class {} with super {}, interfaces {}, and methods {:?}",
            class_name, super_class, interfaces, methods
        );
        let super_class = match super_class {
            "" => None,
            s => Some(s),
        };
        let (_, interfaces) =
            separated_list0(char(','), take_till(|c| c == ',')).parse(interfaces)?;
        Ok((
            remaining,
            ClassInfo {
                name: class_name,
                super_class,
                interfaces,
                methods,
            },
        ))
    }

    pub fn from_string(data: &'a str) -> Result<Vec<ClassInfo<'a>>, String> {
        Ok(many1(ClassInfo::parse)
            .parse(data)
            .map_err(|e| e.to_string())?
            .1)
    }
}
