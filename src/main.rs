use std::borrow::Borrow;
/**
 * Author: Divan Visagie
 * https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format#Header
 * https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions
*/
use std::io::Read;
use std::io::Seek;

use eframe::egui;
use eframe::epi;

use crate::sktypes::info_item::InfoItem;
use crate::sktypes::types::SkType;

mod sktypes;

struct AppState {
    file_path: String,
    values: Vec<InfoItem>
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
                    ui.label( "Value");
                    ui.label("Type");
                    ui.end_row();

                    for track in self.values.iter() {
                        ui.label(track.name.as_str());
                        ui.label(track.value.as_str());
                        ui.label(format!("{:?}", track.sk_type));
                        ui.end_row();
                    }
                });
        });
    }

    fn name(&self) -> &str {
        "Skyrim Save Parser"
    }
}

fn read_file(path: String) -> Vec<InfoItem> {
    tracing::info!("Loading file: {:?}", path);
    let mut file = std::fs::File::open(path)
        .map_err(|err| {
            println!("Error {:?}", err);
        })
        .ok().unwrap();
        

    let file = file.by_ref();

    let items = [
        InfoItem::new(file, "magic", SkType::Char13),
        InfoItem::new(file, "header_size", SkType::UInt32),
        // Start Header Section
        InfoItem::new(file, "version", SkType::UInt32),
        InfoItem::new(file, "save_number", SkType::UInt32),
        InfoItem::new(file, "player_name", SkType::WString),
        InfoItem::new(file, "player_level", SkType::UInt32),
        InfoItem::new(file, "player_location", SkType::WString),
        InfoItem::new(file, "game_date", SkType::WString),
        InfoItem::new(file, "player_race_editor_id", SkType::WString),
        InfoItem::new(file, "player_sex", SkType::UInt16), // 0 = male, 1 = female
        InfoItem::new(file, "player_current_experience", SkType::Float32),
        InfoItem::new(file, "player_level_up_exp", SkType::Float32),
        InfoItem::new(file, "file_time", SkType::Unknown), // TODO: temp solution until FILETIME is implemented
        InfoItem::new(file, "shot_width", SkType::UInt32),
        InfoItem::new(file, "shot_height", SkType::UInt32),
        InfoItem::new(file, "compression", SkType::UInt16), //0 = None, 1 = zlib, 2 = lz4
        // End Header Section
        InfoItem::new(file, "screenshot_data", SkType::UInt8),
        InfoItem::new(file, "uncompressed_length", SkType::UInt32),
        InfoItem::new(file, "compressed_length", SkType::UInt32),
        InfoItem::new(file, "form_version", SkType::UInt8),
        InfoItem::new(file, "plugin_info_size", SkType::UInt32),
        // Start Plugin Info Section
        InfoItem::new(file, "pluginCount", SkType::UInt8),
        //    InfoItem::new("plugins", SkType::WString) //TODO: Implement wstring[plugincount]
    ];
    let v = items.to_vec();
    for i in items {
        i.print_value();
        // c.append(i.name.to_string());
    }

    let sp = file.stream_position()
        .map_err(|err| tracing::error!("Error: {:?}", err))
        .ok().unwrap();
    println!("========================\nposition in file: {:?}", sp);

   
    v
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    let app_state = AppState {
        file_path: String::from(""),
        values: Vec::with_capacity(100)
    };
    let mut window_options = eframe::NativeOptions::default();
    window_options.initial_window_size = Some(egui::Vec2::new(900., 768.));
    eframe::run_native(Box::new(app_state), window_options);

}
