//! The window, driven through egui with real pointer and key events.

use super::*;

#[test]
fn the_home_skin_exists() {
    let (skins, _) = skin::load_all(skin::BUILT_IN, &[]);
    assert!(skins.iter().any(|s| s.name == HOME_SKIN));
}

#[test]
fn plurals() {
    assert_eq!(plural(1, "change", "changes"), "1 change");
    assert_eq!(plural(3, "change", "changes"), "3 changes");
}

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

fn input(size: egui::Vec2, events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        events,
        ..Default::default()
    }
}

/// Draws a frame of the window with `events` as its input.
fn frame_with(app: &mut App, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run_ui(input(egui::vec2(1280.0, 768.0), events), |ui| app.draw(ui))
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
    output
        .shapes
        .iter()
        .rev()
        .find_map(|s| find(&s.shape, text))
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

fn click_text(app: &mut App, ctx: &egui::Context, text: &str) {
    let output = frame_with(app, ctx, Vec::new());
    let rect = text_rect(&output, text).unwrap_or_else(|| panic!("{text:?} not on screen"));
    click(app, ctx, rect.center());
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }]
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

fn empty_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn an_empty_folder_says_so_under_word_menus() {
    let ctx = egui::Context::default();
    let config = Config::with(Install {
        save_folder: Some(empty_dir("arcanaeum_empty_folder_test")),
        ..Install::default()
    });
    let mut app = App::with_context(&ctx, config);
    let screen = settle(&mut app, &ctx);
    for text in [
        "File",
        "Edit",
        "View",
        "Skins",
        "Skyrim",
        "Apply",
        "Copy Report",
        "Load Order",
        "No saves in this folder.",
    ] {
        assert!(screen.contains(text), "{text:?} not on screen:\n{screen}");
    }
}

/// A Steam library in a temp folder, with Skyrim's Proton prefix: Data
/// holds Skyrim.esm, Mod.esp (off) and Other.esp (on).
#[cfg(not(windows))]
fn fake_install(name: &str) -> (PathBuf, PathBuf) {
    let root = empty_dir(name);
    let game = root.join("steamapps/common/Skyrim Special Edition");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    for plugin in ["Skyrim.esm", "Mod.esp", "Other.esp"] {
        std::fs::write(game.join("Data").join(plugin), b"").unwrap();
    }
    std::fs::write(game.join("Skyrim.ccc"), b"").unwrap();
    let plugins_txt = skyplug_core::GameInstall::from_game_dir(&game)
        .unwrap()
        .plugins_txt_path(game::APPID);
    assert!(
        plugins_txt.starts_with(&root),
        "the test must never touch a real Plugins.txt"
    );
    std::fs::create_dir_all(plugins_txt.parent().unwrap()).unwrap();
    std::fs::write(&plugins_txt, "*Other.esp\nMod.esp\n").unwrap();
    (game, plugins_txt)
}

