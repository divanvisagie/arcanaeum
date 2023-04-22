use eframe::egui::{self};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::components::detail_view::DetailView;
use crate::components::save_file_selector::{
    get_default_save_folder, get_files_in_folder, SaveFileSelector,
};
use crate::load_saveinfo_from_path;
use crate::mod_search::vortex_scanner::Plugin;
use crate::parser::header::Header;
use crate::parser::SaveInfo;
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
    pub characters: HashMap<String, Vec<SaveFile>>,
    pub save_folder_path: String,
    pub selected_character: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub error: Option<String>,
    pub folder_path: String,
    pub detail_state: DetailState,
    pub saves_state: SavesState,
}

#[derive(Clone)]
pub struct SaveFile {
    pub path: String,
    pub file_name: String,
    pub header: Option<Header>,
}

pub fn convert_plugins_to_skui(plugins: &Vec<String>) -> Vec<SkUIValue> {
    let mut skui_plugins = Vec::new();
    for plugin in plugins {
        let new_plugin = SkUIValue::new(plugin.as_str(), plugin.to_string(), UIValueType::Plugin);
        skui_plugins.push(new_plugin);
    }
    skui_plugins
}

fn map_saves_to_characters(saves: &Vec<SaveFile>) -> HashMap<String, Vec<SaveFile>> {
    let mut character_map: HashMap<String, Vec<SaveFile>> = HashMap::new();
    for save in saves {
        let character_name = save.header.as_ref().unwrap().player_name.clone();
        if character_map.contains_key(&character_name) {
            character_map
                .get_mut(&character_name)
                .unwrap()
                .push(save.clone());
        } else {
            character_map.insert(character_name, vec![save.clone()]);
        }
    }
    character_map
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("side-panel").show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);

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
        let saves = get_files_in_folder(folder_path.as_str());
        let characters = map_saves_to_characters(&saves);

        Self {
            folder_path: folder_path,
            error: None,
            detail_state: DetailState {
                file_path: String::from(""),
                save_info: None,
                plugins: None,
                mod_map: HashMap::new(),
                installed: HashSet::new(),
            },
            saves_state: SavesState {
                save_file_list: get_files_in_folder(get_default_save_folder().as_str()),
                characters,
                save_folder_path: get_default_save_folder(),
                selected_character: None,
            },
        }
    }
}
