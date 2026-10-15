//! The window: Patina's frame and title bar, the characters and their saves
//! in a sidebar, the chosen save's plugins in the middle, and a status bar.

use std::path::PathBuf;

use egui::Color32;
use patina::frame::{self, TitleBar};
use patina::skin::{self, Hex, Skin};
use serde::Deserialize;

use crate::analysis::{Analysis, Kind, PluginFilter, Status};
use crate::config::Config;
use crate::loader::{Characters, Command, Loader, Update};
use crate::save_file_parser::header::Sex;

/// Stock egui until the user picks a skin.
const HOME_SKIN: &str = skin::DEFAULT_SKIN;

/// The open save's screenshot takes this share of the details panel's width,
/// and at most this share of its height so the plugin list keeps the rest.
/// It never grows past its own size in pixels, where it would blur.
const SAVE_IMAGE_SHARE: f32 = 0.4;

/// How long a status bar notice stays up, in seconds.
const NOTICE_FOR: f64 = 3.0;

/// The `[app.arcanaeum]` skin section: the colours of a plugin's status.
/// Each falls back to the active visuals.
#[derive(Debug, Default, Deserialize)]
struct SkinColours {
    /// On disk, ready to load.
    installed: Option<Hex>,
    /// Not on disk, with a download link from the masterlist.
    available: Option<Hex>,
    /// Not on disk and nowhere to get it.
    missing: Option<Hex>,
    /// Base game and Creation Club plugins.
    included: Option<Hex>,
}

struct Colours {
    installed: Color32,
    available: Color32,
    missing: Color32,
    included: Color32,
}

impl SkinColours {
    fn resolve(&self, v: &egui::Visuals) -> Colours {
        let or = |h: Option<Hex>, c: Color32| h.map_or(c, |h| h.0);
        Colours {
            installed: or(self.installed, v.text_color()),
            available: or(self.available, v.warn_fg_color),
            missing: or(self.missing, v.error_fg_color),
            included: or(self.included, v.weak_text_color()),
        }
    }
}

/// What the menus, shortcuts and panels ask for, handled after drawing.
enum Do {
    OpenFolder,
    Reload,
    OpenSave(PathBuf),
    CopyReport,
    SetSkin(String),
    ReloadSkins,
    SystemTitleBar(bool),
    Quit,
}

pub struct App {
    config: Config,
    custom_frame: bool,
    skins: Vec<Skin>,
    skin_errors: Vec<String>,
    /// `ARCANAEUM_SKIN`: this run's skin, not saved.
    forced_skin: Option<String>,
    skin_colours: SkinColours,
    loader: Loader,
    folder: PathBuf,
    characters: Characters,
    unreadable: usize,
    selected: Option<PathBuf>,
    analysis: Option<Analysis>,
    /// The open save's screenshot, shown top right of its details.
    save_image: Option<egui::TextureHandle>,
    /// The plugin list's column filters; kept when another save is opened.
    plugin_filter: PluginFilter,
    error: Option<String>,
    notice: Option<(String, f64)>,
    /// Screenshot mode: where to save, whether it was asked for, and when
    /// the window was ready.
    screenshot: Option<(PathBuf, bool, Option<f64>)>,
}

