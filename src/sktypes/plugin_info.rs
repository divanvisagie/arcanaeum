use std::io::Read;

use crate::sktypes::{read_string_of_size};

use super::types::SkTypeReadable;


pub struct PluginInfo {
    value: Vec<String>,
    name: String
}

impl PluginInfo {
    #[allow(unused)]
    pub fn from_file(file: &mut std::fs::File, name: &str, item_count: u32, total_size: u32) -> PluginInfo {
        let end_of_sector = total_size - 2; // 2 is the size of u16 which was the first value read in the plugin info section already

        let str = read_string_of_size(file, 3);
        tracing::info!(">> {:?}", str);

        let mut buffer = Vec::with_capacity(end_of_sector.try_into().unwrap());
        file.take(end_of_sector.into())
            .read_to_end(&mut buffer)
            .map_err(|err| tracing::error!("{:?}", err))
            .ok();

        let count = item_count.clone();
        PluginInfo {
            name: name.to_string(),
            value : Vec::with_capacity(count.try_into().unwrap())
        }
    }
}
impl SkTypeReadable for PluginInfo {
    fn get_value_string(&self) -> String {
        format!("{:?}", self.value)
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "wstring[]".to_string()
    }
}
