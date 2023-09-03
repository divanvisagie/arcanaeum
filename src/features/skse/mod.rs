pub find_skyrim_se_install_path() -> Option<PathBuf> {
    let mut path = dirs::document_dir()?;
    path.push("My Games");
    path.push("Skyrim Special Edition");
    Some(path)
}


pub fn lanch_game() {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C");
    cmd.arg("start");
    cmd.arg("steam://rungameid/489830");
    cmd.spawn().expect("failed to execute process");
}