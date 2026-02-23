/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use ahash::AHashMap;
use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_till},
    character::complete::{char, line_ending, usize},
    combinator::opt,
    multi::{many, many1, separated_list0},
    sequence::{preceded, terminated},
};

use log::trace;

type HashMap<K, V> = AHashMap<K, V>;

#[derive(Debug)]
pub struct ClassInfo<'a> {
    pub name: &'a str,
    pub super_class: Option<&'a str>,
    pub interfaces: Vec<&'a str>,
    pub methods: HashMap<String, Method>,
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
        let (remaining, methods): (&str, Vec<Method>) =
            many(methods_count, method).parse(remaining)?;
        trace!(
            "Parsed Class {} with super {}, interfaces {}, and methods {:?}",
            class_name, super_class, interfaces, methods
        );
        let super_class = match super_class {
            "" => None,
            s => Some(s),
        };
        let (_, mut interfaces) =
            separated_list0(char(','), take_till(|c| c == ',')).parse(interfaces)?;
        interfaces.retain(|&s| !s.is_empty());
        Ok((
            remaining,
            ClassInfo {
                name: class_name,
                super_class,
                interfaces,
                methods: methods
                    .iter()
                    .map(|m| (m.signature.to_owned(), m.clone()))
                    .collect(),
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

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Method {
    pub signature: String,
    pub polymorphic_signature: bool,
}

impl Method {
    pub fn new(signature: String) -> Self {
        Method {
            signature,
            polymorphic_signature: false,
        }
    }
}

fn method(input: &str) -> IResult<&str, Method> {
    let (remaining, signature) =
        preceded(tag("--"), take_till(|c| c == ':' || c == '\n')).parse(input)?;
    let (remaining, polymorphic_signature) =
        terminated(opt(preceded(tag(":"), tag("PS"))), line_ending).parse(remaining)?;
    Ok((
        remaining,
        Method {
            signature: signature.to_owned(),
            polymorphic_signature: polymorphic_signature.is_some(),
        },
    ))
}
