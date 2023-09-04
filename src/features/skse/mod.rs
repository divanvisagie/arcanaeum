use std::{path::PathBuf, process::Command};

pub fn find_skyrim_se_install_path() -> Option<PathBuf> {
    let mut path = dirs::document_dir()?;
    path.push("My Games");
    path.push("Skyrim Special Edition");
    Some(path)
}

pub fn download_and_install_skse() {
    // https://github.com/ianpatt/skse64/releases
    print!("Downloading SKSE64...");
}

pub fn lanch_game() {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C");
    cmd.arg("start");
    cmd.arg("steam://rungameid/489830");
    cmd.spawn().expect("failed to execute process");
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
