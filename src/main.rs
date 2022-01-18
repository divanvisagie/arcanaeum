use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::panic;

use eframe::egui;
use eframe::egui::Color32;
use eframe::epi;
use mod_search::vortex_scanner::get_installed_from_all_profiles;
use mod_search::vortex_scanner::get_masterlist_data;
use mod_search::vortex_scanner::Plugin;
use skyrim_savegame::header::PlayerSex;
use skyrim_savegame::parse_save_file;

use crate::sktypes::skui_value::SkUIValue;
use crate::sktypes::skui_value::UIValueType;
use crate::sktypes::types::SkTypeReadable;

mod mod_search;
mod sktypes;

struct AppState {
    file_path: String,
    values: Vec<SkUIValue>,
    mod_map: HashMap<String, Plugin>,
    installed: HashSet<String>,
    error: Option<String>,
}

impl epi::App for AppState {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Browse to file").clicked() {
                let res = rfd::FileDialog::new()
                    .add_filter("Elder Scrolls Save", &["ess"])
                    .set_directory("./input")
                    .pick_file();

                match res {
                    Some(path_buf) => {
                        self.file_path = String::from(path_buf.to_str().unwrap());
                        match load_save_file(self.file_path.to_string()) {
                            Ok(values) => {
                                self.values = values;
                                self.error = None;
                            },
                            Err(e) => {
                                 self.error = Some(e.to_string());
                                 self.values = Vec::new();
                            },
                        };
                    }
                    None => tracing::error!("No file selected"),
                }
            }
            if let Some(e) = &self.error {
                ui.colored_label(Color32::from_rgb(200, 50, 50), e);
            }                                                                    
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("values")
                    .striped(true)
                    .min_row_height(22.)
                    .min_col_width(300.0)
                    .show(ui, |ui| {
                        for value_entry in self.values.iter() {
                            match value_entry.value_type {
                                UIValueType::Plugin => {
                                    ui.label(value_entry.get_value_string());
                                    match value_entry.plugin_type {
                                        sktypes::skui_value::PluginType::Native => {
                                            ui.label("Original Game File/DLC");
                                        }
                                        sktypes::skui_value::PluginType::CreationClub => {
                                            ui.label("Creation Club Mod");
                                        }
                                        sktypes::skui_value::PluginType::Mod => {
                                            let key = &value_entry.get_value_string();
                                            if self.installed.contains(key) {
                                                ui.colored_label(
                                                    Color32::from_rgb(50, 200, 50),
                                                    "Installed",
                                                );
                                            } else if self.mod_map.contains_key(key) {
                                                let value = self.mod_map.get(key).unwrap();

                                                egui::Grid::new(key.as_str()).show(ui, |ui| {
                                                    for l in value.urls.clone() {
                                                        ui.hyperlink(l.as_str());
                                                        ui.end_row();
                                                    }
                                                });
                                            } else {
                                                ui.colored_label(
                                                    Color32::from_rgb(200, 50, 50),
                                                    "Not Found",
                                                );
                                            }
                                        }
                                        sktypes::skui_value::PluginType::NotAPlugin => {}
                                    }
                                }
                                UIValueType::Header => {
                                    ui.heading(value_entry.get_name());
                                }
                                UIValueType::Value | UIValueType::U32(_) => {
                                    ui.label(value_entry.get_name());
                                    ui.label(value_entry.get_value_string());
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    }

    fn name(&self) -> &str {
        "Arcanaeum"
    }
}

fn load_mod_map() -> HashMap<String, Plugin> {
    let mut map = HashMap::new();
    let plugins = get_masterlist_data();
    for p in plugins {
        map.insert(p.name.clone(), p);
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

    let result = panic::catch_unwind(move || {
        let parsed = parse_save_file(buf.to_vec());
        tracing::info!("{:?}", parsed.plugin_info);
        parsed
    });

    if let Err(e) = result {
        tracing::error!("Error parsing the selected file: {:?}", e);
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("Error Parsing Save File"),
        ));
    }

    let parsed = result.ok().unwrap();

    let mut items: Vec<SkUIValue> = Vec::new();
    items.push(SkUIValue::new(
        "File Info",
        "".to_string(),
        UIValueType::Header,
    ));
    // // Start Header Section
    items.push(SkUIValue::new(
        "File Type",
        parsed.magic,
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

    match parsed.header.player_sex {
        PlayerSex::Male => items.push(SkUIValue::new(
            "Character Sex",
            "Male".to_string(),
            UIValueType::Value,
        )),
        PlayerSex::Female => items.push(SkUIValue::new(
            "Character Sex",
            "Female".to_string(),
            UIValueType::Value,
        )),
    }
    //End Header Section
    items.push(SkUIValue::new(
        "Plugins",
        "".to_string(),
        UIValueType::Header,
    ));

    for plugin in parsed.plugin_info {
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
