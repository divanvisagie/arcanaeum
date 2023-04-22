use eframe::{egui::{self, Align2, Sense}, epaint::FontId};

pub struct SelectableItemList<'a> {
    pub items: &'a Vec<String>,
    pub selected: Option<String>,
}

impl SelectableItemList<'_> {
    pub fn new(items: &Vec<String>) -> SelectableItemList {
        SelectableItemList {
            items,
            selected: None,
        }
    }
    
    pub fn show(&mut self, ui: &mut egui::Ui, cb: impl FnOnce(&str)) {
        let current = self.selected.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("values")
            .striped(true)
            .min_row_height(22.)
            .max_col_width(400.0)
            .show(ui, |ui| {
                for value_entry in self.items {
                    if create_clickable_row(ui, value_entry, 44.) {
                        tracing::info!("Selected: {}", value_entry);
                        self.selected = Some(value_entry.to_string());
                    }
                    ui.end_row();
                }
            });
        });

        if current != self.selected {
            cb(&self.selected.as_ref().unwrap());
        }
    }

}

fn create_clickable_row(ui: &mut egui::Ui, value_entry: &str, row_height: f32) -> bool {
    let available_width = ui.available_size().x;
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::new(available_width, row_height), Sense::click());
    let is_hovered = response.hovered();
    let is_clicked = response.clicked();


    // Draw background if hovered
    if is_hovered {
        ui.painter().rect_filled(rect, 2.0, egui::Color32::from_gray(220));
    }

    let text_color = ui.style().visuals.text_color();

    let font_id = FontId::default();

    // Draw row content
    ui.painter().text(
        egui::Pos2::new(rect.min.x + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        value_entry,
        font_id,
        if is_hovered {
            egui::Color32::from_rgb(0, 0, 0)
        } else {
            text_color
        },
    );

    // Draw border
    if is_hovered {
        ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_gray(180)));
    }

    is_clicked
}

