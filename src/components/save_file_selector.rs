use std::{collections::HashMap, io::Read};

use crate::{
    app::{Character, SaveFile, SavesState},
    save_file_parser::parse_header_only,
};
use dirs;
use eframe::{egui, emath::Align};
use std::io::Error;

use super::selectable_item_list::{SelectableItem, SelectableItemList};

pub struct SaveFileSelector<'a> {
    state: &'a mut SavesState,
}

pub fn get_default_save_folder() -> String {
    let mut path = dirs::document_dir().unwrap();
    path.push("My Games");
    path.push("Skyrim Special Edition");
    path.push("Saves");
    path.to_str().unwrap().to_string()
}

fn load_file_buffer(path: &str) -> Result<Vec<u8>, Error> {
    let mut file = std::fs::File::open(path)?;

    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

// Get all .ess files in the target folder and return them as a vector of SaveFile
pub fn read_folder_contents(path: &str) -> Vec<SaveFile> {
    let mut files = Vec::new();
    match std::fs::read_dir(path) {
        Ok(x) => {
            for entry in x {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_file() && path.extension().unwrap() == "ess" {
                    match load_file_buffer(path.to_str().unwrap()) {
                        Ok(buf) => {
                            let header = parse_header_only(buf);
                            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

                            let save_file = SaveFile {
                                path: path.to_str().unwrap().to_string(),
                                header: Some(header),
                                file_name: file_name,
                            };
                            files.push(save_file);
                        }
                        Err(e) => {
                            tracing::error!("Error loading file: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Error reading folder: {}", e);
            return files;
        }
    }
    files
}

fn save_file_to_selectable_item(save_file: SaveFile) -> SelectableItem<SaveFile> {
    let title = save_file.file_name.clone();
    let description = save_file.header.as_ref().unwrap().player_name.clone();
    SelectableItem {
        title,
        description,
        value: save_file,
    }
}

fn saves_for_selected_char(
    selected_char: &Option<String>,
    character_map: &HashMap<String, Character>,
) -> Vec<SaveFile> {
    match selected_char {
        Some(char_name) => {
            let character = character_map.get(char_name).unwrap();
            character.saves.clone()
        }
        None => Vec::new(),
    }
}

impl<'a> SaveFileSelector<'a> {
    pub fn new(state: &'a mut SavesState) -> SaveFileSelector {
        SaveFileSelector { state }
    }

    fn handle_folder_select(&mut self) {
        tracing::info!("Select folder clicked");
        let default_dir = get_default_save_folder();
        let res = rfd::FileDialog::new()
            .set_directory(default_dir)
            .pick_folder();

        match res {
            Some(path_buf) => {
                self.state.save_folder_path = String::from(path_buf.to_str().unwrap());

                tracing::info!("Selected folder: {}", self.state.save_folder_path);
            }
            None => tracing::error!("No folder selected"),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, save_file_selected: impl FnOnce(SaveFile)) {
        if ui.button("Select Skyrim save folder").clicked() {
            self.handle_folder_select();
        }
        ui.separator();

        ui.horizontal_top(|ui| {
            let character_list = self
                .state
                .characters
                .iter()
                .map(|(name, file)| SelectableItem {
                    title: name.to_string(),
                    description: "".to_string(),
                    value: file.clone(),
                })
                .collect::<Vec<SelectableItem<_>>>();
            ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                ui.set_max_width(200.);

                ui.heading("Characters");
                ui.separator();
                SelectableItemList::<Character>::new("character_list", &character_list)
                    .width(200.)
                    .show(ui, |item| {
                        let charname = item.name;
                        self.state.selected_character = Some(charname);
                    });
            });
            ui.separator();

            let s = saves_for_selected_char(&self.state.selected_character, &self.state.characters);

            if let Some(char_name) = &self.state.selected_character {
                let save_file_list = s
                    .clone()
                    .into_iter()
                    .map(|f| save_file_to_selectable_item(f))
                    .collect::<Vec<SelectableItem<_>>>();

                ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                    ui.set_max_width(250.);
                    ui.heading(format!("Saves for {}", char_name));
                    ui.separator();
                    SelectableItemList::<SaveFile>::new("save_file_list", &save_file_list)
                        .width(250.)
                        .show(ui, |item| {
                            tracing::info!("Item in CharSel: {}", item.file_name);
                            save_file_selected(item.clone());
                        });
                });
            }
        });
    }
}
