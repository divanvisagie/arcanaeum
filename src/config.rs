use std::env;
use std::fs;
use std::path::PathBuf;

fn get_config_folder_path() -> PathBuf {
    let app_data_path = env::var("APPDATA").unwrap();
    let mut path_buf = PathBuf::new();
    path_buf.push(app_data_path);
    path_buf.push("Arcaneum");
    path_buf
}
fn get_config_path() -> PathBuf {
    let mut path_buf = get_config_folder_path();
    path_buf.push("config.json");
    path_buf
}

fn create_config_file_if_not_exists() {
    let path_buf = get_config_path();

    if !path_buf.exists() {
        let config = r#"{
        }"#;

        fs::write(path_buf, config).expect("Could not create config file");
    }
}

fn create_config_folder_if_not_exists() {
    let app_data_path = env::var("APPDATA").unwrap();
    let mut path_buf = PathBuf::new();
    path_buf.push(app_data_path);
    path_buf.push("Arcaneum");

    if !path_buf.exists() {
        fs::create_dir(path_buf).expect("Could not create config directory");
    }
}

pub fn create_config_if_not_exists() {
    create_config_folder_if_not_exists();
    create_config_file_if_not_exists();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_config_if_not_exists() {
        create_config_if_not_exists();
        let app_data_path = env::var("APPDATA").unwrap();
        let mut path_buf = PathBuf::new();
        path_buf.push(app_data_path);
        path_buf.push("Arcaneum");
        assert!(path_buf.exists());
    }
}
