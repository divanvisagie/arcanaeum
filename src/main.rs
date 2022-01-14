/**
 * Author: Divan Visagie
 * https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format
 * https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions
*/
use std::io::Read;

use eframe::egui;
use eframe::epi;
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
}

impl epi::App for AppState {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Browse to file").clicked() {
                let res = rfd::FileDialog::new()
                    .add_filter("Elder Scrolls Save", &["ess"])
                    .set_directory("./input")
                    .pick_file()
                    .unwrap()
                    .into_os_string();

                self.file_path = String::from(res.to_str().unwrap());
                self.values = read_file(self.file_path.to_string());
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
                                        },
                                        sktypes::skui_value::PluginType::CreationClub => {
                                            if ui.button("Search Creation Club").clicked() {
                                                tracing::info!("Search creation club");
                                            }
                                        },
                                        sktypes::skui_value::PluginType::Mod => {  
                                            if ui.button("Search Nexus Mods").clicked() {
                                                tracing::info!("Search for mod");
                                            }
                                        },
                                        sktypes::skui_value::PluginType::NotAPlugin => {

                                        },
                                    }
                                }
                                UIValueType::Header => {
                                    ui.heading(value_entry.get_name());
                                }
                                UIValueType::Value => {
                                    ui.label(value_entry.get_name());
                                    ui.label(value_entry.get_value_string());
                                }
                            }
                            ui.end_row();
                        }
                    });
                ui.hyperlink("https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format");
            });

        });
    }

    fn name(&self) -> &str {
        "Arcanaeum"
    }
}

fn read_file(path: String) -> Vec<SkUIValue> {
    tracing::info!("Loading file: {:?}", path);
    let mut file = std::fs::File::open(path)
        .map_err(|err| {
            println!("Error {:?}", err);
        })
        .ok()
        .unwrap();

    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf).expect("Could not read file!");
    let parsed = parse_save_file(buf.to_vec());

    tracing::info!("{:?}", parsed.plugin_info);

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
        UIValueType::Value,
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

    items
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    let app_state = AppState {
        file_path: String::from(""),
        values: Vec::with_capacity(150),
    };
    let mut window_options = eframe::NativeOptions::default();
    window_options.initial_window_size = Some(egui::Vec2::new(600., 768.));
    window_options.decorated = true;
    eframe::run_native(Box::new(app_state), window_options);
}
