use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use eframe::egui::Color32;
use eframe::egui::{self};
use eframe::epi;
use egui::Ui;

use crate::components::save_file_selector::SaveFileSelector;
use crate::components::detail_view::{self, DetailView};
use crate::components::selectable_file_item::SelectableItemList;
use crate::parser::SaveInfo;
use crate::sktypes::skui_value::{SkUIValue, UIValueType};
use crate::sktypes::types::SkTypeReadable;
use crate::{load_installed, load_mod_map, sktypes};
use crate::{load_saveinfo_from_path, mod_search::vortex_scanner::Plugin};

#[derive(Clone)]
pub struct AppState {
    pub file_path: String,
    pub save_info: Option<SaveInfo>,
    pub mod_map: HashMap<String, Plugin>,
    pub installed: HashSet<String>,
    pub error: Option<String>,
    pub plugins: Option<Vec<SkUIValue>>,
    pub folder_path: String,
    pub save_file_list: Vec<String>,
}

fn label_line(ui: &mut Ui, name: &str, value: &str) {
    ui.label(name);
    ui.label(value);
    ui.end_row();
}

fn handle_file_selector_click(app_state: &mut AppState, file_selector_callback: impl Fn(&str)) {
    let res = rfd::FileDialog::new()
    .add_filter("Elder Scrolls Save", &["ess"])
    .set_directory("./input")
    .pick_file();

    match res {
        Some(path_buf) => {
            app_state.file_path = String::from(path_buf.to_str().unwrap());
            let new_state = load_savegame_file(app_state.clone());
            *app_state = new_state;
        }
        None => tracing::error!("No file selected"),
    }
}

fn load_savegame_file(ast: AppState) -> AppState {
    let mut app_state = ast.clone();
    let path = app_state.file_path.to_string();
    tracing::info!("Loading file: {}", path);

    match load_saveinfo_from_path(path) {
        Ok(values) => {
            if values.header.is_se {
                app_state.mod_map = load_mod_map("skyrimse");
                app_state.installed = load_installed("skyrimse");
            } else {
                app_state.mod_map = load_mod_map("skyrim");
                app_state.installed = load_installed("skyrim");
            }
            app_state.error = None;

            let mut plugins = Vec::new();
            for plugin_name in &values.plugin_info.plugins {
                let new_plugin = SkUIValue::new(
                    plugin_name.as_str(),
                    plugin_name.to_string(),
                    UIValueType::Plugin,
                );
                plugins.push(new_plugin);
            }

            app_state.plugins = Some(plugins);
            app_state.save_info = Some(values);
        }
        Err(e) => {
            app_state.error = Some(e.to_string());
            app_state.save_info = None;
            app_state.plugins = None;
        }
    };

    return app_state;
}

fn handle_folder_selector_click(app_state: &mut AppState) {
    let res = rfd::FileDialog::new()
    .pick_folder();

    match res {
        Some(path_buf) => {
            app_state.folder_path = String::from(path_buf.to_str().unwrap());
            tracing::info!("Selected folder: {}", app_state.folder_path);

            // List files in folder_path
            let mut files = Vec::new();
            for entry in std::fs::read_dir(app_state.folder_path.to_string()).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() {
                    files.push(path);
                }
            }

            app_state.save_file_list = files.iter().map(|x| x.to_str().unwrap().to_string()).collect();
        }
        None => tracing::error!("No folder selected"),
    }
}

// fn draw_save_file_details(ctx: &egui::CtxRef) {
//     egui::TopBottomPanel::top("top-panel").show(ctx, |ui| {
//         ui.heading("Selected Save File");
//         ui.label("Select a save file to inspect.");
//         ui.separator();
//         if ui.button("Browse to file").clicked() {
//         //   handle_file_selector_click(self);
//         }
//         if let Some(e) = &self.error {
//             ui.colored_label(Color32::from_rgb(200, 50, 50), e);
//         }
//     });
//     egui::CentralPanel::default().show(ctx, |ui| {
//         egui::CentralPanel::default().show(ctx, |_ui| {
//             egui::Grid::new("values")
//                 .striped(true)
//                 .min_row_height(22.)
//                 .min_col_width(400.0)
//                 .show(ui, |ui| {
//                     if let Some(si) = &self.save_info {
//                         if si.header.is_se {
//                             label_line(ui, "Game", "Skyrim Special Edition");
//                         } else {
//                             label_line(ui, "Game", "Skyrim");
//                         }

//                         label_line(
//                             ui,
//                             "Save Number",
//                             si.header.save_number.to_string().as_str(),
//                         );

//                         label_line(ui, "Character Name", si.header.player_name.as_str());
//                         label_line(
//                             ui,
//                             "Character Level",
//                             si.header.player_level.to_string().as_str(),
//                         );
//                         label_line(
//                             ui,
//                             "Character Sex",
//                             si.header.player_sex.to_string().as_str(),
//                         );
//                         label_line(
//                             ui,
//                             "Character Race",
//                             si.header.player_race_editor_id.as_str(),
//                         );
//                         label_line(ui, "In Game Date", si.header.game_date.as_str());
//                         label_line(ui, "Player Location", si.header.player_location.as_str());
//                     }
//                 });
//         });
//         if let Some(_plugins) = &self.plugins {
//             ui.separator();
//             ui.heading("Plugins");
//             ui.separator();
//         }

//         egui::ScrollArea::vertical().show(ui, |ui| {
//             egui::Grid::new("values")
//                 .striped(true)
//                 .min_row_height(22.)
//                 .min_col_width(400.0)
//                 .max_col_width(400.0)
//                 .show(ui, |ui| {
//                     if let Some(plugins) = &self.plugins {
//                         for value_entry in plugins {
//                             ui.label(value_entry.get_name());
//                             match value_entry.plugin_type {
//                                 sktypes::skui_value::PluginType::Native => {
//                                     ui.label("Original Game File/DLC");
//                                 }
//                                 sktypes::skui_value::PluginType::CreationClub => {
//                                     ui.label("Creation Club Mod");
//                                 }
//                                 sktypes::skui_value::PluginType::Mod => {
//                                     let key = &value_entry.get_value_string();
//                                     if self.installed.contains(key) {
//                                         ui.colored_label(
//                                             Color32::from_rgb(50, 200, 50),
//                                             "Installed",
//                                         );
//                                     } else if self.mod_map.contains_key(key) {
//                                         let value = self.mod_map.get(key).unwrap();

//                                         egui::ScrollArea::vertical().show(ui, |ui| {
//                                             for l in value.urls.clone() {
//                                                 ui.hyperlink(l.as_str());
//                                                 ui.end_row();
//                                             }
//                                         });
//                                     } else {
//                                         ui.colored_label(
//                                             Color32::from_rgb(200, 50, 50),
//                                             "Not Found",
//                                         );
//                                     }
//                                 }
//                                 sktypes::skui_value::PluginType::NotAPlugin => {}
//                             }
//                             ui.end_row();
//                         }
//                     }
//                 });
//         });
//     });
// }

impl epi::App for AppState {

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::SidePanel::left("side-panel").show(ctx, |ui| {
            SaveFileSelector::new(&mut self.save_file_list).show(ui, |item| {
                tracing::info!("File was selected: {}", item);
                self.file_path = item.to_string();
                match load_saveinfo_from_path(self.file_path.to_string()) {
                    Ok(save_file) => {
                        self.save_info = Some(save_file);
                    }
                    Err(e) => {
                        self.error = Some(e.to_string());
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            DetailView::new(&mut self.file_path).show(ctx, ui);
        });
    }

    fn name(&self) -> &str {
        "Arcanaeum"
    }
}