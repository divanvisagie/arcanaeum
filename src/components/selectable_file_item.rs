use eframe::egui;

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
                    ui.label(value_entry);
                    if ui.button("Select").clicked() {
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