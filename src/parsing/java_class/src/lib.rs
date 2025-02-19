pub mod java_class {
    use binrw::prelude::*;

    #[binread]
    #[derive(Debug)]
    #[br(magic = 0xCAFEBABEu32)]
    pub struct Class {
        min_ver: u16,
        maj_ver: u16,
        #[br(temp)]
        const_pool_count: u16,
        #[br(parse_with = parse_const_pool, args(const_pool_count))]
        pub const_pool: Vec<ConstPoolEntry>,
        access_modifiers: u16, //bitfield
        pub this_class_idx: u16,
        super_class_idx: u16,
        #[br(temp)]
        iface_count: u16,
        #[br(count = iface_count)]
        iface_indexes: Vec<u16>,
        #[br(temp)]
        fields_count: u16,
        #[br(count = fields_count)]
        fields: Vec<FieldInfo>,
        #[br(temp)]
        methods_count: u16,
        #[br(count = methods_count)]
        methods: Vec<MethodInfo>,
        #[br(temp)]
        attr_count: u16,
        #[br(count = attr_count)]
        pub attributes: Vec<AttributeInfo>,
    }

    fn read_utf8_lossy(data: Vec<u8>) -> String {
        match String::from_utf8(data) {
            Ok(s) => s,
            Err(_) => "N/A".to_owned(),
        }
    }

    #[binrw::parser(reader, endian)]
    fn parse_const_pool(count: u16) -> binrw::BinResult<Vec<ConstPoolEntry>> {
        let mut result = Vec::with_capacity(count as usize);
        let mut i = 1;
        while i < count {
            let val = ConstPoolEntry::read_options(reader, endian, ())?;
            //doubles and longs take up two indices, so we manually advance them one further
            match val {
                ConstPoolEntry::Long { .. } | ConstPoolEntry::Double { .. } => i += 2,
                _ => i += 1,
            };
            result.push(val);
        }
        Ok(result)
    }

    #[binread]
    #[derive(Debug)]
    pub enum ConstPoolEntry {
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
        #[br(magic = 0x01u8)]
        Utf8 {
            #[br(temp)]
            length: u16,
            #[br(count = length, map = |s: Vec<u8>| read_utf8_lossy(s))]
            value: String,
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
}
