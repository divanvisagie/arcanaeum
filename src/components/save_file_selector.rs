use std::io::Read;

use eframe::egui;
use dirs;
use crate::{app::SaveFile, parser::parse_header_only};
use std::io::Error;

use super::selectable_file_item::{SelectableItemList, SelectableItem};

pub struct SaveFileSelector<'a> {
    pub save_folder_path: String,
    save_files: &'a mut Vec<SaveFile>,
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

                    let save_file = SaveFile {
                        path: path.to_str().unwrap().to_string(),
                        header: Some(header)
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

impl <'a> SaveFileSelector<'a> {
    pub fn new(save_files: &'a mut Vec<SaveFile>) -> SaveFileSelector {
        SaveFileSelector {
            save_folder_path: String::from(""),
            save_files: save_files,
        }
    }

    fn handle_folder_select(&mut self) {
        tracing::info!("Select folder clicked");
        let default_dir = get_default_save_folder();
        let res = rfd::FileDialog::new()
            .set_directory(default_dir)
            .pick_folder();

        match res {
            Some(path_buf) => {
                self.save_folder_path = String::from(path_buf.to_str().unwrap());

                tracing::info!("Selected folder: {}", self.save_folder_path);

                *self.save_files = get_files_in_folder(&self.save_folder_path)
            }
            None => tracing::error!("No folder selected"),
        }
    }

    fn get_save_files(&self) -> Vec<SelectableItem<SaveFile>> {
        self.save_files.clone().into_iter().map(|f| {
            SelectableItem {
                title: f.path.clone(),
                description: "".to_string(),
                value: f,
            }
        }).collect()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, save_file_selected: impl FnOnce(SaveFile)) {
        ui.heading("Save Files");
        if ui.button("Select Skyrim save folder").clicked() {
            tracing::info!("Select folder clicked");
            self.handle_folder_select();
        }
        ui.separator();

        SelectableItemList::<SaveFile>::new(&self.get_save_files()).show(ui, | item| {
            // tracing::info!("Item in CharSel: {}", item);
            save_file_selected(item);
        });
    }
}