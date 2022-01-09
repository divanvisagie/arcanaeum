/**
 * Author: Divan Visagie
 * https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format
 * https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions
*/
use std::io::Read;

use eframe::egui;
use eframe::epi;
use skyrim_savegame::header::PlayerSex;
use skyrim_savegame::parse_save_file;

use crate::sktypes::skchar13::SkChar13;
use crate::sktypes::skuint32::SkUint32;
use crate::sktypes::types::SkTypeReadable;
use crate::sktypes::wstring::SkWstring;

mod sktypes;
mod mod_search;

struct AppState {
    file_path: String,
    values: Vec<Box<dyn SkTypeReadable>>,
}

impl epi::App for AppState {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
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

            egui::ScrollArea::vertical().show(ui, |ui| {
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

    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf).expect("Could not read file!");
    let parsed = parse_save_file(buf.to_vec());

    tracing::info!("{:?}", parsed.plugin_info);

    let mut items: Vec<Box<dyn SkTypeReadable>> = Vec::new();
    // // Start Header Section
    items.push(Box::new(SkChar13::new("File Type", parsed.magic)));
    items.push(Box::new(SkUint32::new(
        "Save Number",
        parsed.header.save_number,
    )));
    items.push(Box::new(SkWstring::new(
        "Character Name",
        parsed.header.player_name,
    )));
    items.push(Box::new(SkUint32::new(
        "Character Level",
        parsed.header.player_level,
    )));
    items.push(Box::new(SkWstring::new(
        "Current Location",
        parsed.header.player_location,
    )));

    items.push(Box::new(SkWstring::new(
        "In-game date",
        parsed.header.game_date,
    )));
    items.push(Box::new(SkWstring::new(
        "Character Race",
        parsed.header.player_race_editor_id,
    )));

    match parsed.header.player_sex {
        PlayerSex::Male => items.push(Box::new(SkWstring::new("Character Sex", "Male".to_string()))),
        PlayerSex::Female => items.push(Box::new(SkWstring::new("Character Sex", "Female".to_string()))),
    }
    // // End Header Section
    items.push(Box::new(SkWstring::new("Plugins",  "".to_string())));

    for plugin in parsed.plugin_info {
        items.push(Box::new(SkWstring::new("Plugin",  plugin)));
    }
    // items.push(Box::new(SkUint8::from_file(file, "screenshot_data")));
    // items.push(Box::new(SkUint32::from_file(file, "uncompressed_length")));
    // items.push(Box::new(SkUint32::from_file(file, "compressed_length")));

    // let form_version = SkUint8::from_file(file, "form_version");
    // items.push(Box::new(form_version.clone()));

    // let plugin_info_size = SkUint32::from_file(file, "plugin_info_size");
    // items.push(Box::new(plugin_info_size.clone()));

    // // Start Plugin Info Section
    // let plugin_count = SkUint8::from_file(file, "plugin_count");
    // items.push(Box::new(plugin_count.clone()));

    // for n in 1..plugin_count.get_value() {
    //     tracing::info!("getting wstring for {:?}", n);
    //     items.push(Box::new(SkWstring::from_file(file, "unnamed")));
    // }

    // let size = plugin_count.get_value();
    // let plugin_info_size_value = plugin_info_size.get_value();
    // items.push(Box::new(PluginInfo::from_file(file, "plugins", size.into(), plugin_info_size_value)));

    // if form_version.get_value() >= 78 {
    //     // Only for SE save games (and formVersion >= 78?). This contains info about ESL plugins.
    //     tracing::info!("Special Edition, should contain Light Plugin Info");
    // }

    // items.push(Box::new(SkUnknown::from_file(file, "form_id_array_count_offset", 18000000)));

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

    // let sp = file
    //     .stream_position()
    //     .map_err(|err| tracing::error!("Error: {:?}", err))
    //     .ok()
    //     .unwrap();
    // println!("========================\nposition in file: {:?}", sp);

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