/// Switches Mod.esp on by its checkbox, then applies with Ctrl+S; and asks
/// before closing with a change not applied.
#[cfg(not(windows))]
#[test]
fn switching_a_plugin_waits_for_apply() {
    let (game_folder, plugins_txt) = fake_install("arcanaeum_fake_install_test");
    let ctx = egui::Context::default();
    let config = Config::with(Install {
        game_folder: Some(game_folder),
        save_folder: Some(empty_dir("arcanaeum_fake_install_saves")),
        ..Install::default()
    });
    let mut app = App::with_context(&ctx, config);
    let screen = settle(&mut app, &ctx);
    assert!(
        screen.contains("Mod.esp") && screen.contains("Disabled"),
        "{screen}"
    );

    // The checkbox sits in the column left of the name.
    let output = frame_with(&mut app, &ctx, Vec::new());
    let name = text_rect(&output, "Mod.esp").unwrap();
    click(
        &mut app,
        &ctx,
        egui::pos2(name.min.x - 29.0, name.center().y),
    );
    let screen = frame(&mut app, &ctx);
    assert_eq!(
        app.pending.changes(),
        vec![("Mod.esp".to_string(), true)],
        "{screen}"
    );
    for text in [
        "Apply (1)",
        "1 change not applied",
        "Load Order  (1 not applied)",
        "· not applied",
    ] {
        assert!(screen.contains(text), "{text:?} not on screen:\n{screen}");
    }
    assert_eq!(
        std::fs::read_to_string(&plugins_txt).unwrap(),
        "*Other.esp\nMod.esp\n"
    );

    // Closing asks first; Cancel stays.
    let mut closing = input(egui::vec2(1280.0, 768.0), Vec::new());
    closing
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    let _ = ctx.run_ui(closing, |ui| app.draw(ui));
    let screen = frame(&mut app, &ctx);
    assert!(screen.contains("Apply your changes?"), "{screen}");
    click_text(&mut app, &ctx, "Cancel");
    assert_eq!(app.closing, Closing::No);

    // Ctrl+S writes it, keeping the old file as a backup.
    let _ = frame_with(&mut app, &ctx, key(egui::Key::S, egui::Modifiers::COMMAND));
    let screen = settle(&mut app, &ctx);
    assert!(app.error.is_none(), "{:?} (is Skyrim running?)", app.error);
    assert_eq!(
        std::fs::read_to_string(&plugins_txt).unwrap(),
        "*Other.esp\n*Mod.esp\n"
    );
    assert_eq!(
        std::fs::read_to_string(plugins_txt.with_extension("txt.bak")).unwrap(),
        "*Other.esp\nMod.esp\n"
    );
    assert!(app.pending.is_empty());
    assert!(
        screen.contains("Plugins.txt updated, 1 changed"),
        "{screen}"
    );
    assert!(!screen.contains("Disabled"), "{screen}");
}

fn real_saves() -> PathBuf {
    PathBuf::from(std::env::var_os("ARCANAEUM_TEST_SAVES").expect("set ARCANAEUM_TEST_SAVES"))
}

/// The real game, but saves from `ARCANAEUM_TEST_SAVES`, with the newest
/// save open. Nothing is written.
fn open_newest_real_save(ctx: &egui::Context) -> App {
    let config = Config::with(Install {
        save_folder: Some(real_saves()),
        ..Install::default()
    });
    let mut app = App::with_context(ctx, config);
    settle(&mut app, ctx);
    let save = app.characters.values().next().expect("no saves")[0]
        .path
        .clone();
    app.act(ctx, Do::Select(Selection::Save(save)));
    settle(&mut app, ctx);
    app
}

/// Opens the newest save in `ARCANAEUM_TEST_SAVES` (a real save folder,
/// only read): `ARCANAEUM_TEST_SAVES=… cargo test window_tests -- --include-ignored`.
#[test]
#[ignore]
fn a_real_save_shows_its_plugins() {
    let ctx = egui::Context::default();
    let mut app = open_newest_real_save(&ctx);
    let screen = frame(&mut app, &ctx);
    let analysis = app.open_save().expect("save didn't open").clone();
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
        let size = egui::vec2(width, height);
        // Twice: panels settle their sizes on the second frame.
        let _ = ctx.run_ui(input(size, Vec::new()), |ui| app.draw(ui));
        let output = ctx.run_ui(input(size, Vec::new()), |ui| app.draw(ui));
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
}

/// Filters the plugins of the newest save in `ARCANAEUM_TEST_SAVES` by
/// clicking and typing, as a user would.
#[test]
#[ignore]
fn the_plugin_columns_filter() {
    let ctx = egui::Context::default();
    let mut app = open_newest_real_save(&ctx);
    let screen = frame(&mut app, &ctx);
    let total = app.open_save().unwrap().save.plugins.len();
    assert!(screen.contains("Skyrim.esm"), "{screen}");
    assert!(screen.contains(&format!("Showing {total} of {total}")));

    // Click into the Plugin column's filter and type.
    click_text(&mut app, &ctx, "Plugin");
    let _ = frame_with(&mut app, &ctx, vec![egui::Event::Text("FISH".into())]);
    let screen = frame(&mut app, &ctx);
    assert_eq!(app.save_filter.name, "FISH");
    assert!(!screen.contains("Skyrim.esm"), "{screen}");
    let plugins = &app.open_save().unwrap().save.plugins;
    let fish = plugins
        .iter()
        .filter(|p| p.to_lowercase().contains("fish"))
        .count();
    assert!(fish > 0, "this save has no fish plugin to filter for");
    assert!(
        screen.contains(&format!("Showing {fish} of {total}")),
        "{screen}"
    );

    // Untick Creation Club in the Status column's menu: the fish go too,
    // and the menu stays open.
    click_text(&mut app, &ctx, "Status: All");
    click_text(&mut app, &ctx, "Creation Club");
    let output = frame_with(&mut app, &ctx, Vec::new());
    assert!(app.save_filter.hidden.contains(&Kind::CreationClub));
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
    let _ = frame_with(&mut app, &ctx, key(egui::Key::Escape, Default::default()));
    click_text(&mut app, &ctx, "Clear Filters");
    let screen = frame(&mut app, &ctx);
    assert!(!app.save_filter.is_active());
    assert!(screen.contains("Skyrim.esm"), "{screen}");
}

