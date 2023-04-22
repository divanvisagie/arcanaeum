use std::{env, fs, io::Error, path::PathBuf};

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

pub fn get_masterlist_data(game: &str) -> Result<Vec<Plugin>, Error> {
    let app_data_path = env::var("APPDATA").unwrap();
    let mut path_buf = PathBuf::new();
    path_buf.push(app_data_path);
    path_buf.push("Vortex");
    path_buf.push(game);
    path_buf.push("masterlist");
    path_buf.push("masterlist.yaml");
    println!("Looking for vortex at: {:?}", path_buf);

    let file_contents = fs::read_to_string(path_buf)?;
    let parsed: MasterListFileType =
        serde_yaml::from_str(file_contents.as_str()).expect("It borked!");
    let plugin_master_list = parsed.plugins.iter().map(|x| parse_plugin(&x)).collect();
    Ok(plugin_master_list)
}

pub fn get_profile_data(profile_name: &str) -> Result<Vec<String>, Error> {
    let app_data_path = env::var("APPDATA").unwrap();
    let mut path_buf = PathBuf::new();
    path_buf.push(app_data_path);
    path_buf.push("Vortex");
    path_buf.push("skyrimse");
    path_buf.push("profiles");
    path_buf.push(profile_name);
    path_buf.push("plugins.txt");

    let file_contents = fs::read_to_string(path_buf)?;
    let plugins_in_profile = file_contents
        .lines()
        .map(|p| p.to_string())
        .filter(|p| p.starts_with("*"))
        .map(|s| {
            let mut chars = s.chars();
            chars.next(); //remove first char
            chars.as_str().to_string()
        })
        .into_iter()
        .collect();
    Ok(plugins_in_profile)
}

pub fn get_profiles(game: &str) -> Result<Vec<String>, Error> {
    let app_data_path = env::var("APPDATA").unwrap();
    let mut path_buf = PathBuf::new();
    path_buf.push(app_data_path);
    path_buf.push("Vortex");
    path_buf.push(game);
    path_buf.push("profiles");

    let read = fs::read_dir(path_buf)?;

    let items = read
        .map(|i| i.unwrap().file_name())
        .map(|i| i.to_str().unwrap().to_string())
        .collect();
    Ok(items)
}

pub fn get_installed_from_all_profiles(game: &str) -> Vec<String> {
    let mut all = Vec::new();
    if let Ok(profiles) = get_profiles(game) {
        for prof in profiles {
            match get_profile_data(&prof) {
                Ok(plugins_in_profile) => {
                    all.extend(plugins_in_profile);
                }
                _ => tracing::error!("Cannot read plugins from profile: {prof}"),
            }
        }
    } else {
        tracing::error!("Could not find any vortex profiles.");
    }
    all
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

    #[test]
    fn get_profile_data_test() {
        let p = get_installed_from_all_profiles("skyrimse");
        println!("String interpolation yay {:?}", p)
    }
}
