use eframe::egui::{self, Ui, Color32};

use crate::{parser::SaveInfo, app::convert_plugins_to_skui, sktypes::{types::SkTypeReadable, self}};

fn label_line(ui: &mut Ui, name: &str, value: &str) {
    ui.label(name);
    ui.label(value);
    ui.end_row();
}


//set column width const
const COL_WIDTH: f32 = 400.0;

pub struct DetailView<'a> {
    file_path: &'a str,
    save_info: &'a Option<SaveInfo>,
}

impl <'a> DetailView <'a> {
    pub fn new(file_path: &'a str, save_info: &'a Option<SaveInfo>) -> DetailView<'a> {
        DetailView {
            file_path,
            save_info
        }
    }

    pub fn show(&mut self,ctx: &egui::CtxRef , _ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("top-panel").show(ctx, |ui| {
            ui.heading("Selected Save File");
            ui.label("File path:");
            ui.label(self.file_path);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("values")
            .striped(true)
            .min_row_height(22.)
            .min_col_width(COL_WIDTH)
            .show(ui, |ui| {
                if let Some(info) = self.save_info {
                    if info.header.is_se {
                        label_line(ui, "Game Type", "Skyrim Special Edition");
                    } else {
                        label_line(ui, "Game Type", "Skyrim");
                    }

                    label_line(
                        ui,
                        "Save Number",
                        info.header.save_number.to_string().as_str(),
                    );

                    label_line(ui, "Character Name", info.header.player_name.as_str());
                    label_line(
                        ui,
                        "Character Level",
                        info.header.player_level.to_string().as_str(),
                    );
                    label_line(
                        ui,
                        "Character Sex",
                        info.header.player_sex.to_string().as_str(),
                    );
                    label_line(
                        ui,
                        "Character Race",
                        info.header.player_race_editor_id.as_str(),
                    );
                    label_line(ui, "In Game Date", info.header.game_date.as_str());
                    label_line(ui, "Player Location", info.header.player_location.as_str());
                }
            });

            if let Some(si) = self.save_info {
                if si.plugin_info.plugin_count > 0 {
                    ui.separator();
                    ui.heading("Plugins");
                    ui.separator();
                }
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("values")
                .striped(true)
                .min_row_height(22.)
                .min_col_width(COL_WIDTH)
                .max_col_width(COL_WIDTH)
                .show(ui, |ui| {
                    if let Some(si) = self.save_info {
                        for value_entry in convert_plugins_to_skui(si.plugin_info.plugins.clone()) {
                            ui.label(value_entry.get_name());
                            match value_entry.plugin_type {
                                sktypes::skui_value::PluginType::Native => {
                                    ui.label("Original Game File/DLC");
                                }
                                sktypes::skui_value::PluginType::CreationClub => {
                                    ui.label("Creation Club Mod");
                                }
                                sktypes::skui_value::PluginType::Mod => {
                                    let key = &value_entry.get_value_string();
                                    // if self.installed.contains(key) {
                                    //     ui.colored_label(
                                    //         Color32::from_rgb(50, 200, 50),
                                    //         "Installed",
                                    //     );
                                    // } else if self.mod_map.contains_key(key) {
                                    //     let value = self.mod_map.get(key).unwrap();

                                    //     egui::ScrollArea::vertical().show(ui, |ui| {
                                    //         for l in value.urls.clone() {
                                    //             ui.hyperlink(l.as_str());
                                    //             ui.end_row();
                                    //         }
                                    //     });
                                    // } else {
                                    //     ui.colored_label(
                                    //         Color32::from_rgb(200, 50, 50),
                                    //         "Not Found",
                                    //     );
                                    // }
                                }
                                sktypes::skui_value::PluginType::NotAPlugin => {}
                            }
                            ui.end_row();
                        }
                    }
                });
            });


        });
    }
}