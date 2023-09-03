use crate::utils::steam::read_steam_library_folders;
use std::path::PathBuf;

pub fn find_skyrim_settings_file() -> Option<PathBuf> {
    if let Some(mut doc_dir) = dirs::document_dir() {
        doc_dir.push("My Games");
        doc_dir.push("Skyrim Special Edition");
        doc_dir.push("SkyrimPrefs.ini");
        if doc_dir.exists() {
            return Some(doc_dir);
        }
    }
    None
}

/**
 * Find the Skyrim Special Edition install path.
 */
pub fn find_skyrim_install_path() -> Option<PathBuf> {
    let steam_path = PathBuf::from("C:/Program Files (x86)/Steam"); // Replace with actual Steam path
    if let Ok(libraries) = read_steam_library_folders(&steam_path) {
        for library in libraries {
            let skyrim_path = library.join("steamapps/common/Skyrim Special Edition");
            if skyrim_path.exists() {
                return Some(skyrim_path);
            }
        }
    }

    None
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
    fn test_find_skyrim_install_path() {
        let path = find_skyrim_install_path();
        assert!(path.is_some());
    }
}
