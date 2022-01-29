use std::collections::{HashMap, HashSet};

use eframe::egui;
use eframe::egui::Color32;
use eframe::epi;

use crate::{
    load_installed, load_mod_map, load_save_file,
    mod_search::vortex_scanner::Plugin,
    sktypes::{
        self,
        skui_value::{SkUIValue, UIValueType},
        types::SkTypeReadable,
    },
};

pub struct AppState {
    pub file_path: String,
    pub values: Vec<SkUIValue>,
    pub mod_map: HashMap<String, Plugin>,
    pub installed: HashSet<String>,
    pub error: Option<String>,
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
                                let (dts, is_se) = values;
                                if is_se {
                                    self.mod_map = load_mod_map("skyrimse");
                                    self.installed = load_installed("skyrimse")
                                } else {
                                    self.mod_map = load_mod_map("skyrim");
                                    self.installed = load_installed("skyrim");
                                }
                                self.values = dts;
                                self.error = None;
                            }
                            Err(e) => {
                                self.error = Some(e.to_string());
                                self.values = Vec::new();
                            }
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
