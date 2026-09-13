use crate::utils::steam::read_steam_libraries;
use std::path::PathBuf;

const SKYRIM_SE_APP_ID: &str = "489830";
const GAME_FOLDER_NAME: &str = "Skyrim Special Edition";

/// Steam install roots to try, in order, covering the common native and
/// Flatpak layouts on Linux, plus the default Windows location.
fn candidate_steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".local/share/Steam"));
        roots.push(home.join(".steam/steam"));
        roots.push(home.join(".steam/root"));
        roots.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
    }
    roots.push(PathBuf::from("C:/Program Files (x86)/Steam"));
    roots
}

/// Find the Steam library that actually owns the Skyrim Special Edition
/// install, per `libraryfolders.vdf` — not just any library with a
/// `steamapps/common/Skyrim Special Edition` folder lying around, since a
/// stale one from a prior install location can persist.
fn find_owning_library() -> Option<PathBuf> {
    for steam_root in candidate_steam_roots() {
        if !steam_root.is_dir() {
            continue;
        }
        let Ok(libraries) = read_steam_libraries(&steam_root) else {
            continue;
        };
        for library in libraries {
            if library.apps.iter().any(|app| app == SKYRIM_SE_APP_ID) {
                return Some(library.path);
            }
        }
    }
    None
}

fn compatdata_documents_dir(library: &PathBuf) -> PathBuf {
    library
        .join("steamapps/compatdata")
        .join(SKYRIM_SE_APP_ID)
        .join("pfx/drive_c/users/steamuser/Documents/My Games")
        .join(GAME_FOLDER_NAME)
}

/// Find the "My Games/Skyrim Special Edition" documents directory.
///
/// Checks the native Documents folder first (Windows, or a Linux install
/// that mirrors it), then falls back to the Wine prefix Steam Play/Proton
/// creates under the Steam library that owns the Skyrim SE install.
pub fn find_skyrim_documents_path() -> Option<PathBuf> {
    if let Some(mut doc_dir) = dirs::document_dir() {
        doc_dir.push("My Games");
        doc_dir.push(GAME_FOLDER_NAME);
        if doc_dir.exists() {
            return Some(doc_dir);
        }
    }

    let library = find_owning_library()?;
    let path = compatdata_documents_dir(&library);
    if path.exists() { Some(path) } else { None }
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
    let library = find_owning_library()?;
    let path = library.join("steamapps/common").join(GAME_FOLDER_NAME);
    if path.exists() { Some(path) } else { None }
}

/// Find the install's `Data` directory, where plugin (`.esp`/`.esm`/`.esl`)
/// files actually live on disk, whether they were placed there manually or
/// by a mod manager.
pub fn find_skyrim_data_path() -> Option<PathBuf> {
    let mut path = find_skyrim_install_path()?;
    path.push("Data");
    if path.exists() { Some(path) } else { None }
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

    #[test]
    fn test_find_skyrim_data_path() {
        let path = find_skyrim_data_path();
        assert!(path.is_some());
    }
}