/// GOG's build outside any Steam library, set up in Settings like any
/// other copy: its folder and Wine prefix, then Save.
#[cfg(not(windows))]
#[test]
fn a_gog_install_is_set_up_in_settings() {
    let (gog, plugins_txt) = fake_gog_install("arcanaeum_gog_install_test");
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, Config::with(Install::default()));
    let screen = settle(&mut app, &ctx);
    assert!(screen.contains("No game folder is set"), "{screen}");

    let _ = frame_with(&mut app, &ctx, key(egui::Key::Comma, egui::Modifiers::COMMAND));
    // The modal's first frame only measures it.
    frame(&mut app, &ctx);
    let screen = frame(&mut app, &ctx);
    for text in ["Settings", "Name", "Game folder", "Wine prefix", "Save folder", "Fill In From Steam"] {
        assert!(screen.contains(text), "{text:?} not on screen:\n{screen}");
    }
    let draft = app.settings.as_mut().unwrap();
    draft.name = "GOG".into();
    draft.game = gog.game_folder.unwrap().display().to_string();
    draft.prefix = gog.wine_prefix.unwrap().display().to_string();
    click_text(&mut app, &ctx, "Save");
    let screen = settle(&mut app, &ctx);
    assert!(app.settings.is_none());
    assert_eq!(app.config.install().name, "GOG");
    assert_eq!(app.paths().map(|p| p.plugins_txt.clone()), Some(plugins_txt));
    assert!(screen.contains("Mod.esp") && screen.contains("Active"), "{screen}");
}

/// Steam's copy in one tab, GOG's in another: a click on a tab shows its
/// copy, but not while changes to the shown one wait to be applied.
#[cfg(not(windows))]
#[test]
fn tabs_switch_between_copies_of_the_game() {
    let (steam_folder, steam_plugins_txt) = fake_install("arcanaeum_tabs_steam");
    let (gog, gog_plugins_txt) = fake_gog_install("arcanaeum_tabs_gog");
    let mut config = Config::with(Install {
        game_folder: Some(steam_folder),
        save_folder: Some(empty_dir("arcanaeum_tabs_steam_saves")),
        ..Install::named("Steam")
    });
    config.installs.push(gog);
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, config);
    let screen = settle(&mut app, &ctx);
    assert!(screen.contains("Steam") && screen.contains("GOG"), "{screen}");
    assert_eq!(app.paths().map(|p| p.plugins_txt.clone()), Some(steam_plugins_txt.clone()));

    app.act(&ctx, Do::Switch("Mod.esp".into(), true));
    click_text(&mut app, &ctx, "GOG");
    assert_eq!(app.config.shown, 0, "changes not applied keep the tab");

    app.act(&ctx, Do::Revert);
    click_text(&mut app, &ctx, "GOG");
    settle(&mut app, &ctx);
    assert_eq!(app.config.shown, 1);
    assert_eq!(app.paths().map(|p| p.plugins_txt.clone()), Some(gog_plugins_txt));

    let _ = frame_with(&mut app, &ctx, key(egui::Key::PageUp, egui::Modifiers::CTRL));
    settle(&mut app, &ctx);
    assert_eq!(app.paths().map(|p| p.plugins_txt.clone()), Some(steam_plugins_txt));
}

