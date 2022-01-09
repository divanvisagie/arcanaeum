use std::{mem::size_of, io::Cursor};

use byteorder::{LittleEndian, ReadBytesExt};

use super::{read_buffer, types::SkTypeReadable};


// UInt 32
#[derive(Clone)]
pub struct SkUint32 {
    name: String,
    value: u32
}
impl SkUint32 {
    #[allow(unused)]
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkUint32{ 
        let size = size_of::<u32>();
        let buffer = read_buffer(file, size);
        let mut rdr = Cursor::new(buffer);
        let value = rdr.read_u32::<LittleEndian>().unwrap();
        SkUint32 {
            name: name.to_string(),
            value
        }
    }

    pub fn new(name: &str, value: u32) -> SkUint32 {
        SkUint32 {
            name: name.to_string(),
            value
        }
    }
    #[allow(unused)]
    pub fn get_value(&self) -> u32 {
        self.value
    }
}
impl SkTypeReadable for SkUint32 {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "uint32".to_string()
    }
}