//! The window: Patina's frame and title bar, with a tab for each copy of the
//! game (ADR-0008); a sidebar with the game's load order and the characters
//! and their saves; the chosen one in the middle; a status bar. Plugins
//! switched on or off wait for Apply (ADR-0006).

use std::path::PathBuf;

use patina::frame::{self, TitleBar};
use patina::skin::{self, Skin};
use patina::tabs::{self, TabAction, TabStrip};
use skyplug_core::saves::{self, Sex};
use skyplug_core::GamePaths;

use crate::analysis::{self, Analysis, Kind, Pending, PluginFilter, Row, Setup};
use crate::config::{Config, Install};
use crate::game;
use crate::loader::{Characters, Command, Loader, Update};
use crate::table::{plugin_table, Colours, SkinColours};

/// Stock egui until the user picks a skin.
const HOME_SKIN: &str = skin::DEFAULT_SKIN;

/// The open save's screenshot takes this share of the details panel's width,
/// and at most this share of its height so the plugin list keeps the rest.
/// It never grows past its own size in pixels, where it would blur.
const SAVE_IMAGE_SHARE: f32 = 0.4;

/// How long a status bar notice stays up, in seconds.
const NOTICE_FOR: f64 = 3.0;

/// What the sidebar has chosen for the middle.
#[derive(Debug, Clone, PartialEq)]
enum Selection {
    LoadOrder,
    Save(PathBuf),
}

/// What the menus, shortcuts and panels ask for, handled after drawing.
enum Do {
    OpenFolder,
    /// Open Settings for the shown tab, or close it without saving.
    Settings(bool),
    /// Pick a folder for one of Settings' fields.
    Browse(Field),
    /// Fill Settings' folders in with Steam's copy of the game.
    FillFromSteam,
    SaveSettings,
    Tab(TabAction),
    Reload,
    Select(Selection),
    CopyReport,
    /// Switch a plugin on or off, until Apply.
    Switch(String, bool),
    /// Switch on every disabled plugin the open save was made with.
    EnableForSave,
    Apply,
    Revert,
    SetSkin(String),
    ReloadSkins,
    SystemTitleBar(bool),
    Quit,
}

/// Settings' folder fields.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Game,
    Prefix,
    Saves,
}

impl Field {
    fn label(self) -> &'static str {
        match self {
            Field::Game => "Game folder",
            Field::Prefix => "Wine prefix",
            Field::Saves => "Save folder",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Field::Game => "The folder holding SkyrimSE.exe",
            Field::Prefix => "The folder holding drive_c",
            Field::Saves => "Empty: the game's own",
        }
    }
}

/// Settings for the shown tab as edited, until saved.
#[derive(Debug, Default)]
struct Draft {
    name: String,
    game: String,
    prefix: String,
    saves: String,
    error: Option<String>,
}

impl Draft {
    fn of(install: &Install) -> Self {
        let text = |path: &Option<PathBuf>| path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        Self {
            name: install.name.clone(),
            game: text(&install.game_folder),
            prefix: text(&install.wine_prefix),
            saves: text(&install.save_folder),
            error: None,
        }
    }

    fn field(&mut self, field: Field) -> &mut String {
        match field {
            Field::Game => &mut self.game,
            Field::Prefix => &mut self.prefix,
            Field::Saves => &mut self.saves,
        }
    }

    fn install(&self) -> Install {
        let path = |text: &str| Some(text.trim()).filter(|t| !t.is_empty()).map(PathBuf::from);
        Install {
            name: Some(self.name.trim()).filter(|n| !n.is_empty()).unwrap_or("Skyrim").to_string(),
            game_folder: path(&self.game),
            wine_prefix: path(&self.prefix),
            save_folder: path(&self.saves),
        }
    }
}

