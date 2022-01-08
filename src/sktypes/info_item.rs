use crate::sktypes::{read_string_of_size, read_u8, read_u16, read_u32, read_f32, read_wstring};

use super::types::SkType;

pub struct InfoItem {
    name: String,
    size: usize,
    sk_type: SkType,
}

impl InfoItem {
    pub fn new(name: &str, sk_type: SkType) -> InfoItem {
        InfoItem {
            name: name.to_string(),
            size: 0,
            sk_type,
        }
    }

    pub fn new_with_size(name: &str, size: usize, sk_type: SkType) -> InfoItem {
        InfoItem {
            name: name.to_string(),
            size,
            sk_type,
        }
    }

    pub fn print_value(&self, br: &mut std::fs::File) {
        match self.sk_type {
            SkType::Chars => {
                let str = read_string_of_size(br, self.size as u32);
                println!("{}:    {:?}", self.name, str);
            }
            SkType::UInt8 => {
                let u = read_u8(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::UInt16 => {
                let u = read_u16(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::UInt32 => {
                let u = read_u32(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::Float32 => {
                let u = read_f32(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::WString => {
                let str = read_wstring(br);
                println!("{}:    {:?}", self.name, str);
            }
        }
    }
}