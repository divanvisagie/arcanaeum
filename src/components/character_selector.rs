use eframe::egui;
use dirs;
use super::selectable_file_item::SelectableItemList;
use std::sync::Mutex;

pub struct SaveFileSelector {
    pub save_folder_path: String,
}

fn get_default_save_folder() -> String {
    let mut path = dirs::document_dir().unwrap();
    path.push("My Games");
    path.push("Skyrim Special Edition");
    path.push("Saves");
    path.to_str().unwrap().to_string()
}

static  SAVE_FILES: Mutex<Vec<String>> = Mutex::new(Vec::new());

impl SaveFileSelector {
    pub fn new() -> SaveFileSelector {
        SaveFileSelector {
            save_folder_path: String::from(""),
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

                // List files in folder_path
                for entry in std::fs::read_dir(&self.save_folder_path).unwrap() {
                    let entry = entry.unwrap();
                    let path = entry.path();
                    if path.is_file() {
                        //SAVE_FILES.push(path.to_str().unwrap().to_string());
                        SAVE_FILES.lock().unwrap().push(path.to_str().unwrap().to_string());
                    }
                }
            }
            None => tracing::error!("No folder selected"),
        }
    }

    fn get_save_files(&self) -> Vec<String> {
        let save_files = SAVE_FILES.lock().unwrap();
        save_files.clone()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, cb: impl FnOnce(&str)) {
        ui.heading("Save Files");
        ui.separator();

        if ui.button("Select Folder").clicked() {
            tracing::info!("Select folder clicked");
            self.handle_folder_select()
        }

        SelectableItemList::new(&self.get_save_files()).show(ui, | item| {
            tracing::info!("Item: {}", item);
        });
    }
}