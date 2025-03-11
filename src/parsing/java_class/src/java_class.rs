/*
* This Source Code Form is subject to the terms of the
* Mozilla Public License, v. 2.0. If a copy of the MPL was not distributed
* with this file, You can obtain one at http://mozilla.org/MPL/2.0/.
*
* SPDX-License-Identifier: MPL-2.0
*/

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};

use binrw::prelude::*;
use binrw::BinReaderExt;
use modular_bitfield_msb::prelude::*;

#[binread]
#[derive(Debug)]
#[br(magic = 0xCAFEBABEu32)]
pub struct Class {
    min_ver: u16,
    maj_ver: u16,
    #[br(temp)]
    const_pool_count: u16,
    #[br(parse_with = parse_const_pool, args(const_pool_count))]
    pub const_pool: HashMap<u16, ConstPoolEntry>,
    pub flags: ClassFlags,
    pub this_class_idx: u16,
    pub super_class_idx: u16,
    #[br(temp)]
    iface_count: u16,
    #[br(count = iface_count)]
    pub iface_indexes: Vec<u16>,
    #[br(temp)]
    fields_count: u16,
    #[br(count = fields_count)]
    fields: Vec<FieldInfo>,
    #[br(temp)]
    methods_count: u16,
    #[br(count = methods_count)]
    pub methods: Vec<MethodInfo>,
    #[br(temp)]
    attr_count: u16,
    #[br(count = attr_count)]
    pub attributes: Vec<AttributeInfo>,
}

impl Class {
    pub fn from<T>(data: &mut T) -> Self
    where
        T: Read + Seek,
    {
        data.read_be().unwrap()
    }

    pub fn get_utf8<'a>(&'a self, index: &u16) -> Result<&'a str, String> {
        if let ConstPoolEntry::Utf8 { value } = &self.const_pool[index] {
            Ok(value.as_str())
        } else {
            Err(format!("Not a UTF8 entry at idx {}!", index))
        }
    }
    pub fn get_methods(&self) -> Result<HashSet<String>, String> {
        let mut result = HashSet::new();
        for method_info in &self.methods {
            let method_name = self.get_utf8(&method_info.name_index)?;
            let method_descriptor = self.get_utf8(&method_info.descriptor_index)?;
            result.insert(format!("{}{}", method_name, method_descriptor,));
        }
        Ok(result)
    }
}

/*
 * 2 Bytes = 0x0000
 * 0000 0000 0000 0000
 * ||||  ||    ||    1 public
 * ||||  ||    |1      final
 * ||||  ||    1       super
 * ||||  |1            interface
 * ||||  1             abstract
 * |||1                synthetic
 * ||1                 annotation
 * |1                  enum
 * 1                   module
 */
#[bitfield(bytes = 2)]
#[derive(Debug, BinRead)]
#[br(map = Self::from_bytes)]
pub struct ClassFlags {
    #[skip(setters)]
    module: bool,
    #[skip(setters)]
    is_enum: bool,
    #[skip(setters)]
    annotation: bool,
    #[skip(setters)]
    synthetic: bool,
    #[skip]
    __: B1,
    #[skip(setters)]
    is_abstract: bool,
    #[skip(setters)]
    interface: bool,
    #[skip]
    __: B3,
    #[skip(setters)]
    is_super: bool,
    #[skip(setters)]
    is_final: bool,
    #[skip]
    __: B3,
    #[skip(setters)]
    public: bool,
}

fn read_utf8_lossy(data: Vec<u8>) -> String {
    match String::from_utf8(data) {
        Ok(s) => s,
        Err(_) => "N/A".to_owned(),
    }
}

#[binrw::parser(reader, endian)]
fn parse_const_pool(count: u16) -> binrw::BinResult<HashMap<u16, ConstPoolEntry>> {
    let mut result = HashMap::with_capacity(count as usize);
    let mut i = 1;
    while i < count {
        let val = ConstPoolEntry::read_options(reader, endian, ())?;
        //doubles and longs take up two indices, so we manually advance them one further
        let next_i = match val {
            ConstPoolEntry::Long { .. } | ConstPoolEntry::Double { .. } => i + 2,
            _ => i + 1,
        };
        result.insert(i, val);
        i = next_i;
    }
    Ok(result)
}

