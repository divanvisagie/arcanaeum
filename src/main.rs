//! Arcanaeum: reads your Skyrim saves to find which mods each one needs,
//! and manages the game's load order. Built on egui and Patina.

// A release build on Windows opens no console window beside the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis;
mod app;
mod config;
mod features;
mod game;
mod loader;
mod mod_search;
mod table;

/// The app id, which Wayland matches to the installed launcher for the icon.
const APP_ID: &str = "com.github.divanvisagie.Arcanaeum";

/// The window icon, rendered from `packaging/linux/arcanaeum.svg` by `make
/// icons`. On Wayland, GNOME shows the installed launcher's icon instead.
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../assets/arcanaeum-icon-128.png");

fn window_icon() -> egui::IconData {
    let icon = image::load_from_memory(WINDOW_ICON_PNG)
        .expect("embedded window icon should decode")
        .to_rgba8();
    let (width, height) = icon.dimensions();
    egui::IconData { rgba: icon.into_raw(), width, height }
}

fn main() -> eframe::Result {
    tracing_subscriber::fmt::init();
    let config = config::Config::load();
    let options = eframe::NativeOptions {
        viewport: patina::frame::viewport(
            egui::ViewportBuilder::default()
                .with_title("Arcanaeum")
                .with_app_id(APP_ID)
                .with_icon(window_icon())
                .with_inner_size([1280.0, 768.0])
                .with_min_inner_size([720.0, 420.0]),
            !config.system_title_bar,
        ),
        ..Default::default()
    };
    eframe::run_native("arcanaeum", options, Box::new(|cc| Ok(Box::new(app::App::new(cc, config)))))
}

#[cfg(test)]
mod tests {
    #[test]
    fn window_icon_decodes_at_its_size() {
        let icon = super::window_icon();
        assert_eq!((icon.width, icon.height), (128, 128));
    }
}