/// Closing the window with changes not applied asks first.
#[derive(Debug, PartialEq)]
enum Closing {
    No,
    Asking,
    /// Applying, then closing once that's done.
    Applying,
    /// Close without asking again.
    Confirmed,
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
    /// The game's files once looked for: its paths, or why it wasn't found.
    game: Option<Result<GamePaths, String>>,
    /// Data, Plugins.txt and Skyrim.ccc once read.
    setup: Option<Result<Setup, String>>,
    /// The save folder, once the game has been looked for.
    folder: Option<PathBuf>,
    characters: Characters,
    unreadable: usize,
    selection: Selection,
    analysis: Option<Analysis>,
    /// The open save's screenshot, shown top right of its details.
    save_image: Option<egui::TextureHandle>,
    pending: Pending,
    /// Each table's column filters, kept while moving between saves.
    save_filter: PluginFilter,
    order_filter: PluginFilter,
    closing: Closing,
    /// Settings, while open.
    settings: Option<Draft>,
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
        let mut app = Self {
            custom_frame: !config.system_title_bar,
            config,
            skins,
            skin_errors,
            forced_skin: std::env::var("ARCANAEUM_SKIN").ok(),
            skin_colours: SkinColours::default(),
            loader: Loader::start(ctx),
            game: None,
            setup: None,
            folder: None,
            characters: Characters::new(),
            unreadable: 0,
            selection: Selection::LoadOrder,
            analysis: None,
            save_image: None,
            pending: Pending::default(),
            save_filter: PluginFilter::default(),
            order_filter: PluginFilter::default(),
            closing: Closing::No,
            settings: None,
            error: None,
            notice: None,
            screenshot: std::env::var_os("ARCANAEUM_SCREENSHOT")
                .map(|p| (PathBuf::from(p), false, None)),
        };
        app.apply_skin(ctx);
        app.loader.send(app.find_game_command());
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

    fn paths(&self) -> Option<&GamePaths> {
        self.game.as_ref().and_then(|g| g.as_ref().ok())
    }

    fn setup(&self) -> Option<&Setup> {
        self.setup.as_ref().and_then(|s| s.as_ref().ok())
    }

    /// The open save, if it's the one selected.
    fn open_save(&self) -> Option<&Analysis> {
        self.analysis
            .as_ref()
            .filter(|a| self.selection == Selection::Save(a.path.clone()))
    }

    fn save_rows(&self) -> Option<Vec<Row>> {
        self.open_save()
            .map(|a| analysis::save_rows(&a.save, self.setup(), &a.links, &self.pending))
    }

    fn order_rows(&self) -> Option<Vec<Row>> {
        self.setup()
            .map(|setup| analysis::load_order_rows(setup, &self.pending))
    }