#[binread]
#[derive(Debug)]
pub enum ConstPoolEntry {
    #[br(magic = 0x01u8)]
    Utf8 {
        #[br(temp)]
        length: u16,
        #[br(count = length, map = |s: Vec<u8>| read_utf8_lossy(s))]
        value: String,
    },
    #[br(magic = 0x07u8)]
    Class { name_index: u16 },
    #[br(magic = 0x09u8)]
    FieldRef {
        class_index: u16,
        name_type_index: u16,
    },
    #[br(magic = 0x0Au8)]
    MethodRef {
        class_index: u16,
        name_type_index: u16,
    },
    #[br(magic = 0x0Bu8)]
    IfaceMethodRef {
        class_index: u16,
        name_type_index: u16,
    },
    #[br(magic = 0x08u8)]
    String { index: u16 },
    #[br(magic = 0x03u8)]
    Int { value: i32 },
    #[br(magic = 0x04u8)]
    Float { value: f32 },
    #[br(magic = 0x05u8)]
    Long { value: i64 },
    #[br(magic = 0x06u8)]
    Double { value: f64 },
    #[br(magic = 0x0Cu8)]
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    #[br(magic = 0x0Fu8)]
    MethodHandle { ref_kind: u8, ref_index: u16 },
    #[br(magic = 0x10u8)]
    MethodType { descriptor_index: u16 },
    #[br(magic = 0x12u8)]
    InvokeDynamic {
        bootstrap_index: u16,
        name_type_index: u16,
    },
    #[br(magic = 0x13u8)]
    Module { name_index: u16 },
    #[br(magic = 0x14u8)]
    Package { name_index: u16 },
}

#[binread]
#[derive(Debug)]
pub struct FieldInfo {
    flags: u16, //bitfield
    name_index: u16,
    descriptor_index: u16,
    #[br(temp)]
    attributes_count: u16,
    #[br(count = attributes_count)]
    attributes: Vec<AttributeInfo>,
}

#[binread]
#[derive(Debug)]
pub struct MethodInfo {
    flags: MethodFlags,
    pub name_index: u16,
    pub descriptor_index: u16,
    #[br(temp)]
    attributes_count: u16,
    #[br(count = attributes_count)]
    attributes: Vec<AttributeInfo>,
}

/*
 * 0000 0000 0000 0000
 *    | || | |||| |||1 public
 *    | || | |||| ||1  private
 *    | || | |||| |1   protected
 *    | || | |||| 1    static
 *    | || | |||1      final
 *    | || | ||1       synchronized
 *    | || | |1        bridge
 *    | || | 1         varargs
 *    | || 1           native
 *    | |1             abstract
 *    | 1              strict
 *    1                synthetic
 */
#[bitfield(bytes = 2)]
#[derive(Debug, BinRead)]
#[br(map = Self::from_bytes)]
pub struct MethodFlags {
    #[skip]
    __: B3,
    #[skip(setters)]
    synthetic: bool,

    #[skip(setters)]
    is_strict: bool,
    #[skip(setters)]
    is_abstract: bool,
    #[skip]
    __: B1,
    #[skip(setters)]
    is_native: bool,

    #[skip(setters)]
    has_varargs: bool,
    #[skip(setters)]
    is_bridge: bool,
    #[skip(setters)]
    is_synchronized: bool,
    #[skip(setters)]
    is_final: bool,

    #[skip(setters)]
    is_static: bool,
    #[skip(setters)]
    is_protected: bool,
    #[skip(setters)]
    is_private: bool,
    #[skip(setters)]
    is_public: bool,
}

#[binread]
#[derive(Debug)]
pub struct AttributeInfo {
    name_index: u16,
    #[br(temp)]
    length: u32,
    #[br(count = length)]
    data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
