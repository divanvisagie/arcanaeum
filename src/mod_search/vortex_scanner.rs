use std::{
    fs,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

fn vortex_config_dir() -> Result<PathBuf, Error> {
    dirs::config_dir().ok_or_else(|| Error::new(ErrorKind::NotFound, "Could not determine config directory"))
}

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

pub fn get_masterlist_data(game: &str) -> Result<Vec<Plugin>, Error> {
    let mut path_buf = vortex_config_dir()?;
    path_buf.push("Vortex");
    path_buf.push(game);
    path_buf.push("masterlist");
    path_buf.push("masterlist.yaml");
    tracing::info!("Looking for the Vortex masterlist at {:?}", path_buf);

    let file_contents = fs::read_to_string(path_buf)?;
    let parsed: MasterListFileType =
        serde_yaml::from_str(file_contents.as_str()).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    let plugin_master_list = parsed.plugins.iter().map(|x| parse_plugin(&x)).collect();
    Ok(plugin_master_list)
}

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
        let plugins = get_masterlist_data("skyrimse");
        println!("{:?}", plugins);
    }
}