fn load_skins() -> (Vec<Skin>, Vec<String>) {
    let dirs: Vec<_> = skin::app_dir("arcanaeum").into_iter().collect();
    skin::load_all(skin::BUILT_IN, &dirs)
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
        patina::appearance::watch(&cc.egui_ctx);
        Self::with_context(&cc.egui_ctx, config)
    }

    fn with_context(ctx: &egui::Context, config: Config) -> Self {
        let (skins, skin_errors) = load_skins();
        let folder = std::env::var_os("ARCANAEUM_SAVE_FOLDER")
            .map_or_else(|| config.save_folder(), PathBuf::from);
        let mut app = Self {
            custom_frame: !config.system_title_bar,
            config,
            skins,
            skin_errors,
            forced_skin: std::env::var("ARCANAEUM_SKIN").ok(),
            skin_colours: SkinColours::default(),
            loader: Loader::start(ctx),
            folder,
            characters: Characters::new(),
            unreadable: 0,
            selected: None,
            analysis: None,
            save_image: None,
            plugin_filter: PluginFilter::default(),
            error: None,
            notice: None,
            screenshot: std::env::var_os("ARCANAEUM_SCREENSHOT")
                .map(|p| (PathBuf::from(p), false, None)),
        };
        app.apply_skin(ctx);
        app.loader.send(Command::ScanFolder(app.folder.clone()));
        app
    }

    /// The chosen skin, falling back to `HOME_SKIN` (a saved name may no
    /// longer exist), then Default.
    fn skin(&self) -> &Skin {
        let find = |name: &str| self.skins.iter().find(|s| s.name == name);
        let name = self
            .forced_skin
            .as_deref()
            .or(self.config.skin.as_deref())
            .unwrap_or(HOME_SKIN);
        find(name)
            .or_else(|| find(HOME_SKIN))
            .unwrap_or(&self.skins[0])
    }

    fn apply_skin(&mut self, ctx: &egui::Context) {
        self.skin().apply(ctx);
        let name = self.skin().name.clone();
        self.skin_errors.retain(|e| !e.starts_with("app.arcanaeum"));
        match self.skin().app_section::<SkinColours>("arcanaeum") {
            Ok((colours, warnings)) => {
                self.skin_colours = colours;
                self.skin_errors.extend(
                    warnings
                        .into_iter()
                        .map(|w| format!("app.arcanaeum in {name}: {w}")),
                );
            }
            Err(e) => {
                self.skin_colours = SkinColours::default();
                self.skin_errors
                    .push(format!("app.arcanaeum in {name}: {e}"));
            }
        }
    }

    fn notify(&mut self, ctx: &egui::Context, text: impl Into<String>) {
        self.notice = Some((text.into(), ctx.input(|i| i.time)));
    }

    fn handle_updates(&mut self, ctx: &egui::Context) {
        for update in self.loader.poll() {
            match update {
                Update::Folder {
                    path,
                    characters,
                    unreadable,
                } if path == self.folder => {
                    self.characters = characters;
                    self.unreadable = unreadable;
                    if self.screenshot.is_some() && self.selected.is_none() {
                        // `ARCANAEUM_CHARACTER` names whose newest save to show;
                        // otherwise the first character's.
                        let saves = match std::env::var("ARCANAEUM_CHARACTER") {
                            Ok(name) => {
                                let saves = self.characters.get(&name);
                                if saves.is_none() {
                                    let names: Vec<_> = self.characters.keys().collect();
                                    eprintln!("no character named {name:?}; there are {names:?}");
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                                saves
                            }
                            Err(_) => self.characters.values().next(),
                        };
                        if let Some(save) = saves.and_then(|s| s.first()) {
                            self.selected = Some(save.path.clone());
                            self.loader.send(Command::OpenSave(save.path.clone()));
                        }
                    }
                }
                Update::Folder { .. } => {}
                Update::Save(Ok(analysis)) => {
                    if self.selected.as_deref().map(|p| p.display().to_string())
                        == Some(analysis.path.clone())
                    {
                        self.save_image = analysis.screenshot().map(|image| {
                            ctx.load_texture("save-screenshot", image, egui::TextureOptions::LINEAR)
                        });
                        self.analysis = Some(*analysis);
                    }
                }
                Update::Save(Err(e)) => self.error = Some(e),
            }
        }
    }

    fn shortcuts(&self, ctx: &egui::Context, actions: &mut Vec<Do>) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let pressed = |m, k| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if pressed(Modifiers::COMMAND | Modifiers::SHIFT, Key::C) {
            actions.push(Do::CopyReport);
        }
        if pressed(Modifiers::COMMAND, Key::O) {
            actions.push(Do::OpenFolder);
        }
        if pressed(Modifiers::COMMAND, Key::R) || pressed(Modifiers::NONE, Key::F5) {
            actions.push(Do::Reload);
        }
        if pressed(Modifiers::COMMAND, Key::Q) {
            actions.push(Do::Quit);
        }
    }

    /// Left side of the title bar: word menus (Patina's ADR-0006).
    fn menus(&self, ui: &mut egui::Ui, actions: &mut Vec<Do>) {
        fn item(
            ui: &mut egui::Ui,
            actions: &mut Vec<Do>,
            enabled: bool,
            label: &str,
            keys: &str,
            action: Do,
        ) {
            if ui
                .add_enabled(enabled, egui::Button::new(label).shortcut_text(keys))
                .clicked()
            {
                actions.push(action);
                ui.close();
            }
        }
        ui.menu_button("File", |ui| {
            item(
                ui,
                actions,
                true,
                "Open Save Folder…",
                "Ctrl+O",
                Do::OpenFolder,
            );
            item(ui, actions, true, "Reload", "Ctrl+R", Do::Reload);
            ui.separator();
            item(ui, actions, true, "Quit", "Ctrl+Q", Do::Quit);
        });
        ui.menu_button("Edit", |ui| {
            item(
                ui,
                actions,
                self.analysis.is_some(),
                "Copy Report",
                "Ctrl+Shift+C",
                Do::CopyReport,
            );
        });
        ui.menu_button("View", |ui| {
            let mut system = !self.custom_frame;
            if ui.checkbox(&mut system, "System Title Bar").changed() {
                actions.push(Do::SystemTitleBar(system));
            }
        });
        ui.menu_button("Skins", |ui| {
            let current = self.skin().name.clone();
            for skin in &self.skins {
                if ui.radio(skin.name == current, &skin.name).clicked() {
                    actions.push(Do::SetSkin(skin.name.clone()));
                }
            }
            ui.separator();
            if ui.button("Reload Skins").clicked() {
                actions.push(Do::ReloadSkins);
            }
            for error in &self.skin_errors {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
    }

    /// Right side of the title bar: the app's output, the chosen save's report.
    fn output(&self, ui: &mut egui::Ui, actions: &mut Vec<Do>) {
        let copy = ui
            .add_enabled(self.analysis.is_some(), egui::Button::new("Copy Report"))
            .on_hover_text("Copy the save's plugins and where to get them (Ctrl+Shift+C)");
        if copy.clicked() {
            actions.push(Do::CopyReport);
        }
    }

    fn act(&mut self, ctx: &egui::Context, action: Do) {
        match action {
            Do::OpenFolder => {
                let picked = rfd::FileDialog::new()
                    .set_title("Open Save Folder")
                    .set_directory(&self.folder)
                    .pick_folder();
                if let Some(folder) = picked {
                    self.config.save_folder = Some(folder.clone());
                    self.config.save();
                    self.folder = folder;
                    self.characters.clear();
                    self.selected = None;
                    self.analysis = None;
                    self.save_image = None;
                    self.loader.send(Command::ScanFolder(self.folder.clone()));
                }
            }
            Do::Reload => {
                self.loader.send(Command::ScanFolder(self.folder.clone()));
                if let Some(path) = self.selected.clone() {
                    self.loader.send(Command::OpenSave(path));
                }
            }
            Do::OpenSave(path) => {
                self.selected = Some(path.clone());
                self.loader.send(Command::OpenSave(path));
            }
            Do::CopyReport => {
                if let Some(analysis) = &self.analysis {
                    ctx.copy_text(analysis.report());
                    self.notify(ctx, "Report copied");
                }
            }
            Do::SetSkin(name) => {
                self.forced_skin = None;
                self.config.skin = Some(name);
                self.config.save();
                self.apply_skin(ctx);
            }
            Do::ReloadSkins => {
                (self.skins, self.skin_errors) = load_skins();
                self.apply_skin(ctx);
            }
            Do::SystemTitleBar(on) => {
                self.custom_frame = !on;
                self.config.system_title_bar = on;
                self.config.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(on));
            }
            Do::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
    }

    fn sidebar(&self, ui: &mut egui::Ui, actions: &mut Vec<Do>) {
        ui.add_space(6.0);
        ui.heading("Characters");
        ui.add_space(4.0);
        if self.characters.is_empty() {
            if self.loader.busy() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak("Reading saves…");
                });
            } else {
                ui.weak("No saves in this folder.");
                if ui.button("Open Save Folder…").clicked() {
                    actions.push(Do::OpenFolder);
                }
            }
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let only_one = self.characters.len() == 1;
                for (name, saves) in &self.characters {
                    let holds_selected = saves
                        .iter()
                        .any(|s| Some(&s.path) == self.selected.as_ref());
                    egui::CollapsingHeader::new(format!("{name}  ({})", saves.len()))
                        .id_salt(name)
                        .default_open(only_one || holds_selected)
                        .show(ui, |ui| {
                            for save in saves {
                                let h = &save.header;
                                let selected = Some(&save.path) == self.selected.as_ref();
                                let label = format!("{}  ·  {}", h.save_number, h.player_location);
                                let row = ui
                                    .add(egui::Button::selectable(selected, label).truncate())
                                    .on_hover_text(format!(
                                        "{}\nLevel {}, {}",
                                        save.file_name, h.player_level, h.game_date
                                    ));
                                if row.clicked() && !selected {
                                    actions.push(Do::OpenSave(save.path.clone()));
                                }
                            }
                        });
                }
            });
    }

    fn details(&mut self, ui: &mut egui::Ui) {
        let Some(analysis) = &self.analysis else {
            ui.centered_and_justified(|ui| {
                if self.selected.is_some() && self.loader.busy() {
                    ui.spinner();
                } else {
                    ui.weak("Pick a save on the left to see the mods it needs.");
                }
            });
            return;
        };
        let h = &analysis.save.header;
        let colours = self.skin_colours.resolve(ui.visuals());

        let info = |ui: &mut egui::Ui| {
            ui.add_space(6.0);
            ui.heading(&h.player_name);
            let race = h
                .player_race_editor_id
                .strip_suffix("Race")
                .unwrap_or(&h.player_race_editor_id);
            let sex = match h.player_sex {
                Sex::Male => ", male",
                Sex::Female => ", female",
                Sex::Undefined => "",
            };
            ui.weak(format!(
                "Level {} {race}{sex}  ·  {}",
                h.player_level, h.player_location
            ));
            ui.add_space(6.0);
            egui::Grid::new("details")
                .num_columns(2)
                .spacing([24.0, 4.0])
                .show(ui, |ui| {
                    let line = |ui: &mut egui::Ui, name: &str, value: &str| {
                        ui.weak(name);
                        ui.add(egui::Label::new(value).truncate());
                        ui.end_row();
                    };
                    line(
                        ui,
                        "Game",
                        if h.is_se {
                            "Skyrim Special Edition"
                        } else {
                            "Skyrim"
                        },
                    );
                    line(ui, "Save", &h.save_number.to_string());
                    line(ui, "In-game date", &h.game_date);
                    line(ui, "File", &analysis.path);
                });
        };

        // The save's screenshot sits top right, the details fill the rest.
        match &self.save_image {
            Some(texture) => {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    let [w, h] = texture.size().map(|s| s as f32);
                    let height = (ui.available_width() * SAVE_IMAGE_SHARE * h / w)
                        .min(ui.available_height() * SAVE_IMAGE_SHARE)
                        .min(h / ui.ctx().pixels_per_point());
                    let size = egui::vec2(height * w / h, height);
                    ui.add(
                        egui::Image::new((texture.id(), size))
                            .corner_radius(ui.visuals().widgets.noninteractive.corner_radius),
                    );
                    ui.add_space(12.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), info);
                });
            }
            None => info(ui),
        }

        ui.add_space(8.0);
        ui.separator();
        let (installed, available, missing) = analysis.counts();
        ui.horizontal(|ui| {
            ui.heading("Plugins");
            ui.add_space(8.0);
            ui.colored_label(colours.installed, format!("{installed} installed"));
            ui.weak("·");
            ui.colored_label(colours.available, format!("{available} to download"));
            ui.weak("·");
            ui.colored_label(colours.missing, format!("{missing} not found"));
        });
        ui.add_space(4.0);
        plugin_table(ui, analysis, &mut self.plugin_filter, &colours);
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let now = ui.input(|i| i.time);
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, at)| now - at > NOTICE_FOR)
        {
            self.notice = None;
        }
        ui.horizontal(|ui| {
            if self.loader.busy() {
                ui.spinner();
            }
            if let Some(error) = self.error.clone() {
                ui.colored_label(ui.visuals().error_fg_color, error);
                if ui.small_button("Dismiss").clicked() {
                    self.error = None;
                }
            } else if let Some((notice, at)) = &self.notice {
                ui.label(notice);
                ui.ctx()
                    .request_repaint_after_secs((NOTICE_FOR - (now - at)) as f32);
            } else {
                let saves: usize = self.characters.values().map(Vec::len).sum();
                let mut text = format!("{} characters, {saves} saves", self.characters.len());
                if self.unreadable > 0 {
                    text.push_str(&format!(", {} unreadable", self.unreadable));
                }
                ui.weak(text);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(egui::RichText::new(self.folder.display().to_string()).weak())
                        .truncate(),
                );
            });
        });
    }

    /// Screenshot mode: once the first save is shown, save the window and quit.
    fn take_screenshot(&mut self, ctx: &egui::Context) {
        let ready = self.analysis.is_some() || (!self.loader.busy() && self.characters.is_empty());
        let Some((path, requested, ready_at)) = &mut self.screenshot else {
            return;
        };
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [w, h] = image.size;
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            if let Err(e) =
                image::save_buffer(&*path, &rgba, w as u32, h as u32, image::ColorType::Rgba8)
            {
                eprintln!("screenshot failed: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let now = ctx.input(|i| i.time);
        if ready && ready_at.is_none() {
            *ready_at = Some(now);
        }
        match ready_at {
            Some(at) if !*requested && now - *at > 1.0 => {
                *requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            _ => ctx.request_repaint_after_secs(0.1),
        }
    }
}

/// The open save's plugins, in load order, with a filter in each column's
/// header that stays put while the rows scroll.
fn plugin_table(
    ui: &mut egui::Ui,
    analysis: &Analysis,
    filter: &mut PluginFilter,
    colours: &Colours,
) {
    use egui_extras::{Column, TableBuilder};

    let colour = |kind: Kind| match kind {
        Kind::BaseGame | Kind::CreationClub => colours.included,
        Kind::Installed => colours.installed,
        Kind::NotInstalled => colours.available,
        Kind::NotFound => colours.missing,
    };
    let shown: Vec<_> = analysis
        .plugins
        .iter()
        .enumerate()
        .filter(|(i, row)| filter.matches(*i, row))
        .collect();

    ui.horizontal(|ui| {
        ui.weak(format!(
            "Showing {} of {}",
            shown.len(),
            analysis.plugins.len()
        ));
        if filter.is_active() && ui.small_button("Clear Filters").clicked() {
            *filter = PluginFilter::default();
        }
    });

    let row_height = ui.spacing().interact_size.y + 4.0;
    TableBuilder::new(ui)
        .id_salt("plugins")
        .striped(true)
        .resizable(true)
        .auto_shrink(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(48.0))
        .column(Column::initial(320.0).at_least(120.0).clip(true))
        .column(Column::remainder().at_least(160.0).clip(true))
        .header(row_height + 4.0, |mut header| {
            header.col(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut filter.index)
                        .hint_text("#")
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text("Filter by load order index (hex)");
            });
            header.col(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut filter.name)
                        .hint_text("Plugin")
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text("Filter by any part of the plugin's name");
            });
            header.col(|ui| {
                let hidden = filter.hidden.len();
                let label = match hidden {
                    0 => "Status: All".to_string(),
                    n => format!("Status: {} of {}", Kind::ALL.len() - n, Kind::ALL.len()),
                };
                // Stays open while statuses are ticked on and off.
                let config = egui::containers::menu::MenuConfig::new()
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
                egui::containers::menu::MenuButton::new(label)
                    .config(config)
                    .ui(ui, |ui| {
                        for kind in Kind::ALL {
                            let mut shown = !filter.hidden.contains(&kind);
                            let text = egui::RichText::new(kind.label()).color(colour(kind));
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
            });
        })
        .body(|body| {
            body.rows(row_height, shown.len(), |mut row| {
                let (i, plugin) = shown[row.index()];
                row.col(|ui| {
                    ui.weak(format!("{i:02X}"));
                });
                row.col(|ui| {
                    ui.add(egui::Label::new(&plugin.name).truncate());
                });
                row.col(|ui| {
                    let kind = plugin.kind();
                    ui.colored_label(colour(kind), kind.label());
                    if let Status::Available(urls) = &plugin.status {
                        for url in urls {
                            ui.hyperlink(url);
                        }
                    }
                });
            });
        });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        frame::clear_color(visuals, self.custom_frame)
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        frame::release_grab(ctx, raw_input);
    }
}

