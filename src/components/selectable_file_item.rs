use eframe::{egui::{self, Align2, Sense}, epaint::FontId};

// Type Representing a selectable item for display in a selectable item list
pub struct SelectableItem<T> {
    pub title: String,
    pub description: String,
    pub value: T,
}

pub struct SelectableItemList<'a, T> {
    pub items: &'a Vec<SelectableItem<T>>,
    pub selected: Option<T>,
}

impl <'a, T>SelectableItemList<'a, T> where T: Clone {
    pub fn new(items: &Vec<SelectableItem<T>>) -> SelectableItemList<T> {
        SelectableItemList {
            items,
            selected: None,
        }
    }
    
    pub fn show(&mut self, ui: &mut egui::Ui, cb: impl FnOnce(T)) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("values")
            .striped(true)
            .min_row_height(22.)
            .max_col_width(400.0)
            .show(ui, |ui| {
                for si in self.items {
                    if create_clickable_row(ui, si.title.clone(), 44.) {
                        tracing::info!("Selected: {}", si.title);
                        self.selected = Some(si.value.clone());
                    }
                    ui.end_row();
                }
            });
        });

        if let Some(selected) = self.selected.clone() {
            cb(selected);
        }
    }

}

fn create_clickable_row(ui: &mut egui::Ui, value_entry: String, row_height: f32) -> bool {
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

