use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::panic;

use eframe::egui;

use mod_search::vortex_scanner::get_installed_from_all_profiles;
use mod_search::vortex_scanner::get_masterlist_data;
use mod_search::vortex_scanner::Plugin;

use crate::app::AppState;
use crate::parser::parse;
use crate::sktypes::skui_value::SkUIValue;
use crate::sktypes::skui_value::UIValueType;

mod app;
mod mod_search;
mod parser;
mod sktypes;

fn load_mod_map() -> HashMap<String, Plugin> {
    let mut map = HashMap::new();
    if let Ok(plugins) = get_masterlist_data() {
        for plugin in plugins {
            map.insert(plugin.name.clone(), plugin);
        }
    }
    map
}

fn load_installed() -> HashSet<String> {
    let mut installed = HashSet::new();
    for p in get_installed_from_all_profiles() {
        installed.insert(p);
    }
    installed
}

fn load_save_file(path: String) -> Result<Vec<SkUIValue>, Error> {
    tracing::info!("Loading file: {:?}", path);
    let mut file = std::fs::File::open(path)?;

    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf)?;

    let parsed = parse(buf);

    let mut items: Vec<SkUIValue> = Vec::new();
    items.push(SkUIValue::new(
        "File Info",
        "".to_string(),
        UIValueType::Header,
    ));
    // // Start Header Section
    items.push(SkUIValue::new(
        "File Type",
        parsed.magic_string,
        UIValueType::Value,
    ));
    items.push(SkUIValue::new(
        "Save Number",
        parsed.header.save_number.to_string(),
        UIValueType::U32(parsed.header.save_number),
    ));
    items.push(SkUIValue::new(
        "Character Name",
        parsed.header.player_name,
        UIValueType::Value,
    ));
    items.push(SkUIValue::new(
        "Character Level",
        parsed.header.player_level.to_string(),
        UIValueType::Value,
    ));
    items.push(SkUIValue::new(
        "Current Location",
        parsed.header.player_location,
        UIValueType::Value,
    ));

    items.push(SkUIValue::new(
        "In-game date",
        parsed.header.game_date,
        UIValueType::Value,
    ));
    items.push(SkUIValue::new(
        "Character Race",
        parsed.header.player_race_editor_id,
        UIValueType::Value,
    ));

    items.push(SkUIValue::new(
        "Character Sex",
        parsed.header.player_sex.to_string(),
        UIValueType::Value,
    ));

    //End Header Section
    items.push(SkUIValue::new(
        "Plugins",
        "".to_string(),
        UIValueType::Header,
    ));

    for plugin in parsed.plugin_info.plugins {
        items.push(SkUIValue::new("Plugin", plugin, UIValueType::Plugin));
    }

    Ok(items)
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    let app_state = AppState {
        file_path: String::from(""),
        values: Vec::with_capacity(150),
        mod_map: load_mod_map(),
        installed: load_installed(),
        error: None,
    };
    let mut window_options = eframe::NativeOptions::default();
    window_options.initial_window_size = Some(egui::Vec2::new(800., 768.));
    window_options.decorated = true;
    eframe::run_native(Box::new(app_state), window_options);
}
