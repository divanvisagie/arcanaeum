use super::{types::SkTypeReadable};

pub enum UIValueType {
    Plugin,
    Header,
    Value
}

pub struct SkUIValue {
    name: String,
    value: String,
    pub value_type: UIValueType
}
impl SkUIValue {
    pub fn new(name: &str, value: String, value_type: UIValueType) -> SkUIValue {
        SkUIValue {
            name: name.to_string(),
            value,
            value_type
        }
    }

}
impl SkTypeReadable for SkUIValue {
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
