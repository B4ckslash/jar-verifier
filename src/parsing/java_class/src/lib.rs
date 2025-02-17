use binrw::prelude::*;

#[binread]
#[br(magic = 0xCAFEBABEu32)]
pub struct Class {
    min_ver: u16,
    maj_ver: u16,
    #[br(temp)]
    const_pool_count: u16,
    #[br(count = const_pool_count)]
    const_pool: Vec<todo!()>,
    access_modifiers: u16, //bitfield
    this_class_idx: u16,
    super_class_idx: u16,
    #[br(temp)]
    iface_count: u16,
    #[br(count = iface_count)]
    iface_indexes: Vec<u16>,
    #[br(temp)]
    fields_count: u16,
    #[br(count = fields_count)]
    fields: Vec<todo!()>,
    #[br(temp)]
    methods_count: u16,
    #[br(count = methods_count)]
    methods: Vec<todo!()>,
    #[br(temp)]
    attr_count: u16,
    #[br(count = attr_count)]
    attributes: Vec<todo!()>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
