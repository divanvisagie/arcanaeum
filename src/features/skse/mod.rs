use std::path::PathBuf;

use crate::utils::skyrim_se::find_skyrim_documents_path;

pub fn find_skyrim_se_install_path() -> Option<PathBuf> {
    find_skyrim_documents_path()
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

    #[test]
    fn test_find_skyrim_se_install_path() {
        let path = find_skyrim_se_install_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.exists());
    }
}
