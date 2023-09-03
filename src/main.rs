use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Error;
use std::io::Read;

use eframe::egui;

use mod_search::vortex_scanner::get_installed_from_all_profiles;
use mod_search::vortex_scanner::get_masterlist_data;
use mod_search::vortex_scanner::Plugin;
use save_file_parser::SaveInfo;

use crate::app::AppState;
use crate::config::create_config_if_not_exists;
use crate::save_file_parser::parse;

mod app;
mod components;
mod config;
mod features;
mod mod_search;
mod save_file_parser;
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

fn load_saveinfo_from_path(path: String) -> Result<SaveInfo, Error> {
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

    create_config_if_not_exists();

    let icon_data = {
        let icon_raw = include_bytes!("../assets/icon-256.png");
        let image = image::load_from_memory(icon_raw).expect("icon must be valid");
        let image = image.to_rgba8();
        eframe::IconData {
            width: image.width(),
            height: image.height(),
            rgba: image.into_vec(),
        }
    };

    let mut window_options = eframe::NativeOptions::default();
    window_options.initial_window_size = Some(egui::Vec2::new(1280., 768.));
    window_options.resizable = true;
    window_options.decorated = true;
    window_options.icon_data = Some(icon_data);

    match eframe::run_native(
        "Arcanaeum",
        window_options,
        Box::new(|_cc| Box::<AppState>::default()),
    ) {
        Ok(_) => {}
        Err(e) => {
            tracing::error!("Error: {}", e);
        }
    }
}
