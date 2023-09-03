use eframe::egui::{self};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::components::detail_view::DetailView;
use crate::components::save_file_selector::{
    get_default_save_folder, read_folder_contents, SaveFileSelector,
};
use crate::load_saveinfo_from_path;
use crate::mod_search::vortex_scanner::Plugin;
use crate::save_file_parser::header::Header;
use crate::save_file_parser::SaveInfo;
use crate::sktypes::skui_value::{SkUIValue, UIValueType};
use crate::{load_installed, load_mod_map};

#[derive(Clone)]
pub struct DetailState {
    pub file_path: String,
    pub save_info: Option<SaveInfo>,
    pub plugins: Option<Vec<SkUIValue>>,
    pub mod_map: HashMap<String, Plugin>,
    pub installed: HashSet<String>,
}

#[derive(Clone)]
pub struct SavesState {
    pub save_file_list: Vec<SaveFile>,
    pub characters: HashMap<String, Character>,
    pub save_folder_path: String,
    pub selected_character: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub error: Option<String>,
    pub folder_path: String,
    pub detail_state: DetailState,
    pub saves_state: SavesState,
    pub show_window: bool,
}

#[derive(Clone)]
pub struct SaveFile {
    pub path: String,
    pub file_name: String,
    pub header: Option<Header>,
}

#[derive(Clone)]
pub struct Character {
    pub name: String,
    pub saves: Vec<SaveFile>,
}

pub fn convert_plugins_to_skui(plugins: &Vec<String>) -> Vec<SkUIValue> {
    let mut skui_plugins = Vec::new();
    for plugin in plugins {
        let new_plugin = SkUIValue::new(plugin.as_str(), plugin.to_string(), UIValueType::Plugin);
        skui_plugins.push(new_plugin);
    }
    skui_plugins
}

fn group_saves_by_character(saves: &Vec<SaveFile>) -> HashMap<String, Character> {
    let mut character_map: HashMap<String, Character> = HashMap::new();
    for save in saves {
        let character = Character {
            name: save.header.as_ref().unwrap().player_name.clone(),
            saves: vec![save.clone()],
        };

        let character_name = save.header.as_ref().unwrap().player_name.clone();
        if character_map.contains_key(&character_name) {
            character_map
                .get_mut(&character_name)
                .unwrap()
                .saves
                .push(save.clone());
        } else {
            character_map.insert(character_name, character);
        }
    }
    character_map
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("side-panel").show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);
            if ui.button("Fix resolution").clicked() {
                //pop up a new window with a button to fix the resolution
                self.show_window = true;
            }
            if self.show_window {
                egui::Window::new("Resolution Fixer")
                    .open(&mut self.show_window)
                    .show(ctx, |ui| {
                        ui.label("Hello from the new window!");
                    });
            }

            SaveFileSelector::new(&mut self.saves_state).show(ui, |item| {
                self.detail_state.file_path = item.path.clone();
                match load_saveinfo_from_path(self.detail_state.file_path.to_string()) {
                    Ok(save_file) => {
                        if save_file.header.is_se {
                            self.detail_state.mod_map = load_mod_map("skyrimse");
                            self.detail_state.installed = load_installed("skyrimse");
                        } else {
                            self.detail_state.mod_map = load_mod_map("skyrim");
                            self.detail_state.installed = load_installed("skyrim");
                        }

                        let plugins = convert_plugins_to_skui(&save_file.plugin_info.plugins);

                        self.detail_state.plugins = Some(plugins);
                        self.detail_state.save_info = Some(save_file);
                    }
                    Err(e) => {
                        self.error = Some(e.to_string());
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            DetailView::new(&mut self.detail_state).show(ctx, ui);
        });
    }
}

impl Default for AppState {
    fn default() -> Self {
        let folder_path = get_default_save_folder();
        let saves = read_folder_contents(folder_path.as_str());
        let characters = group_saves_by_character(&saves);

        Self {
            show_window: false,
            folder_path,
            error: None,
            detail_state: DetailState {
                file_path: String::from(""),
                save_info: None,
                plugins: None,
                mod_map: HashMap::new(),
                installed: HashSet::new(),
            },
            saves_state: SavesState {
                save_file_list: read_folder_contents(get_default_save_folder().as_str()),
                characters,
                save_folder_path: get_default_save_folder(),
                selected_character: None,
            },
        }
    }
}
