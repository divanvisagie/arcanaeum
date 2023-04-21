use eframe::egui;


pub struct DetailView<'a> {
    file_path: &'a str,
}

impl <'a> DetailView <'a> {
    pub fn new(file_path: &str) -> DetailView {
        DetailView {
            file_path,
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
            .max_col_width(400.0)
            .show(ui, |ui| {
                ui.label("Name");
            });
        });
    }
}