    fn handle_updates(&mut self, ctx: &egui::Context) {
        for update in self.loader.poll() {
            match update {
                Update::Game(game) => {
                    let folder = std::env::var_os("ARCANAEUM_SAVE_FOLDER").map_or_else(
                        || self.config.install().save_folder(game.as_ref().ok()),
                        PathBuf::from,
                    );
                    if let Ok(paths) = &game {
                        self.loader.send(Command::ReadSetup(paths.clone()));
                    }
                    self.loader.send(Command::ScanFolder(folder.clone()));
                    self.folder = Some(folder);
                    self.game = Some(game);
                }
                Update::Folder {
                    path,
                    characters,
                    unreadable,
                } if Some(&path) == self.folder.as_ref() => {
                    self.characters = characters;
                    self.unreadable = unreadable;
                    if self.screenshot.is_some() && self.selection == Selection::LoadOrder {
                        self.open_screenshot_save(ctx);
                    }
                }
                Update::Folder { .. } => {}
                Update::Setup(setup) => self.setup = Some(setup),
                Update::Save(Ok(analysis)) => {
                    if self.selection == Selection::Save(analysis.path.clone()) {
                        self.save_image = analysis.screenshot().map(|image| {
                            ctx.load_texture("save-screenshot", image, egui::TextureOptions::LINEAR)
                        });
                        self.analysis = Some(*analysis);
                    }
                }
                Update::Save(Err(e)) => self.error = Some(e),
                Update::Applied(Ok(changed)) => {
                    self.pending.clear();
                    self.notify(ctx, format!("Plugins.txt updated, {changed} changed"));
                    if let Some(paths) = self.paths().cloned() {
                        self.loader.send(Command::ReadSetup(paths));
                    }
                    if self.closing == Closing::Applying {
                        self.closing = Closing::Confirmed;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Update::Applied(Err(e)) => {
                    self.error = Some(format!("Couldn't write Plugins.txt: {e}"));
                    self.closing = Closing::No;
                }
            }
        }
    }

    /// Screenshot mode: opens the newest save of `ARCANAEUM_CHARACTER`, or
    /// of the first character.
    fn open_screenshot_save(&mut self, ctx: &egui::Context) {
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
            let path = save.path.clone();
            self.act(ctx, Do::Select(Selection::Save(path)));
        }
    }

    fn shortcuts(&self, ctx: &egui::Context, actions: &mut Vec<Do>) {
        use egui::{Key, KeyboardShortcut, Modifiers};
        let pressed = |m, k| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        if pressed(Modifiers::COMMAND | Modifiers::SHIFT, Key::C) {
            actions.push(Do::CopyReport);
        }
        if pressed(Modifiers::COMMAND, Key::S) {
            actions.push(Do::Apply);
        }
        if pressed(Modifiers::COMMAND, Key::O) {
            actions.push(Do::OpenFolder);
        }
        if pressed(Modifiers::COMMAND, Key::Comma) {
            actions.push(Do::Settings(true));
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
        let changes = !self.pending.is_empty();
        ui.menu_button("File", |ui| {
            item(
                ui,
                actions,
                true,
                "Open Save Folder…",
                "Ctrl+O",
                Do::OpenFolder,
            );
            item(ui, actions, true, "Settings…", "Ctrl+,", Do::Settings(true));
            ui.separator();
            item(ui, actions, true, "Reload", "Ctrl+R", Do::Reload);
            ui.separator();
            item(ui, actions, true, "Quit", "Ctrl+Q", Do::Quit);
        });
        ui.menu_button("Edit", |ui| {
            item(ui, actions, changes, "Apply Changes", "Ctrl+S", Do::Apply);
            item(ui, actions, changes, "Revert Changes", "", Do::Revert);
            ui.separator();
            item(
                ui,
                actions,
                self.report().is_some(),
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

    /// Right side of the title bar, laid out right to left: the app's
    /// output, the changed load order and the report (ADR-0003, ADR-0006).
    fn output(&self, ui: &mut egui::Ui, actions: &mut Vec<Do>) {
        let label = match self.pending.len() {
            0 => "Apply".to_string(),
            n => format!("Apply ({n})"),
        };
        let apply = ui
            .add_enabled(!self.pending.is_empty(), egui::Button::new(label))
            .on_hover_text("Write the switched plugins to Plugins.txt (Ctrl+S)");
        if apply.clicked() {
            actions.push(Do::Apply);
        }
        let copy = ui
            .add_enabled(self.report().is_some(), egui::Button::new("Copy Report"))
            .on_hover_text("Copy the plugins shown and where to get them (Ctrl+Shift+C)");
        if copy.clicked() {
            actions.push(Do::CopyReport);
        }
    }

    /// The report for what's shown: the load order, or the open save.
    fn report(&self) -> Option<String> {
        match &self.selection {
            Selection::LoadOrder => self
                .order_rows()
                .map(|rows| analysis::report("Load order", &rows)),
            Selection::Save(_) => {
                let save = self.open_save()?;
                let h = &save.save.header;
                let title = format!(
                    "{} (level {}), save {}\n{}",
                    h.player_name,
                    h.player_level,
                    h.save_number,
                    save.path.display()
                );
                Some(analysis::report(&title, &self.save_rows()?))
            }
        }
    }

    /// Finding the game in the folder and prefix the config names.
    fn find_game_command(&self) -> Command {
        Command::FindGame {
            folder: self.config.install().game_folder.clone(),
            prefix: self.config.install().wine_prefix.clone(),
        }
    }

    /// Asks the game folder or the Steam lookup again, as the config now says.
    fn find_game(&mut self) {
        self.config.save();
        self.game = None;
        self.setup = None;
        self.pending.clear();
        self.loader.send(self.find_game_command());
    }

    /// Shows the install in tab `i`, starting over: its game, its saves.
    fn show_install(&mut self, i: usize) {
        self.config.shown = i;
        self.folder = None;
        self.characters.clear();
        self.unreadable = 0;
        self.selection = Selection::LoadOrder;
        self.analysis = None;
        self.save_image = None;
        self.find_game();
    }

    /// Tabs: show, add or close an install. Its Plugins.txt changes not
    /// applied would be lost, so the shown one stays until they're gone.
    fn tab(&mut self, ctx: &egui::Context, action: TabAction) {
        let leaves = match action {
            TabAction::Show(i) => i != self.config.shown,
            TabAction::New => true,
            TabAction::Close(i) => i == self.config.shown,
        };
        if leaves && !self.pending.is_empty() {
            self.notify(ctx, "Apply or revert the changes first");
            return;
        }
        match action {
            TabAction::Show(i) if leaves => self.show_install(i),
            TabAction::Show(_) => {}
            TabAction::New => {
                self.config.installs.push(Install::named("New Install"));
                self.show_install(self.config.installs.len() - 1);
                self.settings = Some(Draft::of(self.config.install()));
            }
            TabAction::Close(_) if self.config.installs.len() == 1 => {
                self.notify(ctx, "The last tab stays");
            }
            TabAction::Close(i) => {
                self.config.installs.remove(i);
                if leaves {
                    self.show_install(i.min(self.config.installs.len() - 1));
                } else {
                    if i < self.config.shown {
                        self.config.shown -= 1;
                    }
                    self.config.save();
                }
            }
        }
    }

    fn act(&mut self, ctx: &egui::Context, action: Do) {
        match action {
            Do::OpenFolder => {
                let mut dialog = rfd::FileDialog::new().set_title("Open Save Folder");
                if let Some(folder) = &self.folder {
                    dialog = dialog.set_directory(folder);
                }
                if let Some(folder) = dialog.pick_folder() {
                    self.config.install_mut().save_folder = Some(folder.clone());
                    self.config.save();
                    self.characters.clear();
                    self.selection = Selection::LoadOrder;
                    self.analysis = None;
                    self.save_image = None;
                    self.loader.send(Command::ScanFolder(folder.clone()));
                    self.folder = Some(folder);
                }
            }
            Do::Settings(true) => self.settings = Some(Draft::of(self.config.install())),
            Do::Settings(false) => self.settings = None,
            Do::Browse(field) => {
                let Some(draft) = &mut self.settings else { return };
                let mut dialog = rfd::FileDialog::new().set_title(field.hint());
                let current = PathBuf::from(draft.field(field).trim());
                if current.is_dir() {
                    dialog = dialog.set_directory(current);
                }
                if let Some(folder) = dialog.pick_folder() {
                    *draft.field(field) = folder.display().to_string();
                }
            }
            Do::FillFromSteam => {
                let Some(draft) = &mut self.settings else { return };
                match game::steam_copy() {
                    Ok((game_folder, prefix)) => {
                        draft.game = game_folder.display().to_string();
                        draft.prefix = prefix.map(|p| p.display().to_string()).unwrap_or_default();
                        draft.error = None;
                    }
                    Err(e) => draft.error = Some(e),
                }
            }
            Do::SaveSettings => {
                let Some(draft) = &mut self.settings else { return };
                let install = draft.install();
                let current = self.config.install();
                let moved = (&install.game_folder, &install.wine_prefix, &install.save_folder)
                    != (&current.game_folder, &current.wine_prefix, &current.save_folder);
                if moved && !self.pending.is_empty() {
                    draft.error = Some("Apply or revert the switched plugins first.".into());
                    return;
                }
                *self.config.install_mut() = install;
                self.settings = None;
                if moved {
                    self.show_install(self.config.shown);
                } else {
                    self.config.save();
                }
            }
            Do::Tab(action) => self.tab(ctx, action),
            Do::Reload => {
                if let Some(paths) = self.paths().cloned() {
                    self.loader.send(Command::ReadSetup(paths));
                }
                if let Some(folder) = self.folder.clone() {
                    self.loader.send(Command::ScanFolder(folder));
                }
                if let Selection::Save(path) = self.selection.clone() {
                    self.loader.send(Command::OpenSave(path));
                }
            }
            Do::Select(selection) => {
                if let Selection::Save(path) = &selection {
                    self.loader.send(Command::OpenSave(path.clone()));
                }
                self.selection = selection;
            }
            Do::CopyReport => {
                if let Some(report) = self.report() {
                    ctx.copy_text(report);
                    self.notify(ctx, "Report copied");
                }
            }
            Do::Switch(name, on) => {
                if let Some(Ok(setup)) = &self.setup {
                    self.pending.set(setup, &name, on);
                }
            }
            Do::EnableForSave => {
                let disabled: Vec<_> = self
                    .save_rows()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|row| row.kind == Kind::Disabled)
                    .collect();
                if let Some(Ok(setup)) = &self.setup {
                    for row in disabled {
                        self.pending.set(setup, &row.name, true);
                    }
                }
            }
            Do::Apply => {
                let Some(plugins_txt) = self.paths().map(|p| p.plugins_txt.clone()) else {
                    return;
                };
                if self.pending.is_empty() {
                    return;
                }
                if game::is_running() {
                    self.error = Some(
                        "Skyrim is running, and it rewrites Plugins.txt when it quits. Close it, then apply."
                            .into(),
                    );
                    self.closing = Closing::No;
                    return;
                }
                self.loader
                    .send(Command::Apply(plugins_txt, self.pending.changes()));
            }
            Do::Revert => self.pending.clear(),
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
        ui.heading("Game");
        ui.add_space(4.0);
        let label = match self.pending.len() {
            0 => "Load Order".to_string(),
            n => format!("Load Order  ({n} not applied)"),
        };
        let selected = self.selection == Selection::LoadOrder;
        let width = ui.available_width();
        let row = ui.add(
            egui::Button::selectable(selected, label)
                .min_size(egui::vec2(width, 0.0))
                .truncate(),
        );
        if row.clicked() && !selected {
            actions.push(Do::Select(Selection::LoadOrder));
        }

        ui.add_space(10.0);
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
        let current = match &self.selection {
            Selection::Save(path) => Some(path),
            Selection::LoadOrder => None,
        };
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let only_one = self.characters.len() == 1;
                for (name, saves) in &self.characters {
                    let holds_selected = saves.iter().any(|s| Some(&s.path) == current);
                    egui::CollapsingHeader::new(format!("{name}  ({})", saves.len()))
                        .id_salt(name)
                        .default_open(only_one || holds_selected)
                        .show(ui, |ui| {
                            for save in saves {
                                let h = &save.header;
                                let selected = Some(&save.path) == current;
                                let label = format!("{}  ·  {}", h.save_number, h.player_location);
                                let saved = saves::save_timestamp(&save.file_name)
                                    .map(|t| format!(", saved {t}"))
                                    .unwrap_or_default();
                                let row = ui
                                    .add(egui::Button::selectable(selected, label).truncate())
                                    .on_hover_text(format!(
                                        "{}\nLevel {}, {}{saved}",
                                        save.file_name, h.player_level, h.game_date
                                    ));
                                if row.clicked() && !selected {
                                    actions.push(Do::Select(Selection::Save(save.path.clone())));
                                }
                            }
                        });
                }
            });
    }

    fn colours(&self, ui: &egui::Ui) -> Colours {
        self.skin_colours.resolve(ui.visuals())
    }

    fn load_order_view(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Do>) {
        let waiting = |ui: &mut egui::Ui, text: &str| {
            ui.centered_and_justified(|ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.weak(text);
                });
            });
        };
        let paths = match &self.game {
            None => return waiting(ui, "Looking for Skyrim Special Edition…"),
            Some(Err(e)) => {
                ui.add_space(6.0);
                ui.heading("Skyrim Special Edition wasn't found");
                ui.label(e);
                ui.weak(
                    "Set the folder holding SkyrimSE.exe in Settings, and on Linux the Wine \
                     prefix it runs in, or fill both in from Steam's copy.",
                );
                if ui.button("Settings…").clicked() {
                    actions.push(Do::Settings(true));
                }
                return;
            }
            Some(Ok(paths)) => paths.clone(),
        };
        let rows = match &self.setup {
            None => return waiting(ui, "Reading the load order…"),
            Some(Err(e)) => {
                ui.add_space(6.0);
                ui.heading("The load order couldn't be read");
                ui.colored_label(ui.visuals().error_fg_color, e);
                return;
            }
            Some(Ok(setup)) => analysis::load_order_rows(setup, &self.pending),
        };
        let colours = self.colours(ui);

        ui.add_space(6.0);
        ui.heading("Load Order");
        ui.add(
            egui::Label::new(egui::RichText::new(paths.plugins_txt.display().to_string()).weak())
                .truncate(),
        );
        ui.add_space(6.0);
        let counts = analysis::counts(&rows);
        ui.horizontal_wrapped(|ui| {
            let mut first = true;
            for kind in Kind::ALL {
                let n = counts.get(&kind).copied().unwrap_or(0);
                if n == 0 {
                    continue;
                }
                if !first {
                    ui.weak("·");
                }
                first = false;
                ui.colored_label(
                    colours.of(kind),
                    format!("{n} {}", kind.label().to_lowercase()),
                );
            }
            if !self.pending.is_empty() {
                ui.add_space(12.0);
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!(
                        "{} not applied",
                        plural(self.pending.len(), "change", "changes")
                    ),
                );
            }
        });
        ui.add_space(4.0);
        for (name, on) in plugin_table(ui, "load-order", &rows, &mut self.order_filter, &colours) {
            actions.push(Do::Switch(name, on));
        }
    }

