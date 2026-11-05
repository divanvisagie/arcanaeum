use std::path::PathBuf;

use crate::game;

pub fn find_skyrim_se_install_path() -> Option<PathBuf> {
    game::documents_dir()
}

pub fn download_and_install_skse() {
    // https://github.com/ianpatt/skse64/releases
    print!("Downloading SKSE64...");
}

pub fn lanch_game() {
    if let Err(e) = webbrowser::open("steam://rungameid/489830") {
        tracing::error!("Failed to launch game: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Looks for a real Skyrim install, which a Windows test prefix hasn't (scripts/windows.sh).
    #[cfg_attr(windows, ignore = "needs a real Skyrim install")]
    #[test]
    fn test_find_skyrim_se_install_path() {
        let path = find_skyrim_se_install_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.exists());
    }
}
