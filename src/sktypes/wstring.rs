use super::{read_u16, types::SkTypeReadable, read_string_of_size};


// Wstring
pub struct SkWstring {
    name: String,
    value: String
}
impl SkWstring {
    #[allow(unused)]
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkWstring{ 
        let size = read_u16(file);
        let value = read_string_of_size(file, size.into());
        SkWstring {
            name: name.to_string(),
            value
        }
    }

    pub fn new(name: &str, value: String) -> SkWstring {
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