impl App {
    fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.handle_updates(&ctx);
        // Every frame: Default's colours follow the desktop's light/dark setting.
        let style = self.skin().frame_style(ui.visuals());
        let radius = if self.custom_frame {
            frame::corner_radius(&ctx, style.radius)
        } else {
            0
        };

        let mut actions = Vec::new();
        self.shortcuts(&ctx, &mut actions);
        let mut output = Vec::new();
        egui::Panel::top("header")
            .frame(frame::header_frame(ui.style(), &style, radius))
            .show(ui, |ui| {
                TitleBar::new("Arcanaeum", &style)
                    .custom(self.custom_frame)
                    .menu_bar(true)
                    .show(
                        ui,
                        |ui| self.menus(ui, &mut actions),
                        |ui| self.output(ui, &mut output),
                    );
            });
        actions.append(&mut output);

        // The status bar holds both bottom corners.
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::side_top_panel(ui.style()).corner_radius(egui::CornerRadius {
                    sw: radius,
                    se: radius,
                    ..Default::default()
                }),
            )
            .show(ui, |ui| self.status_bar(ui));

        egui::Panel::left("characters")
            .default_size(280.0)
            .resizable(true)
            .show(ui, |ui| self.sidebar(ui, &mut actions));

        egui::CentralPanel::default().show(ui, |ui| self.details(ui));

        for action in actions {
            self.act(&ctx, action);
        }
        if self.custom_frame {
            frame::edges(&ctx, radius, style.border);
        }
        self.take_screenshot(&ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_home_skin_exists() {
        let (skins, _) = skin::load_all(skin::BUILT_IN, &[]);
        assert!(skins.iter().any(|s| s.name == HOME_SKIN));
    }

    #[test]
    fn status_colours_come_from_the_skin_or_the_visuals() {
        let skin = Skin::parse(
            "format = 1\nname = \"Test\"\nbase = \"dark\"\n[app.arcanaeum]\nmissing = \"#ff0000\"\n",
        )
        .unwrap();
        let (colours, warnings) = skin.app_section::<SkinColours>("arcanaeum").unwrap();
        assert!(warnings.is_empty());
        let v = egui::Visuals::dark();
        let resolved = colours.resolve(&v);
        assert_eq!(resolved.missing, Color32::from_rgb(255, 0, 0));
        assert_eq!(resolved.available, v.warn_fg_color);
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;

    fn texts(shape: &egui::Shape, out: &mut String) {
        match shape {
            egui::Shape::Text(t) => {
                out.push_str(t.galley.text());
                out.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| texts(s, out)),
            _ => {}
        }
    }

    /// Draws a frame of the window with `events` as its input.
    fn frame_with(
        app: &mut App,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 768.0),
            )),
            events,
            ..Default::default()
        };
        ctx.run_ui(input, |ui| app.draw(ui))
    }

    /// Where exactly the text `text` is drawn, on top if it's drawn more
    /// than once (a menu over the table).
    fn text_rect(output: &egui::FullOutput, text: &str) -> Option<egui::Rect> {
        fn find(shape: &egui::Shape, text: &str) -> Option<egui::Rect> {
            match shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(t.galley.rect.translate(t.pos.to_vec2()))
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, text)),
                _ => None,
            }
        }
        output.shapes.iter().rev().find_map(|s| find(&s.shape, text))
    }

    /// A click at `at`: move there, press, release, a frame each.
    fn click(app: &mut App, ctx: &egui::Context, at: egui::Pos2) {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let _ = frame_with(app, ctx, vec![egui::Event::PointerMoved(at)]);
        let _ = frame_with(app, ctx, vec![button(true)]);
        let _ = frame_with(app, ctx, vec![button(false)]);
    }

    /// Draws a frame of the window and returns all the text on screen.
    fn frame(app: &mut App, ctx: &egui::Context) -> String {
        let output = frame_with(app, ctx, Vec::new());
        let mut out = String::new();
        output.shapes.iter().for_each(|s| texts(&s.shape, &mut out));
        out
    }

    fn settle(app: &mut App, ctx: &egui::Context) -> String {
        for _ in 0..600 {
            frame(app, ctx);
            if !app.loader.busy() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        frame(app, ctx)
    }

    #[test]
    fn an_empty_folder_says_so_under_word_menus() {
        let dir = std::env::temp_dir().join("arcanaeum_empty_folder_test");
        std::fs::create_dir_all(&dir).unwrap();
        let ctx = egui::Context::default();
        let config = Config {
            save_folder: Some(dir),
            ..Default::default()
        };
        let mut app = App::with_context(&ctx, config);
        let screen = settle(&mut app, &ctx);
        for text in [
            "File",
            "Edit",
            "View",
            "Skins",
            "Arcanaeum",
            "Copy Report",
            "No saves in this folder.",
        ] {
            assert!(screen.contains(text), "{text:?} not on screen:\n{screen}");
        }
    }

    /// Opens the newest save in `ARCANAEUM_TEST_SAVES` (a real save folder,
    /// only read): `ARCANAEUM_TEST_SAVES=… cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn a_real_save_shows_its_plugins() {
        let dir = PathBuf::from(
            std::env::var_os("ARCANAEUM_TEST_SAVES").expect("set ARCANAEUM_TEST_SAVES"),
        );
        let ctx = egui::Context::default();
        let mut app = App::with_context(
            &ctx,
            Config {
                save_folder: Some(dir),
                ..Default::default()
            },
        );
        settle(&mut app, &ctx);
        let save = app
            .characters
            .values()
            .next()
            .and_then(|s| s.first())
            .expect("no saves")
            .path
            .clone();
        app.act(&ctx, Do::OpenSave(save));
        let screen = settle(&mut app, &ctx);
        let analysis = app.analysis.as_ref().expect("save didn't open");
        assert!(screen.contains("Plugins"), "{screen}");
        assert!(screen.contains(&analysis.save.header.player_name));

        let texture = app.save_image.as_ref().expect("no save screenshot");
        let image = analysis.screenshot().unwrap();
        let lit = image
            .pixels
            .iter()
            .filter(|c| c.r() > 16 || c.g() > 16 || c.b() > 16)
            .count();
        assert!(lit > image.pixels.len() / 4, "screenshot is mostly black");
        let id = texture.id();
        let mut drawn_at = |width: f32, height: f32| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, height),
                )),
                ..Default::default()
            };
            // Twice: panels settle their sizes on the second frame.
            let _ = ctx.run_ui(input.clone(), |ui| app.draw(ui));
            let output = ctx.run_ui(input, |ui| app.draw(ui));
            output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Mesh(m) if m.texture_id == id => Some(m.calc_bounds()),
                    egui::Shape::Rect(r) if r.fill_texture_id() == id => Some(r.rect),
                    _ => None,
                })
                .expect("screenshot not drawn")
        };
        let small = drawn_at(900.0, 600.0);
        let large = drawn_at(2560.0, 1440.0);
        eprintln!(
            "screenshot {:?} drawn at {small:?}, then {large:?}",
            image.size
        );
        assert!(
            large.max.x > 2500.0 && large.min.y < 120.0,
            "not top right: {large:?}"
        );
        assert!(
            small.width() < large.width(),
            "didn't shrink with the window"
        );
        assert_eq!(
            large.size(),
            egui::vec2(image.size[0] as f32, image.size[1] as f32),
            "not capped at its own size"
        );
        eprintln!("{screen}");
    }

    /// Filters the plugins of the newest save in `ARCANAEUM_TEST_SAVES` by
    /// clicking and typing, as a user would.
    #[test]
    #[ignore]
    fn the_plugin_columns_filter() {
        let dir = PathBuf::from(
            std::env::var_os("ARCANAEUM_TEST_SAVES").expect("set ARCANAEUM_TEST_SAVES"),
        );
        let ctx = egui::Context::default();
        let config = Config {
            save_folder: Some(dir),
            ..Default::default()
        };
        let mut app = App::with_context(&ctx, config);
        settle(&mut app, &ctx);
        let save = app.characters.values().next().unwrap()[0].path.clone();
        app.act(&ctx, Do::OpenSave(save));
        let screen = settle(&mut app, &ctx);
        let total = app.analysis.as_ref().unwrap().plugins.len();
        assert!(screen.contains("Skyrim.esm"), "{screen}");
        assert!(screen.contains(&format!("Showing {total} of {total}")));

        // Click into the Plugin column's filter and type.
        let output = frame_with(&mut app, &ctx, Vec::new());
        let field = text_rect(&output, "Plugin").expect("no Plugin filter");
        click(&mut app, &ctx, field.center());
        let _ = frame_with(&mut app, &ctx, vec![egui::Event::Text("FISH".into())]);
        let screen = frame(&mut app, &ctx);
        assert_eq!(app.plugin_filter.name, "FISH");
        assert!(!screen.contains("Skyrim.esm"), "{screen}");
        let fish = app
            .analysis
            .as_ref()
            .unwrap()
            .plugins
            .iter()
            .filter(|p| p.name.to_lowercase().contains("fish"))
            .count();
        assert!(fish > 0, "this save has no fish plugin to filter for");
        assert!(
            screen.contains(&format!("Showing {fish} of {total}")),
            "{screen}"
        );

        // Untick Creation Club in the Status column's menu: the fish go too.
        let output = frame_with(&mut app, &ctx, Vec::new());
        click(
            &mut app,
            &ctx,
            text_rect(&output, "Status: All").unwrap().center(),
        );
        let output = frame_with(&mut app, &ctx, Vec::new());
        click(
            &mut app,
            &ctx,
            text_rect(&output, "Creation Club")
                .expect("menu not open")
                .center(),
        );
        let output = frame_with(&mut app, &ctx, Vec::new());
        assert!(app.plugin_filter.hidden.contains(&Kind::CreationClub));
        assert!(
            text_rect(&output, "Base game").is_some(),
            "menu closed after one tick"
        );
        let screen = frame(&mut app, &ctx);
        assert!(
            screen.contains(&format!("Showing 0 of {total}")),
            "{screen}"
        );

        // Clear Filters brings everything back.
        let _ = frame_with(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
        let output = frame_with(&mut app, &ctx, Vec::new());
        click(
            &mut app,
            &ctx,
            text_rect(&output, "Clear Filters").unwrap().center(),
        );
        let screen = frame(&mut app, &ctx);
        assert!(!app.plugin_filter.is_active());
        assert!(screen.contains("Skyrim.esm"), "{screen}");
    }
}
