
use eframe::egui::{self};
use eframe::epi;

use crate::components::save_file_selector::SaveFileSelector;
use crate::components::detail_view::{DetailView, DetailViewState};
use crate::sktypes::skui_value::{SkUIValue, UIValueType};
use crate::{load_installed, load_mod_map};
use crate::{load_saveinfo_from_path};

#[derive(Clone)]
pub struct AppState {
    pub error: Option<String>,
    pub folder_path: String,
    pub save_file_list: Vec<String>,
    pub detail_state: DetailViewState,
}

pub fn convert_plugins_to_skui(plugins: &Vec<String>) -> Vec<SkUIValue> {
    let mut skui_plugins = Vec::new();
    for plugin in plugins {
        let new_plugin = SkUIValue::new(plugin.as_str(), plugin.to_string(), UIValueType::Plugin);
        skui_plugins.push(new_plugin);
    }
    skui_plugins
}


impl epi::App for AppState {

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {

        egui::SidePanel::left("side-panel").show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);

            SaveFileSelector::new(&mut self.save_file_list).show(ui, |item| {
                tracing::info!("File was selected: {}", item);
                self.detail_state.file_path = item.to_string();
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

    fn name(&self) -> &str {
        "Arcanaeum"
    }
}