    fn save_view(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Do>) {
        let (Some(analysis), Some(rows)) = (self.open_save(), self.save_rows()) else {
            ui.centered_and_justified(|ui| {
                if self.loader.busy() {
                    ui.spinner();
                } else {
                    ui.weak("This save couldn't be opened.");
                }
            });
            return;
        };
        let h = &analysis.save.header;
        let colours = self.colours(ui);

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
                Sex::Unknown(_) => "",
            };
            ui.weak(format!(
                "Level {} {race}{sex}  ·  {}",
                h.player_level, h.player_location
            ));
            ui.add_space(6.0);
            let file_name = analysis
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
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
                    if let Some(saved) = saves::save_timestamp(&file_name) {
                        line(ui, "Saved", &saved);
                    }
                    line(ui, "In-game date", &h.game_date);
                    line(ui, "File", &analysis.path.display().to_string());
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
        let counts = analysis::counts(&rows);
        let count = |kind| counts.get(&kind).copied().unwrap_or(0);
        ui.horizontal_wrapped(|ui| {
            ui.heading("Plugins");
            ui.add_space(8.0);
            for (i, kind) in [Kind::Active, Kind::Disabled, Kind::NotInstalled, Kind::NotFound]
                .into_iter()
                .enumerate()
            {
                if i > 0 {
                    ui.weak("·");
                }
                ui.colored_label(colours.of(kind), format!("{} {}", count(kind), kind.label().to_lowercase()));
            }
            let disabled = count(Kind::Disabled);
            if disabled > 0 {
                ui.add_space(12.0);
                let enable = ui
                    .button(format!(
                        "Enable {}",
                        plural(disabled, "Disabled Plugin", "Disabled Plugins")
                    ))
                    .on_hover_text("Switch on every plugin this save was made with that's in Data but switched off");
                if enable.clicked() {
                    actions.push(Do::EnableForSave);
                }
            }
        });
        if self.setup().is_none() {
            ui.weak("Without the game's Data folder and Plugins.txt, only the plugins' names are known.");
        }
        ui.add_space(4.0);
        for (name, on) in plugin_table(ui, "save-plugins", &rows, &mut self.save_filter, &colours) {
            actions.push(Do::Switch(name, on));
        }
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
            if let Some(folder) = &self.folder {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(folder.display().to_string()).weak())
                            .truncate(),
                    );
                });
            }
        });
    }

    /// Closing with changes not applied: apply them, drop them, or stay.
    fn closing(&mut self, ctx: &egui::Context, actions: &mut Vec<Do>) {
        let asked = ctx.input(|i| i.viewport().close_requested());
        if asked && !self.pending.is_empty() && self.closing != Closing::Confirmed {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.closing == Closing::No {
                self.closing = Closing::Asking;
            }
        }
        if self.closing != Closing::Asking {
            return;
        }
        let modal = egui::Modal::new(egui::Id::new("closing")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.heading("Apply your changes?");
            ui.label(format!(
                "{} switched on or off, but Plugins.txt hasn't been changed yet.",
                plural(self.pending.len(), "plugin is", "plugins are")
            ));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Apply and Quit").clicked() {
                    self.closing = Closing::Applying;
                    actions.push(Do::Apply);
                }
                if ui.button("Quit Without Applying").clicked() {
                    self.closing = Closing::Confirmed;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ui.button("Cancel").clicked() {
                    self.closing = Closing::No;
                }
            });
        });
        if modal.should_close() && self.closing == Closing::Asking {
            self.closing = Closing::No;
        }
    }

    /// The shown tab's settings: its name and folders, the same for any
    /// copy of the game. Fill In From Steam finds Steam's copy's folders.
    fn settings_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Do>) {
        let Some(draft) = &mut self.settings else {
            return;
        };
        let modal = egui::Modal::new(egui::Id::new("settings")).show(ctx, |ui| {
            ui.set_width(520.0);
            ui.heading("Settings");
            ui.weak("For the tab shown; each tab is a copy of the game.");
            ui.add_space(8.0);
            let mut fields = vec![Field::Game];
            // On Windows the game's files are in the user's own folders.
            if !cfg!(windows) {
                fields.push(Field::Prefix);
            }
            fields.push(Field::Saves);
            // Grid cells shrink to their content, so text fields are sized.
            let field_size = egui::vec2(360.0, ui.spacing().interact_size.y);
            egui::Grid::new("settings_fields")
                .num_columns(3)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Name");
                    ui.add_sized(field_size, egui::TextEdit::singleline(&mut draft.name));
                    ui.end_row();
                    for field in fields {
                        ui.label(field.label());
                        ui.add_sized(
                            field_size,
                            egui::TextEdit::singleline(draft.field(field)).hint_text(field.hint()),
                        );
                        if ui.button("Browse…").clicked() {
                            actions.push(Do::Browse(field));
                        }
                        ui.end_row();
                    }
                });
            ui.add_space(4.0);
            if ui
                .button("Fill In From Steam")
                .on_hover_text("Set the game folder and Wine prefix to Steam's copy of the game")
                .clicked()
            {
                actions.push(Do::FillFromSteam);
            }
            if let Some(error) = &draft.error {
                ui.add_space(4.0);
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    actions.push(Do::SaveSettings);
                }
                if ui.button("Cancel").clicked() {
                    actions.push(Do::Settings(false));
                }
            });
        });
        if modal.should_close() {
            actions.push(Do::Settings(false));
        }
    }

    /// Screenshot mode: once the first save is shown, save the window and quit.
    fn take_screenshot(&mut self, ctx: &egui::Context) {
        let ready = self.open_save().is_some()
            || (!self.loader.busy() && self.folder.is_some() && self.characters.is_empty());
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
        self.closing(&ctx, &mut actions);
        self.settings_window(&ctx, &mut actions);
        if self.closing == Closing::No && self.settings.is_none() {
            self.shortcuts(&ctx, &mut actions);
        }
        let mut output = Vec::new();
        let titles: Vec<&str> = self.config.installs.iter().map(|i| i.name.as_str()).collect();
        let shown = self.config.shown;
        let mut tab = None;
        if self.closing == Closing::No && self.settings.is_none() {
            tab = tabs::keys(&ctx, titles.len(), shown);
        }
        let header = frame::header_frame(ui.style(), &style, radius);
        egui::Panel::top("header").frame(header).show(ui, |ui| {
            // The bar's full height, padding and all, for the tabs to fill.
            let span = (ui.max_rect() + header.inner_margin).y_range();
            TitleBar::new("", &style)
                .custom(self.custom_frame)
                .menu_bar(true)
                .span(span)
                .show(
                    ui,
                    |ui| {
                        self.menus(ui, &mut actions);
                        let strip = TabStrip::new(&titles, shown, &style)
                            .span(span)
                            .new_hint("New Install (Ctrl+Shift+T)")
                            .show(ui);
                        tab = tab.or(strip);
                    },
                    |ui| self.output(ui, &mut output),
                );
        });
        actions.extend(tab.map(Do::Tab));
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

        egui::Panel::left("sidebar")
            .default_size(280.0)
            .resizable(true)
            .show(ui, |ui| self.sidebar(ui, &mut actions));

        egui::CentralPanel::default().show(ui, |ui| match self.selection {
            Selection::LoadOrder => self.load_order_view(ui, &mut actions),
            Selection::Save(_) => self.save_view(ui, &mut actions),
        });

        for action in actions {
            self.act(&ctx, action);
        }
        if self.custom_frame {
            frame::edges(&ctx, radius, style.border);
        }
        self.take_screenshot(&ctx);
    }
}

/// "1 change", "3 changes".
fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
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

#[cfg(test)]
mod tests;
