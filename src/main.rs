use std::collections::HashMap;
/**
 * Author: Divan Visagie
 * https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format#Header
 * https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions
*/
use std::io::Read;
use std::io::Seek;

use eframe::egui;
use eframe::epi;

use crate::sktypes::types::SkChar13;
use crate::sktypes::types::SkFloat32;
use crate::sktypes::types::SkTypeReadable;
use crate::sktypes::types::SkUint16;
use crate::sktypes::types::SkUint32;
use crate::sktypes::types::SkUint8;
use crate::sktypes::types::SkUnknown;
use crate::sktypes::types::SkWstring;

mod sktypes;

struct AppState {
    file_path: String,
    values: Vec<Box<dyn SkTypeReadable>>,
}

impl epi::App for AppState {
    fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Browse to file").clicked() {
                let res = rfd::FileDialog::new()
                    .add_filter("Elder Scrolls Save", &["ess"])
                    .set_directory("./input")
                    .pick_file()
                    .unwrap()
                    .into_os_string();

                self.file_path = String::from(res.to_str().unwrap());
                self.values = read_file(self.file_path.to_string());
            }

            egui::Grid::new("values")
                .striped(true)
                .min_col_width(300.0)
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.label("Value");
                    ui.label("Type");
                    ui.end_row();

                    for value_entry in self.values.iter() {
                        ui.label(value_entry.get_name());
                        ui.label(value_entry.get_value_string());
                        ui.label(value_entry.get_type());
                        ui.end_row();
                    }
                });

            ui.hyperlink("https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format");
        });
    }

    fn name(&self) -> &str {
        "Skyrim Save Parser"
    }
}

fn read_file(path: String) -> Vec<Box<dyn SkTypeReadable>> {
    tracing::info!("Loading file: {:?}", path);
    let mut file = std::fs::File::open(path)
        .map_err(|err| {
            println!("Error {:?}", err);
        })
        .ok()
        .unwrap();

    let file = file.by_ref();

    let mut items: Vec<Box<dyn SkTypeReadable>> = Vec::new();
    items.push(Box::new(SkChar13::from_file(file, "magic")));
    items.push(Box::new(SkUint32::from_file(file, "header_size")));
    // Start Header Section
    items.push(Box::new(SkUint32::from_file(file, "version")));
    items.push(Box::new(SkUint32::from_file(file, "save_number")));
    items.push(Box::new(SkWstring::from_file(file, "player_name")));
    items.push(Box::new(SkUint32::from_file(file, "player_level")));
    items.push(Box::new(SkWstring::from_file(file, "player_location")));
    items.push(Box::new(SkWstring::from_file(file, "game_date")));
    items.push(Box::new(SkWstring::from_file(file, "player_race_editor")));
    items.push(Box::new(SkUint16::from_file(file, "player_sex")));
    items.push(Box::new(SkFloat32::from_file(file, "player_current_experience")));
    items.push(Box::new(SkFloat32::from_file(file, "player_level_up")));
    items.push(Box::new(SkUnknown::from_file(file, "file_time", 8)));
    items.push(Box::new(SkUint32::from_file(file, "shot_width")));
    items.push(Box::new(SkUint32::from_file(file, "shot_height")));
    items.push(Box::new(SkUint16::from_file(file, "compression")));
    // End Header Section

    
    items.push(Box::new(SkUint16::from_file(file, "compression")));
    items.push(Box::new(SkUint8::from_file(file, "screenshot_data")));
    items.push(Box::new(SkUint32::from_file(file, "uncompressed_length")));
    items.push(Box::new(SkUint32::from_file(file, "compressed_length")));
    items.push(Box::new(SkUint8::from_file(file, "form_version")));
    items.push(Box::new(SkUint32::from_file(file, "plugin_info_size")));

    // Start Plugin Info Section
    let plugin_count = SkUint8::from_file(file, "plugin_count");
    items.push(Box::new(plugin_count));
    
    //    InfoItem::new("plugins", SkType::WString) //TODO: Implement wstring[plugincount]
    // let meta_state = HashMap::new();
    // let vector = items.to_vec();
    // for i in items {
    //     i.print_value();
    //     // c.append(i.name.to_string());
    //     if i.name == "plugin_info_size" {
    //         //Do Plugin experiment
    //         InfoItem::new_with_size(file, "plugin1", SkType::Unknown, 100);
    //     }
    // }

    let sp = file
        .stream_position()
        .map_err(|err| tracing::error!("Error: {:?}", err))
        .ok()
        .unwrap();
    println!("========================\nposition in file: {:?}", sp);

    items
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    let app_state = AppState {
        file_path: String::from(""),
        values: Vec::with_capacity(150),
    };
    let mut window_options = eframe::NativeOptions::default();
    window_options.initial_window_size = Some(egui::Vec2::new(900., 768.));
    eframe::run_native(Box::new(app_state), window_options);
}