/// GOG's build outside any Steam library, in a temp folder, with its own
/// Wine prefix: Data holds Skyrim.esm and Mod.esp (on).
#[cfg(not(windows))]
fn fake_gog_install(name: &str) -> (Install, PathBuf) {
    let root = empty_dir(name);
    let game = root.join("Skyrim Special Edition");
    let prefix = root.join("prefix");
    std::fs::create_dir_all(game.join("Data")).unwrap();
    std::fs::write(game.join("goggame-1711230643.info"), "{}").unwrap();
    for plugin in ["Skyrim.esm", "Mod.esp"] {
        std::fs::write(game.join("Data").join(plugin), b"").unwrap();
    }
    std::fs::write(game.join("Skyrim.ccc"), b"").unwrap();
    let local = prefix.join("drive_c/users/steamuser/AppData/Local/Skyrim Special Edition GOG");
    std::fs::create_dir_all(&local).unwrap();
    std::fs::write(local.join("Plugins.txt"), "*Mod.esp\n").unwrap();
    let install = Install {
        name: "GOG".into(),
        game_folder: Some(game),
        wine_prefix: Some(prefix),
        save_folder: Some(empty_dir(&format!("{name}_saves"))),
    };
    (install, local.join("Plugins.txt"))
}

/// A tab is renamed from its right-click menu's dialog, and an empty name
/// leaves it as it was.
#[test]
fn a_tab_is_renamed() {
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, Config::with(Install::named("Steam")));
    settle(&mut app, &ctx);
    app.act(&ctx, Do::Rename(0));
    assert_eq!(app.renaming, Some((0, "Steam".to_string())));
    app.act(&ctx, Do::RenameTo("  GOG ".into()));
    assert_eq!(app.config.install().name, "GOG");
    assert!(app.renaming.is_none());
    app.act(&ctx, Do::Rename(0));
    app.act(&ctx, Do::RenameTo("   ".into()));
    assert_eq!(app.config.install().name, "GOG");
}

