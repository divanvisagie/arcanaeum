use crate::utils::steam::read_steam_library_folders;
use std::path::PathBuf;

const SKYRIM_SE_APP_ID: &str = "489830";

/// Steam install roots to look for a `libraryfolders.vdf` in, across platforms.
fn candidate_steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".steam/root"));
    }
    roots.push(PathBuf::from("C:/Program Files (x86)/Steam"));
    roots
}

/// All Steam library folders across every discoverable Steam install.
fn find_steam_libraries() -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    for root in candidate_steam_roots() {
        if !root.exists() {
            continue;
        }
        libraries.push(root.clone());
        if let Ok(extra) = read_steam_library_folders(&root) {
            libraries.extend(extra);
        }
    }
    libraries
}

/// Find the "My Games/Skyrim Special Edition" documents directory.
///
/// Checks the native Documents folder first (Windows, or a Linux install
/// that mirrors it), then falls back to the Wine prefix Steam Play/Proton
/// creates under each Steam library's `steamapps/compatdata`.
pub fn find_skyrim_documents_path() -> Option<PathBuf> {
    if let Some(mut doc_dir) = dirs::document_dir() {
        doc_dir.push("My Games");
        doc_dir.push("Skyrim Special Edition");
        if doc_dir.exists() {
            return Some(doc_dir);
        }
    }

    for library in find_steam_libraries() {
        let mut path = library;
        path.push("steamapps/compatdata");
        path.push(SKYRIM_SE_APP_ID);
        path.push("pfx/drive_c/users/steamuser/Documents/My Games/Skyrim Special Edition");
        if path.exists() {
            return Some(path);
        }
    }

    None
}

pub fn find_skyrim_settings_file() -> Option<PathBuf> {
    let mut path = find_skyrim_documents_path()?;
    path.push("SkyrimPrefs.ini");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

pub fn find_default_save_folder() -> Option<PathBuf> {
    let mut path = find_skyrim_documents_path()?;
    path.push("Saves");
    Some(path)
}

/// Find the Skyrim Special Edition install path.
pub fn find_skyrim_install_path() -> Option<PathBuf> {
    for library in find_steam_libraries() {
        let skyrim_path = library.join("steamapps/common/Skyrim Special Edition");
        if skyrim_path.exists() {
            return Some(skyrim_path);
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
