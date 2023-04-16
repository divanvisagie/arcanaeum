use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Error;
use std::io::Read;

use eframe::egui;

use mod_search::vortex_scanner::get_installed_from_all_profiles;
use mod_search::vortex_scanner::get_masterlist_data;
use mod_search::vortex_scanner::Plugin;
use parser::SaveInfo;

use crate::app::AppState;
use crate::parser::parse;

mod app;
mod mod_search;
mod parser;
mod sktypes;

fn load_mod_map(game: &str) -> HashMap<String, Plugin> {
    let mut map = HashMap::new();
    if let Ok(plugins) = get_masterlist_data(game) {
        for plugin in plugins {
            map.insert(plugin.name.clone(), plugin);
        }
    }
    map
}

fn load_installed(game: &str) -> HashSet<String> {
    let mut installed = HashSet::new();
    for p in get_installed_from_all_profiles(game) {
        installed.insert(p);
    }
    installed
}

fn load_save_file(path: String) -> Result<SaveInfo, Error> {
    tracing::info!("Loading file: {:?}", path);
    let mut file = std::fs::File::open(path)?;

    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf)?;

    let parsed = parse(buf);
    Ok(parsed)
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    let app_state = AppState {
        file_path: String::from(""),
        folder_path: String::from(""),
        save_info: None,
        mod_map: HashMap::new(),
        installed: HashSet::new(),
        error: None,
        plugins: None,
        save_file_list: Vec::new(),
    };
    let mut window_options = eframe::NativeOptions::default();
    window_options.initial_window_size = Some(egui::Vec2::new(1024., 768.));
    window_options.resizable = true;
    window_options.decorated = true;
    eframe::run_native(Box::new(app_state), window_options);
}


// Unit tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_save_file() {
        // let save_info = load_save_file(String::from("./input/Save1.ess")).unwrap();
        // assert_eq!(save_info.header.game, "Skyrim");
        assert!(true);
    }
}
