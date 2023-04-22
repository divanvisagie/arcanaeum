use eframe::egui;
use dirs;
use super::selectable_file_item::SelectableItemList;

pub struct SaveFileSelector<'a> {
    pub save_folder_path: String,
    save_files: &'a mut Vec<String>,
}

pub fn get_default_save_folder() -> String {
    let mut path = dirs::document_dir().unwrap();
    path.push("My Games");
    path.push("Skyrim Special Edition");
    path.push("Saves");
    path.to_str().unwrap().to_string()
}

pub fn get_files_in_folder(path: &str) -> Vec<String> {
    let mut files = Vec::new();
    // List files in folder_path
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() && path.extension().unwrap() == "ess" {
            files.push(path.to_str().unwrap().to_string());
        }
    }
    files
}

impl <'a> SaveFileSelector<'a> {
    pub fn new(save_files: &'a mut Vec<String>) -> SaveFileSelector {
        SaveFileSelector {
            save_folder_path: String::from(""),
            save_files: save_files
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

    fn get_save_files(&self) -> Vec<String> {
        self.save_files.clone()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, save_file_selected: impl FnOnce(&str)) {
        ui.heading("Save Files");
        if ui.button("Select Skyrim save folder").clicked() {
            tracing::info!("Select folder clicked");
            self.handle_folder_select();
        }
        ui.separator();


        SelectableItemList::new(&self.get_save_files()).show(ui, | item| {
            tracing::info!("Item in CharSel: {}", item);
            save_file_selected(item);
        });
    }
}