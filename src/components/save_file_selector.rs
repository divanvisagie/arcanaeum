use std::io::Read;

use crate::{
    app::{SaveFile, SavesState},
    parser::parse_header_only,
};
use dirs;
use eframe::egui;
use std::io::Error;

use super::selectable_file_item::{SelectableItem, SelectableItemList};

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
pub fn get_files_in_folder(path: &str) -> Vec<SaveFile> {
    let mut files = Vec::new();

    for entry in std::fs::read_dir(path).unwrap() {
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
    files
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

                // *self.state.save_file_list = get_files_in_folder(&self.state.save_folder_path)
            }
            None => tracing::error!("No folder selected"),
        }
    }

    fn get_save_files(&self) -> Vec<SelectableItem<SaveFile>> {
        self.state
            .save_file_list
            .clone()
            .into_iter()
            .map(|f| SelectableItem {
                title: f.file_name.clone(),
                description: "".to_string(),
                value: f,
            })
            .collect()
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        save_file_selected: impl FnOnce(SaveFile),
    ) {
        ui.vertical(|ui| {
            ui.label("Skyrim Save Editor");
            ui.heading("Save Files");
            if ui.button("Select Skyrim save folder").clicked() {
                tracing::info!("Select folder clicked");
                self.handle_folder_select();
            }
            ui.separator();
        });

        ui.horizontal(|ui| {
            let x = self
                .state
                .characters
                .iter()
                .map(|(name, file)| SelectableItem {
                    title: name.to_string(),
                    description: "".to_string(),
                    value: file.clone(),
                })
                .collect::<Vec<SelectableItem<_>>>();

            SelectableItemList::<Vec<SaveFile>>::new("character_list", &x)
                .width(200.)
                .show(ui, |item| {
                    let charname = item
                        .first()
                        .unwrap()
                        .header
                        .as_ref()
                        .unwrap()
                        .player_name
                        .clone();

                    tracing::info!(
                        "Item in CharSel: {}",
                        item.first().unwrap().header.as_ref().unwrap().player_name
                    );
                    self.state.selected_character = Some(charname);
                });

            if let Some(selected_char) = &self.state.selected_character {
                let x = self
                    .state
                    .characters
                    .get(selected_char)
                    .unwrap()
                    .clone()
                    .into_iter()
                    .map(|f| SelectableItem {
                        title: f.file_name.clone(),
                        description: "".to_string(),
                        value: f,
                    })
                    .collect::<Vec<SelectableItem<_>>>();

                SelectableItemList::<SaveFile>::new("save_file_list", &x).show(ui, |item| {
                    tracing::info!("Item in CharSel: {}", item.file_name);
                    save_file_selected(item.clone());
                });
            }
        });
    }
}
