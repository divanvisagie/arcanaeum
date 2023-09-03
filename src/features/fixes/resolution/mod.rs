extern crate ini;
use ini::Ini;

use std::path::PathBuf;

fn find_skyrim_settings_file() -> Option<PathBuf> {
    if let Some(mut doc_dir) = dirs::document_dir() {
        doc_dir.push("My Games");
        doc_dir.push("Skyrim Special Edition");
        doc_dir.push("Skyrim.ini");
        if doc_dir.exists() {
            return Some(doc_dir);
        }
    }
    None
}

fn set_skyrim_resolution(file_path: PathBuf, width: u32, height: u32) -> Result<(), String> {
    let mut conf = match Ini::load_from_file(&file_path) {
        Ok(c) => c,
        Err(_) => return Err("Failed to load INI file".to_string()),
    };

    let section = conf
        .section_mut(Some("Display"))
        .ok_or("Section [Display] not found in INI file")?;

    section.insert("iSize W", width.to_string());
    section.insert("iSize H", height.to_string());

    if conf.write_to_file(&file_path).is_ok() {
        Ok(())
    } else {
        Err("Failed to write to INI file".to_string())
    }
}

pub fn fix_resolution() -> Result<(), String> {
    let screen_width = 3440;
    let screen_height = 1440;

    if let Some(settings_file_path) = find_skyrim_settings_file() {
        set_skyrim_resolution(settings_file_path, screen_width, screen_height)
    } else {
        Err("Skyrim settings file not found".to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_skyrim_settings_file() {
        let path = find_skyrim_settings_file();
        assert!(path.is_some());
    }

    #[test]
    fn test_set_skyrim_resolution() {
        let result = fix_resolution();
        assert!(result.is_ok());
    }
}
