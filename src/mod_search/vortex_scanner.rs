use serde_json::Number;
use serde_yaml::Mapping;
use std::fs;

use serde::{Deserialize, Serialize};
use serde_yaml::Sequence;

#[allow(dead_code)]
pub fn read_vortext_mods() {
    // let path = "C:\\Users\\visag\\AppData\\Roaming\\Vortex\\skyrimse\\mods\\vortex.deployment.msgpack";
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PluginFileType {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<serde_yaml::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MasterListFileType {
    plugins: Vec<PluginFileType>,
}

#[allow(unused)]
pub fn get_masterlist_data() -> Vec<PluginFileType> {
    let path = "C:\\Users\\visag\\AppData\\Roaming\\Vortex\\skyrimse\\masterlist\\masterlist.yaml";
    let mut file_contents = fs::read_to_string(path).expect("Could not read file");
    let parsed: MasterListFileType =
        serde_yaml::from_str(file_contents.as_str()).expect("It borked!");
    parsed.plugins
}

#[allow(unused)]
pub fn parse_plugin(plugin_file_type: &PluginFileType) {
    let plugin_name = &plugin_file_type.name;
    if let Some(v) = &plugin_file_type.url {
        match v {
            serde_yaml::Value::Sequence(s) => {
                println!("It Be a sequence {:?}", s)
            },
            _ => ()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_masterlist_data_test() {
        let plugins = get_masterlist_data();

        let p = plugins.get(125).unwrap();
        parse_plugin(p);
        // println!("{:?}", plugins);
    }
}
