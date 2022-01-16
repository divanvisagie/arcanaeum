use std::{env, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Plugin {
    pub name: String,
    pub urls: Vec<String>,
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
pub fn get_masterlist_data() -> Vec<Plugin> {
    let app_data_path = env::var("APPDATA").unwrap();
    let mut path_buf = PathBuf::new();
    path_buf.push(app_data_path);
    path_buf.push("Vortex");
    path_buf.push("skyrimse");
    path_buf.push("masterlist");
    path_buf.push("masterlist.yaml");
    println!("Looking for vortex at: {:?}", path_buf);
   
    let mut file_contents = fs::read_to_string(path_buf).expect("Could not read file");
    let parsed: MasterListFileType =
        serde_yaml::from_str(file_contents.as_str()).expect("It borked!");
    parsed.plugins.iter().map(|x| parse_plugin(&x)).collect()
}

#[allow(unused)]
pub fn parse_plugin(plugin_file_type: &PluginFileType) -> Plugin {
    let name = plugin_file_type.name.clone();
    let mut urls = Vec::new();
    if let Some(v) = &plugin_file_type.url {
        if let serde_yaml::Value::Sequence(seq) = v {
            for item in seq {
                if let serde_yaml::Value::String(s) = item {
                    urls.push(s.clone());
                }
            }
        }
    }
    Plugin { name, urls }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_masterlist_data_test() {
        let plugins = get_masterlist_data();
        println!("{:?}", plugins);
    }
}
