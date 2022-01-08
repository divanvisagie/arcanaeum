use std::{mem::size_of, io::{Cursor, Read}};

use byteorder::{LittleEndian, ReadBytesExt};

use super::{read_string_of_size, read_buffer, read_u16};

pub trait SkTypeReadable {
    fn get_value_string(&self) -> String;
    fn get_name(&self) -> String;
    fn get_type(&self) -> String;
}

// Char 13
pub struct SkChar13 {
    name: String,
    value: String,
}
impl SkChar13 {
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkChar13{
        let value = read_string_of_size(file, 13 as u32);
        SkChar13 {
            name: name.to_string(),
            value
        }
    }
}
impl SkTypeReadable for SkChar13 {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "char[13]".to_string()
    }
}


// UInt 32
#[derive(Clone)]
pub struct SkUint32 {
    name: String,
    value: u32
}
impl SkUint32 {
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

// Wstring
pub struct SkWstring {
    name: String,
    value: String
}
impl SkWstring {
    
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkWstring{ 
        let size = read_u16(file);
        let value = read_string_of_size(file, size.into());
        SkWstring {
            name: name.to_string(),
            value
        }
    }
}
impl SkTypeReadable for SkWstring {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "wstring".to_string()
    }
}

// UInt 16
pub struct SkUint16 {
    name: String,
    value: u16
}
impl SkUint16 {
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkUint16{ 
        let size = size_of::<u16>();
        let buffer = read_buffer(file, size);
        let mut rdr = Cursor::new(buffer);
        let value = rdr.read_u16::<LittleEndian>().unwrap();
        SkUint16 {
            name: name.to_string(),
            value
        }
    }
}
impl SkTypeReadable for SkUint16 {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "uint16".to_string()
    }
}

// UInt 8
#[derive(Clone)]
pub struct SkUint8 {
    name: String,
    pub value: u8
}


impl SkUint8 {
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkUint8 { 
        let size = size_of::<u8>();
        let buffer = read_buffer(file, size);
        let value = buffer[0];
        SkUint8 {
            name: name.to_string(),
            value
        }
    }

    pub fn get_value(&self) -> u8 {
        self.value
    }
}
impl SkTypeReadable for SkUint8 {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "uint16".to_string()
    }
}



// Float 32
pub struct SkFloat32 {
    name: String,
    value: f32
}
impl SkFloat32 {
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkFloat32{ 
        let size = size_of::<f32>();
        let buffer = read_buffer(file, size);
        let mut rdr = Cursor::new(buffer);
        let value = rdr.read_f32::<LittleEndian>().unwrap();
        SkFloat32 {
            name: name.to_string(),
            value
        }
    }
}
impl SkTypeReadable for SkFloat32 {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "float32".to_string()
    }
}


// Unknown
// Used for handling any unknown type gracefully
pub struct SkUnknown {
    name: String,
    value: String //We store this for debugging so that we can eventually support the type
}
impl SkUnknown {
    pub fn from_file(file: &mut std::fs::File, name: &str, size: usize) -> SkUnknown{ 
        let buffer = read_buffer(file, size);
        SkUnknown {
            name: name.to_string(),
            value: format!("{:?}", buffer)
        }
    }
}
impl SkTypeReadable for SkUnknown {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "binary (dec)".to_string()
    }
}
