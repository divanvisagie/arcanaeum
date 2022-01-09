use super::{types::SkTypeReadable, read_string_of_size};

// Char 13
pub struct SkChar13 {
    name: String,
    value: String,
}
impl SkChar13 {
    #[allow(unused)]
    pub fn from_file(file: &mut std::fs::File, name: &str) -> SkChar13{
        let value = read_string_of_size(file, 13 as u32);
        SkChar13 {
            name: name.to_string(),
            value
        }
    }
    pub fn new(name: &str, value: String) -> SkChar13 {
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