/// The model preview: a model from the game's files is drawn and its textures
/// are listed with where each came from, a drag turns it, and Copy Report
/// has something to copy.
#[test]
fn the_model_preview_shows_a_model_its_textures_and_turns() {
    let paths = crate::preview::tests::fixture_install("app_preview");
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, Config::with(Install::default()));
    settle(&mut app, &ctx);
    app.game = Some(Ok(paths.clone()));

    click_text(&mut app, &ctx, "Model Preview");
    let screen = frame(&mut app, &ctx);
    assert!(screen.contains("Type a model's path"), "{screen}");

    app.preview.add("meshes/fixture/quad.nif", &paths);
    let mut screen = String::new();
    for _ in 0..400 {
        screen = frame(&mut app, &ctx);
        if app.preview.is_drawn() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(app.preview.is_drawn(), "no picture was drawn:\n{screen}");
    let screen = frame(&mut app, &ctx);
    for text in ["Model Preview", "quad.nif", "Textures", "diffuse: red.dds", "loose: textures/fixture/red.dds", "normal: gone_n.dds  MISSING"] {
        assert!(screen.contains(text), "{text:?} not on screen:\n{screen}");
    }

    // Copy Report copies the same facts.
    let report = app.report().expect("a report");
    assert!(report.contains("red.dds") && report.contains("MISSING") && report.contains("from loose: textures/fixture/red.dds"), "{report}");

    // Dragging the picture turns the camera: right turns the model toward the right.
    let output = frame_with(&mut app, &ctx, Vec::new());
    let hint = text_rect(&output, "Drag to turn, scroll to zoom").expect("the hint");
    let start = egui::pos2(hint.min.x + 150.0, hint.max.y + 200.0);
    let before = app.preview.camera();
    let button = |pressed| egui::Event::PointerButton { pos: start, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    let _ = frame_with(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
    let _ = frame_with(&mut app, &ctx, vec![button(true)]);
    let _ = frame_with(&mut app, &ctx, vec![egui::Event::PointerMoved(start + egui::vec2(40.0, 0.0))]);
    let _ = frame_with(&mut app, &ctx, vec![egui::Event::PointerMoved(start + egui::vec2(80.0, 0.0))]);
    let _ = frame_with(&mut app, &ctx, vec![button(false)]);
    let after = app.preview.camera();
    assert!(after.azimuth > before.azimuth + 20.0, "azimuth {} -> {}", before.azimuth, after.azimuth);

    // A view button sets the azimuth; Reset restores the start.
    click_text(&mut app, &ctx, "Back");
    assert_eq!(app.preview.camera().azimuth, 180.0);
    click_text(&mut app, &ctx, "Reset");
    assert_eq!(app.preview.camera(), before);
}

/// A model that isn't there says so in the report, and the view doesn't give up.
#[test]
fn a_model_the_game_does_not_have_is_reported() {
    let paths = crate::preview::tests::fixture_install("app_preview_missing");
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, Config::with(Install::default()));
    settle(&mut app, &ctx);
    app.game = Some(Ok(paths.clone()));
    app.selection = Selection::Preview;
    app.preview.add("meshes/nothing/here.nif", &paths);
    let mut screen = String::new();
    for _ in 0..200 {
        screen = frame(&mut app, &ctx);
        if screen.contains("no such file") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(screen.contains("the game has no such file"), "{screen}");
    assert!(screen.contains("here.nif"), "{screen}");
}

/// Armours are picked from a plugin's records: open the picker, choose a plugin, click an
/// armour to add its model, click again to take it away.
#[test]
fn armours_are_picked_from_a_plugin() {
    let paths = crate::preview::tests::fixture_install("app_preview_picker");
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, Config::with(Install::default()));
    settle(&mut app, &ctx);
    app.game = Some(Ok(paths.clone()));
    app.selection = Selection::Preview;

    // Waits until `text` is on screen (the worker answers in its own time).
    let until = |app: &mut App, text: &str| {
        let mut screen = String::new();
        for _ in 0..400 {
            screen = frame(app, &ctx);
            if screen.contains(text) {
                return screen;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        panic!("{text:?} never appeared:\n{screen}");
    };

    frame(&mut app, &ctx);
    click_text(&mut app, &ctx, "From a Plugin…");
    until(&mut app, "Fixture.esp  (2)");
    click_text(&mut app, &ctx, "Fixture.esp  (2)");
    let screen = until(&mut app, "Fixture Cuirass   Body");
    assert!(screen.contains("2 of 2 in Fixture.esp") && screen.contains("Fixture Cuirass Alt   Body"), "{screen}");

    click_text(&mut app, &ctx, "Fixture Cuirass   Body");
    assert_eq!(app.preview.items(), ["meshes/fixture/quad.nif"]);
    for _ in 0..400 {
        frame(&mut app, &ctx);
        if app.preview.is_drawn() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(app.preview.is_drawn(), "the armour's model was drawn");

    // The variant is a different item though it is the same model: it adds beside the first.
    click_text(&mut app, &ctx, "Fixture Cuirass Alt   Body");
    assert_eq!(app.preview.items(), ["meshes/fixture/quad.nif", "meshes/fixture/quad.nif"]);
    // Clicking one again takes it away; Add All Shown puts both back.
    click_text(&mut app, &ctx, "Fixture Cuirass   Body");
    assert_eq!(app.preview.items().len(), 1);
    click_text(&mut app, &ctx, "Fixture Cuirass Alt   Body");
    assert!(app.preview.items().is_empty());
    click_text(&mut app, &ctx, "Add All Shown");
    assert_eq!(app.preview.items().len(), 2);
}

/// Hair is picked in its own window: open it, hide a hairstyle, apply (a patch plugin is written and
/// switched on), then show it again and apply (the patch goes).
#[test]
fn hair_is_hidden_and_shown_from_its_window() {
    let paths = crate::hair::tests::hair_install("app_hair");
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, Config::with(Install::default()));
    settle(&mut app, &ctx);
    app.game = Some(Ok(paths.clone()));
    app.selection = Selection::Preview;

    let until = |app: &mut App, text: &str| {
        let mut screen = String::new();
        for _ in 0..400 {
            screen = frame(app, &ctx);
            if screen.contains(text) {
                return screen;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        panic!("{text:?} never appeared:\n{screen}");
    };

    frame(&mut app, &ctx);
    click_text(&mut app, &ctx, "Hair…");
    // The plugin with hair is chosen for you; women's hairstyles are what's listed first.
    let screen = until(&mut app, "1 of 1 shown in game");
    assert!(screen.contains("FixtureHairs.esp  (3)") && screen.contains("Zod"), "{screen}");
    assert!(!screen.contains("Zac") && !screen.contains("Stubble"), "men's hair and beards are filtered out:\n{screen}");

    click_text(&mut app, &ctx, "Zod");
    let screen = until(&mut app, "0 of 1 shown in game");
    assert!(screen.contains("Apply (1)") && screen.contains("FixtureHairs.esp  (3, 1 hidden)"), "{screen}");
    // Revert puts it back without writing anything.
    click_text(&mut app, &ctx, "Revert");
    until(&mut app, "1 of 1 shown in game");
    assert!(!paths.data_dir.join(skyplug_core::hair::PATCH_NAME).exists());

    click_text(&mut app, &ctx, "Zod");
    until(&mut app, "Apply (1)");
    click_text(&mut app, &ctx, "Apply (1)");
    until(&mut app, "1 hidden in Arcanaeum Hair Picks.esp");
    let patch = paths.data_dir.join(skyplug_core::hair::PATCH_NAME);
    assert_eq!(skyplug_core::hair::hidden_by_patch(&patch).unwrap().len(), 1);
    assert!(std::fs::read_to_string(&paths.plugins_txt).unwrap().contains("*Arcanaeum Hair Picks.esp"));
    // The load order shown is read again, so the new plugin is in it.
    for _ in 0..200 {
        frame(&mut app, &ctx);
        if app.setup.as_ref().is_some_and(|s| s.as_ref().is_ok_and(|s| s.statuses.iter().any(|p| p.name == skyplug_core::hair::PATCH_NAME))) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(app.setup.as_ref().is_some_and(|s| s.as_ref().is_ok_and(|s| s.statuses.iter().any(|p| p.name == skyplug_core::hair::PATCH_NAME))), "the load order was read again");

    click_text(&mut app, &ctx, "Zod");
    until(&mut app, "Apply (1)");
    click_text(&mut app, &ctx, "Apply (1)");
    until(&mut app, "every hair is shown again");
    assert!(!patch.exists());
}

/// Double-clicking a hair opens it large, in a window of its own, and leaves its tick as it was.
#[test]
fn a_hair_opens_large_on_double_click() {
    let paths = crate::hair::tests::hair_install("app_hair_big");
    let ctx = egui::Context::default();
    let mut app = App::with_context(&ctx, Config::with(Install::default()));
    settle(&mut app, &ctx);
    app.game = Some(Ok(paths.clone()));
    app.selection = Selection::Preview;
    let until = |app: &mut App, ok: &dyn Fn(&mut App, &str) -> bool| {
        let mut screen = String::new();
        for _ in 0..400 {
            screen = frame(app, &ctx);
            if ok(app, &screen) {
                return screen;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        panic!("never happened:\n{screen}");
    };

    frame(&mut app, &ctx);
    click_text(&mut app, &ctx, "Hair…");
    until(&mut app, &|_, s| s.contains("1 of 1 shown in game"));
    assert!(app.preview.hair().big_label().is_none());

    // Two clicks on the card in quick succession are a double-click.
    let at = text_rect(&frame_with(&mut app, &ctx, vec![]), "Zod").expect("the card is drawn").center();
    // egui reads a third click within 0.6 s of an earlier one as a triple-click wherever it was:
    // let the click that opened the window age first (a test frame is 1/60 s).
    for _ in 0..60 {
        frame(&mut app, &ctx);
    }
    let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
    let _ = frame_with(&mut app, &ctx, vec![egui::Event::PointerMoved(at)]);
    // Two clicks a few frames apart (egui says double within 0.3 s), each its own frame.
    for events in [vec![button(true)], vec![button(false)], vec![], vec![], vec![button(true)], vec![button(false)]] {
        let _ = frame_with(&mut app, &ctx, events);
    }
    let screen = until(&mut app, &|a, _| a.preview.hair().big_label().is_some());
    assert_eq!(app.preview.hair().big_label().as_deref(), Some("Zod"));
    assert!(screen.contains("Shown in game"), "{screen}");
    // The picture comes, and the two clicks left the tick as it was.
    until(&mut app, &|a, _| a.preview.hair().big_drawn());
    assert!(screen.contains("1 of 1 shown in game"), "{screen}");
    assert_eq!(app.preview.hair().changes(), 0);

    // Its tick in the big window is the same tick as on the card.
    click_text(&mut app, &ctx, "Shown in game");
    until(&mut app, &|_, s| s.contains("0 of 1 shown in game"));
    assert_eq!(app.preview.hair().changes(), 1);
}
