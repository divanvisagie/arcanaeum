use crate::sktypes::{read_string_of_size, read_u8, read_u16, read_u32, read_f32, read_wstring};

use super::{types::SkType, read_unknown};

pub struct InfoItem {
    name: String,
    sk_type: SkType,
    value: String
}

pub trait PrintableInfoItem {
    fn print_value(&self);
}

impl InfoItem {
    pub fn new(file: &mut std::fs::File, name: &str, sk_type: SkType) -> InfoItem {
        tracing::info!("Parsing {:?} of type {:?}", name, sk_type);
        match sk_type {
            SkType::Char13 => {
                let str = read_string_of_size(file, 13 as u32);
                InfoItem {
                    name: name.to_string(),
                    sk_type,
                    value: str.to_string()
                }
            }
            SkType::UInt8 => {
                let v = read_u8(file);
                InfoItem {
                    name: name.to_string(),
                    sk_type,
                    value: v.to_string()
                }
            },
            SkType::UInt32 => {
                let v = read_u32(file);
                InfoItem {
                    name: name.to_string(),
                    sk_type,
                    value: v.to_string()
                }
            },
            SkType::UInt16 => {
                let v = read_u16(file);
                InfoItem {
                    name: name.to_string(),
                    sk_type,
                    value: v.to_string()
                }
            },
            SkType::Float32 => {
                let v = read_f32(file);
                InfoItem {
                    name: name.to_string(),
                    sk_type,
                    value: v.to_string()
                }
            },
            SkType::WString => {
                let str = read_wstring(file);
                InfoItem {
                    name: name.to_string(),
                    sk_type,
                    value: str.to_string()
                }
            },
            SkType::Unknown => {
                let str = read_unknown(file, 8);
                InfoItem {
                    name: name.to_string(),
                    sk_type,
                    value: str.to_string()
                }
            },
        }
    }

    pub fn print_value(&self) {
        println!("{}                         : {:?}                                         ({:?})", self.name, self.value, self.sk_type)
    }
}