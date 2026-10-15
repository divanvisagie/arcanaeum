//! The plugin table both views share: one row per plugin, a filter in each
//! column's header that stays put while the rows scroll, and a switch for
//! the plugins Plugins.txt decides.

use egui::Color32;
use patina::skin::Hex;
use serde::Deserialize;

use crate::analysis::{Kind, PluginFilter, Row};

/// The `[app.arcanaeum]` skin section: the colours of a plugin's status.
/// Each falls back to the active visuals.
#[derive(Debug, Default, Deserialize)]
pub struct SkinColours {
    /// In Data and switched on.
    installed: Option<Hex>,
    /// In Data but switched off.
    disabled: Option<Hex>,
    /// Not in Data, with a download link from the masterlist.
    available: Option<Hex>,
    /// Not in Data and nowhere to get it.
    missing: Option<Hex>,
    /// Base game and Creation Club plugins, which always load.
    included: Option<Hex>,
}

pub struct Colours {
    pub installed: Color32,
    pub disabled: Color32,
    pub available: Color32,
    pub missing: Color32,
    pub included: Color32,
}

impl SkinColours {
    pub fn resolve(&self, v: &egui::Visuals) -> Colours {
        let or = |h: Option<Hex>, c: Color32| h.map_or(c, |h| h.0);
        Colours {
            installed: or(self.installed, v.text_color()),
            disabled: or(self.disabled, v.warn_fg_color),
            available: or(self.available, v.hyperlink_color),
            missing: or(self.missing, v.error_fg_color),
            included: or(self.included, v.weak_text_color()),
        }
    }
}

impl Colours {
    pub fn of(&self, kind: Kind) -> Color32 {
        match kind {
            Kind::BaseGame | Kind::CreationClub => self.included,
            Kind::Active => self.installed,
            Kind::Disabled => self.disabled,
            Kind::NotInstalled => self.available,
            Kind::NotFound => self.missing,
        }
    }
}

/// Draws `rows` that pass `filter`, and returns the switches flipped this
/// frame as (plugin, on).
pub fn plugin_table(
    ui: &mut egui::Ui,
    id: &str,
    rows: &[Row],
    filter: &mut PluginFilter,
    colours: &Colours,
) -> Vec<(String, bool)> {
    use egui_extras::{Column, TableBuilder};

    let shown: Vec<&Row> = rows.iter().filter(|row| filter.matches(row)).collect();
    let mut flipped = Vec::new();

    ui.horizontal(|ui| {
        ui.weak(format!("Showing {} of {}", shown.len(), rows.len()));
        if filter.is_active() && ui.small_button("Clear Filters").clicked() {
            *filter = PluginFilter::default();
        }
    });

    let row_height = ui.spacing().interact_size.y + 4.0;
    TableBuilder::new(ui)
        .id_salt(id)
        .striped(true)
        .resizable(true)
        .auto_shrink(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(48.0))
        .column(Column::exact(28.0))
        .column(Column::initial(320.0).at_least(120.0).clip(true))
        .column(Column::remainder().at_least(160.0).clip(true))
        .header(row_height + 4.0, |mut header| {
            header.col(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut filter.index)
                        .hint_text("#")
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text("Filter by index (hex)");
            });
            header.col(|ui| {
                ui.weak("On").on_hover_text("Whether Plugins.txt loads it");
            });
            header.col(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut filter.name)
                        .hint_text("Plugin")
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text("Filter by any part of the plugin's name");
            });
            header.col(|ui| status_filter(ui, filter, colours));
        })
        .body(|body| {
            body.rows(row_height, shown.len(), |mut table_row| {
                let row = shown[table_row.index()];
                table_row.col(|ui| {
                    ui.weak(row.index_text());
                });
                table_row.col(|ui| {
                    if let Some(on) = row.toggle {
                        let mut on_now = on;
                        let hint = if row.pending {
                            "Changed, not applied yet"
                        } else {
                            "Switch on or off in Plugins.txt"
                        };
                        if ui.checkbox(&mut on_now, "").on_hover_text(hint).changed() {
                            flipped.push((row.name.clone(), on_now));
                        }
                    }
                });
                table_row.col(|ui| {
                    let name = egui::RichText::new(&row.name);
                    let name = if row.pending { name.italics() } else { name };
                    ui.add(egui::Label::new(name).truncate());
                });
                table_row.col(|ui| {
                    ui.colored_label(colours.of(row.kind), row.kind.label());
                    if row.pending {
                        ui.weak("· not applied");
                    }
                    for url in &row.links {
                        ui.hyperlink(url);
                    }
                });
            });
        });
    flipped
}

/// The Status column's filter: a checkbox per status, in a menu that stays
/// open while they're ticked on and off.
fn status_filter(ui: &mut egui::Ui, filter: &mut PluginFilter, colours: &Colours) {
    let label = match filter.hidden.len() {
        0 => "Status: All".to_string(),
        n => format!("Status: {} of {}", Kind::ALL.len() - n, Kind::ALL.len()),
    };
    let config = egui::containers::menu::MenuConfig::new()
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    egui::containers::menu::MenuButton::new(label)
        .config(config)
        .ui(ui, |ui| {
            for kind in Kind::ALL {
                let mut shown = !filter.hidden.contains(&kind);
                let text = egui::RichText::new(kind.label()).color(colours.of(kind));
                if ui.checkbox(&mut shown, text).changed() {
                    if shown {
                        filter.hidden.remove(&kind);
                    } else {
                        filter.hidden.insert(kind);
                    }
                }
            }
            ui.separator();
            if ui.button("Show All").clicked() {
                filter.hidden.clear();
                ui.close();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_colours_come_from_the_skin_or_the_visuals() {
        let skin = patina::skin::Skin::parse(
            "format = 1\nname = \"Test\"\nbase = \"dark\"\n[app.arcanaeum]\nmissing = \"#ff0000\"\n",
        )
        .unwrap();
        let (colours, warnings) = skin.app_section::<SkinColours>("arcanaeum").unwrap();
        assert!(warnings.is_empty());
        let v = egui::Visuals::dark();
        let resolved = colours.resolve(&v);
        assert_eq!(resolved.of(Kind::NotFound), Color32::from_rgb(255, 0, 0));
        assert_eq!(resolved.of(Kind::Disabled), v.warn_fg_color);
        assert_eq!(resolved.of(Kind::NotInstalled), v.hyperlink_color);
    }
}
