//! Picking hair (ADR-0012): a window that draws every hair the load order has, so you can tick
//! the ones you want on the character creation screen and hide the rest.
//!
//! Hair packs add hundreds of hairs to one flat list. Here each hair is a picture you click;
//! Apply writes a small patch plugin ([`skyplug_core::hair`]) that clears the playable flag of
//! the unticked ones and switches it on in `Plugins.txt`. Nothing is deleted, so ticking a
//! hair again and applying brings it back.
//!
//! Like the model preview (ADR-0009) this has a worker thread of its own (ADR-0002): reading
//! the plugins, drawing the pictures one by one as they scroll into view, and writing the patch.
//! It has its own, not the preview's, so a screenful of hair doesn't hold up turning a model.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use skyplug_core::armour::FormKey;
use skyplug_core::assets::AssetResolver;
use skyplug_core::dds::Rgba;
use skyplug_core::hair::{hidden_by_patch, HairCatalog, PartKind, PATCH_NAME};
use skyplug_core::render::{Camera, Model};
use skyplug_core::GamePaths;

use crate::table::Colours;

/// How big a picture of one hair is, in pixels.
const THUMB: usize = 112;
/// Textures for the pictures are decoded at about this size: they're small.
const THUMB_TEXTURE_SIDE: usize = 256;
/// The big picture's size, and the texture detail it's drawn with.
const BIG: usize = 448;
const BIG_TEXTURE_SIDE: usize = 1024;
const VIEWS: [(&str, f32); 5] = [("Front", 0.0), ("¾", 30.0), ("Right", 90.0), ("Back", 180.0), ("Left", 270.0)];
const CARD_W: f32 = THUMB as f32 + 8.0;
const CARD_H: f32 = THUMB as f32 + 8.0 + 20.0;
const GAP: f32 = 6.0;

/// One hair as the window shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct HairRow {
    pub key: FormKey,
    pub label: String,
    pub editor_id: String,
    /// The plugin that adds it, as named on disk.
    pub plugin: String,
    /// Where its model is, as a game path.
    pub model: String,
    pub male: bool,
    pub female: bool,
    /// A beard or other facial hair, not a hairstyle.
    pub facial: bool,
}

/// What reading the load order found.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    pub rows: Vec<HairRow>,
    /// Plugins with hair and how many, in load order.
    pub plugins: Vec<(String, usize)>,
    /// What the patch on disk hides now.
    pub hidden: HashSet<FormKey>,
    /// Plugins that couldn't be read.
    pub problems: Vec<String>,
}

pub enum Job {
    /// Read the load order's hair and the patch's picks.
    Load { paths: Box<GamePaths> },
    /// Draw these, in order, instead of anything not yet drawn from an earlier request.
    Thumbs { paths: Box<GamePaths>, wanted: Vec<(FormKey, String)> },
    /// Draw one hair large, from `camera`; a newer request in the same batch replaces this one.
    Big { id: u64, paths: Box<GamePaths>, model: String, camera: Camera },
    /// Write the patch so that exactly `hidden` is hidden, and switch it on (or remove it).
    Apply { paths: Box<GamePaths>, hidden: Vec<FormKey> },
}

pub enum Reply {
    Loaded(Result<Loaded, String>),
    Thumb { key: FormKey, result: Result<Rgba, String> },
    Big { id: u64, result: Result<Rgba, String> },
    /// The patch was written: what was done, or why not.
    Applied(Result<String, String>),
}

// ------------------------------------------------------------------ worker

/// The worker's own GPU device, with a renderer for the small pictures and one for the big one (each keeps
/// its target and the scene it last drew, so neither is rebuilt when the other runs).
struct HairGpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    thumbs: crate::gpu::Offscreen,
    big: crate::gpu::Offscreen,
}

fn open_gpu() -> Result<HairGpu, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).map_err(|e| format!("no GPU to draw with: {e}"))?;
    let (device, queue) = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|e| format!("the GPU wouldn't open: {e}"))?;
    let (thumbs, big) = (crate::gpu::Offscreen::new(&device, &queue).keeping_textures(), crate::gpu::Offscreen::new(&device, &queue));
    Ok(HairGpu { device, queue, thumbs, big })
}

struct Worker {
    /// The hair drawn large, kept (with its scene, already on the GPU) so turning it doesn't read it again.
    big: Option<(String, crate::gpu::Scene)>,
    catalog: Option<HairCatalog>,
    resolver: Option<(std::path::PathBuf, AssetResolver)>,
    gpu: Option<HairGpu>,
    /// Decoded thumbnail-size textures, shared by the hairs that name them (a pack's styles share a handful).
    textures: skyplug_core::render::TextureCache,
}

impl Worker {
    fn load(&mut self, paths: &GamePaths) -> Result<Loaded, String> {
        self.textures.clear(); // the install may have changed
        let catalog = HairCatalog::for_game(paths).map_err(|e| format!("{e:#}"))?;
        let rows = catalog
            .pickable()
            .filter(|h| h.playable)
            .map(|h| HairRow {
                key: h.key.clone(),
                label: h.label().to_string(),
                editor_id: h.editor_id.clone(),
                plugin: h.defined_in.clone(),
                model: h.model.clone(),
                male: h.male,
                female: h.female,
                facial: h.kind == PartKind::FacialHair,
            })
            .collect();
        let hidden = hidden_by_patch(&paths.data_dir.join(PATCH_NAME)).map_err(|e| format!("{e:#}"))?.into_iter().collect();
        let loaded = Loaded { rows, plugins: catalog.plugins_with_hair(), hidden, problems: catalog.problems().to_vec() };
        self.catalog = Some(catalog);
        Ok(loaded)
    }

    fn resolver(&mut self, paths: &GamePaths) -> Result<&AssetResolver, String> {
        if self.resolver.as_ref().is_none_or(|(data, _)| data != &paths.data_dir) {
            let resolver = AssetResolver::for_game(paths).map_err(|e| format!("{e:#}"))?;
            self.resolver = Some((paths.data_dir.clone(), resolver));
        }
        Ok(&self.resolver.as_ref().expect("set above").1)
    }

    fn gpu(&mut self) -> Result<&mut HairGpu, String> {
        if self.gpu.is_none() {
            self.gpu = Some(open_gpu()?);
        }
        Ok(self.gpu.as_mut().expect("set above"))
    }

    /// A hair's picture: its model with its textures, from the side that shows the most of it, drawn on the GPU.
    fn thumb(&mut self, paths: &GamePaths, model: &str) -> Result<Rgba, String> {
        self.resolver(paths)?;
        let (_, resolver) = self.resolver.as_ref().expect("set above");
        let model = Model::from_asset_cached(resolver, model, THUMB_TEXTURE_SIDE, false, &mut self.textures).map_err(|e| format!("{e:#}"))?.ok_or("the game has no such model")?;
        if model.mesh_count() == 0 {
            return Err(model.warnings.first().cloned().unwrap_or_else(|| "nothing it can draw".into()));
        }
        let scene = crate::gpu::Scene::from_model(&model);
        let g = self.gpu()?;
        let (w, h, pixels) = g.thumbs.rgba(&g.device, &g.queue, &scene, &Camera { azimuth: 30.0, elevation: 5.0, zoom: 1.0 }, [THUMB as u32; 2]);
        Ok(Rgba { width: w as usize, height: h as usize, pixels })
    }

    fn big(&mut self, paths: &GamePaths, model: &str, camera: &Camera) -> Result<Rgba, String> {
        if self.big.as_ref().is_none_or(|(m, _)| m != model) {
            let resolver = self.resolver(paths)?;
            // Loaded for shading: the big picture shows what the normal map does.
            let loaded = Model::from_asset_shaded(resolver, model, BIG_TEXTURE_SIDE).map_err(|e| format!("{e:#}"))?.ok_or("the game has no such model")?;
            if loaded.mesh_count() == 0 {
                return Err(loaded.warnings.first().cloned().unwrap_or_else(|| "nothing it can draw".into()));
            }
            self.big = Some((model.to_string(), crate::gpu::Scene::from_model(&loaded)));
        }
        // The scene is taken out and put back so the GPU context can be borrowed beside it.
        let (name, scene) = self.big.take().expect("set above");
        let drawn = self.gpu().map(|g| g.big.rgba(&g.device, &g.queue, &scene, camera, [BIG as u32; 2]));
        self.big = Some((name, scene));
        let (w, h, pixels) = drawn?;
        Ok(Rgba { width: w as usize, height: h as usize, pixels })
    }

    fn apply(&mut self, paths: &GamePaths, hidden: &[FormKey]) -> Result<String, String> {
        let catalog = self.catalog.as_ref().ok_or("the hair hasn't been read yet")?;
        let patch = paths.data_dir.join(PATCH_NAME);
        catalog.write_patch(hidden, &patch).map_err(|e| format!("{e:#}"))?;
        let on = patch.exists();
        skyplug_core::plugins::apply_changes(&paths.plugins_txt, &[(PATCH_NAME.to_string(), on)]).map_err(|e| format!("{e:#}"))?;
        Ok(if on { format!("{} hidden in {}", hidden.len(), PATCH_NAME) } else { format!("every hair is shown again; {} removed", PATCH_NAME) })
    }
}

fn run(jobs: Receiver<Job>, replies: Sender<Reply>, ctx: egui::Context) {
    let mut worker = Worker { big: None, catalog: None, resolver: None, gpu: None, textures: Default::default() };
    let mut queue: VecDeque<(FormKey, String)> = VecDeque::new();
    let mut paths: Option<GamePaths> = None;
    let send = |reply: Reply| {
        let ok = replies.send(reply).is_ok();
        ctx.request_repaint();
        ok
    };
    loop {
        // Block for a job only when there's no picture left to draw; otherwise take whatever
        // has arrived and carry on, so a new request replaces a stale queue between pictures.
        let mut batch: Vec<Job> = Vec::new();
        if queue.is_empty() {
            match jobs.recv() {
                Ok(j) => batch.push(j),
                Err(_) => return,
            }
        }
        loop {
            match jobs.try_recv() {
                Ok(j) => batch.push(j),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        // Of several big pictures asked for at once (turning it), only the newest is drawn.
        let newest_big = batch.iter().rposition(|j| matches!(j, Job::Big { .. }));
        let batch = batch.into_iter().enumerate().filter(|(i, j)| !matches!(j, Job::Big { .. }) || Some(*i) == newest_big).map(|(_, j)| j);
        for job in batch {
            let id_of_big = if let Job::Big { id, .. } = &job { Some(*id) } else { None };
            let reply = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job {
                Job::Load { paths: p } => {
                    let result = worker.load(&p);
                    paths = Some(*p);
                    Some(Reply::Loaded(result))
                }
                Job::Thumbs { paths: p, wanted } => {
                    queue = wanted.into();
                    paths = Some(*p);
                    None
                }
                Job::Big { id, paths: p, model, camera } => {
                    paths = Some(*p.clone());
                    Some(Reply::Big { id, result: worker.big(&p, &model, &camera) })
                }
                Job::Apply { paths: p, hidden } => Some(Reply::Applied(worker.apply(&p, &hidden))),
            }))
            .unwrap_or_else(|_| match id_of_big {
                Some(id) => Some(Reply::Big { id, result: Err("this model crashed the reader or the renderer".into()) }),
                None => Some(Reply::Applied(Err("reading the hair crashed".into()))),
            });
            if let Some(reply) = reply {
                if !send(reply) {
                    return;
                }
            }
        }
        if let (Some((key, model)), Some(p)) = (queue.pop_front(), paths.as_ref()) {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| worker.thumb(p, &model)))
                .unwrap_or_else(|_| Err("this model crashed the reader or the renderer".into()));
            if !send(Reply::Thumb { key, result }) {
                return;
            }
        }
    }
}

// -------------------------------------------------------------------- view

/// A hair's picture, once there is one.
enum Thumb {
    Ready(egui::TextureHandle),
    Failed(String),
}

/// Which hairs to list: by sex, and hairstyles or facial hair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Show {
    Female,
    Male,
    All,
}

/// A hair drawn large, in a window of its own.
struct Big {
    key: FormKey,
    label: String,
    editor_id: String,
    model: String,
    camera: Camera,
    /// Which request the picture answers; a reply to an older one is dropped.
    id: u64,
    picture: Option<egui::TextureHandle>,
    error: Option<String>,
    loading: bool,
}

pub struct HairPicker {
    jobs: Sender<Job>,
    replies: Receiver<Reply>,
    open: bool,
    /// `None` while reading.
    loaded: Option<Result<Loaded, String>>,
    /// The hairs to hide: what the patch hides now, plus what's been changed since.
    hide: HashSet<FormKey>,
    /// What the patch on disk hides, to tell what's changed.
    applied: HashSet<FormKey>,
    chosen: Option<String>,
    plugin_filter: String,
    filter: String,
    show: Show,
    facial: bool,
    thumbs: HashMap<FormKey, Thumb>,
    /// What the worker was last asked to draw.
    asked: Vec<FormKey>,
    /// The hair drawn large, once one has been double-clicked, and the last request's number.
    big: Option<Big>,
    big_id: u64,
    /// Screenshot mode: the hair to draw large as soon as the list is read.
    open_big: Option<String>,
    applying: bool,
    /// What the last Apply did, or why it couldn't.
    status: Option<Result<String, String>>,
    /// Plugins.txt changed, so the load order shown is out of date.
    plugins_changed: bool,
}

impl HairPicker {
    pub fn start(ctx: &egui::Context) -> Self {
        let (jobs, inbox) = channel();
        let (outbox, replies) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || run(inbox, outbox, ctx));
        HairPicker {
            jobs,
            replies,
            open: false,
            loaded: None,
            hide: HashSet::new(),
            applied: HashSet::new(),
            chosen: None,
            plugin_filter: String::new(),
            filter: String::new(),
            show: Show::Female,
            facial: false,
            thumbs: HashMap::new(),
            asked: Vec::new(),
            big: None,
            big_id: 0,
            open_big: None,
            applying: false,
            status: None,
            plugins_changed: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Reading, drawing or writing is going on.
    pub fn busy(&self) -> bool {
        self.open && (self.loaded.is_none() || self.applying || !self.asked.is_empty() || self.big.as_ref().is_some_and(|b| b.loading))
    }

    /// Whether `Plugins.txt` was changed since this was last asked; asking clears it.
    pub fn take_plugins_changed(&mut self) -> bool {
        std::mem::take(&mut self.plugins_changed)
    }

    /// Screenshot mode and tests: open it on `plugin`, with `filter` typed.
    pub fn show_plugin(&mut self, plugin: &str, filter: &str, paths: &GamePaths) {
        self.open(paths);
        self.chosen = Some(plugin.to_string());
        self.filter = filter.to_string();
    }

    /// Screenshot mode: also draw the hair with this name large.
    pub fn show_big(&mut self, label: &str) {
        self.open_big = Some(label.to_string());
    }

    /// The name of the hair shown large, if one is.
    #[cfg(test)]
    pub fn big_label(&self) -> Option<String> {
        self.big.as_ref().map(|b| b.label.clone())
    }

    /// The large picture has been drawn.
    #[cfg(test)]
    pub fn big_drawn(&self) -> bool {
        self.big.as_ref().is_some_and(|b| b.picture.is_some() && !b.loading)
    }

    /// Whether the chosen plugin's hair is read and every picture asked for has come.
    pub fn is_ready(&self) -> bool {
        matches!(self.loaded, Some(Ok(_))) && self.asked.is_empty() && self.open_big.is_none() && !self.big.as_ref().is_some_and(|b| b.loading)
    }

    /// Open the window and read the load order.
    pub fn open(&mut self, paths: &GamePaths) {
        self.open = true;
        self.loaded = None;
        self.thumbs.clear();
        self.asked.clear();
        self.big = None;
        self.status = None;
        let _ = self.jobs.send(Job::Load { paths: Box::new(paths.clone()) });
    }

    /// How many hairs differ from what the patch on disk does.
    pub fn changes(&self) -> usize {
        self.hide.symmetric_difference(&self.applied).count()
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let replies: Vec<_> = self.replies.try_iter().collect();
        for reply in replies {
            match reply {
                Reply::Loaded(result) => {
                    if let Ok(l) = &result {
                        self.hide = l.hidden.clone();
                        self.applied = l.hidden.clone();
                        if self.chosen.is_none() {
                            // The first mod's hair, not the base game's.
                            self.chosen = l.plugins.iter().rev().find(|(p, _)| !p.eq_ignore_ascii_case("skyrim.esm")).or(l.plugins.first()).map(|(p, _)| p.clone());
                        }
                    }
                    self.loaded = Some(result);
                }
                Reply::Thumb { key, result } => {
                    self.asked.retain(|k| *k != key);
                    let thumb = match result {
                        Ok(image) => {
                            let color = egui::ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.pixels);
                            Thumb::Ready(ctx.load_texture(format!("hair-{}-{:x}", key.plugin, key.id), color, egui::TextureOptions::LINEAR))
                        }
                        Err(e) => Thumb::Failed(e),
                    };
                    self.thumbs.insert(key, thumb);
                }
                Reply::Big { id, result } => {
                    if let Some(big) = self.big.as_mut().filter(|b| b.id == id) {
                        big.loading = false;
                        match result {
                            Ok(image) => {
                                let color = egui::ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.pixels);
                                match &mut big.picture {
                                    Some(texture) => texture.set(color, egui::TextureOptions::LINEAR),
                                    None => big.picture = Some(ctx.load_texture("hair-big", color, egui::TextureOptions::LINEAR)),
                                }
                                big.error = None;
                            }
                            Err(e) => big.error = Some(e),
                        }
                    }
                }
                Reply::Applied(result) => {
                    self.applying = false;
                    if result.is_ok() {
                        self.applied = self.hide.clone();
                        self.plugins_changed = true;
                    }
                    self.status = Some(result);
                }
            }
        }
    }

    /// Draw `hair` large in its own window.
    fn open_big(&mut self, hair: &HairRow, paths: &GamePaths) {
        let camera = Camera { azimuth: 30.0, elevation: 5.0, zoom: 1.0 };
        self.big = Some(Big {
            key: hair.key.clone(),
            label: hair.label.clone(),
            editor_id: hair.editor_id.clone(),
            model: hair.model.clone(),
            camera,
            id: 0,
            picture: None,
            error: None,
            loading: false,
        });
        self.ask_big(paths);
    }

    /// Ask for the big picture again, as the camera is now.
    fn ask_big(&mut self, paths: &GamePaths) {
        self.big_id += 1;
        let Some(big) = &mut self.big else { return };
        big.id = self.big_id;
        if big.model.is_empty() {
            big.error = Some("it has no model".into());
            return;
        }
        big.loading = true;
        let _ = self.jobs.send(Job::Big { id: big.id, paths: Box::new(paths.clone()), model: big.model.clone(), camera: big.camera });
    }

    /// The hair of the chosen plugin that the filters let through.
    fn shown_rows<'a>(&self, loaded: &'a Loaded) -> Vec<&'a HairRow> {
        let needle = self.filter.to_lowercase();
        loaded
            .rows
            .iter()
            .filter(|r| self.chosen.as_deref().is_some_and(|p| r.plugin.eq_ignore_ascii_case(p)))
            .filter(|r| r.facial == self.facial)
            .filter(|r| match self.show {
                Show::Female => r.female,
                Show::Male => r.male,
                Show::All => true,
            })
            .filter(|r| needle.is_empty() || r.label.to_lowercase().contains(&needle) || r.editor_id.to_lowercase().contains(&needle))
            .collect()
    }

    fn apply(&mut self, paths: &GamePaths) {
        if crate::game::is_running() {
            self.status = Some(Err("Skyrim is running, and it rewrites Plugins.txt when it quits. Close it, then apply.".into()));
            return;
        }
        self.applying = true;
        self.status = None;
        let mut hidden: Vec<FormKey> = self.hide.iter().cloned().collect();
        hidden.sort_by(|a, b| (&a.plugin, a.id).cmp(&(&b.plugin, b.id)));
        let _ = self.jobs.send(Job::Apply { paths: Box::new(paths.clone()), hidden });
    }

    /// Draw the window, if it's open.
    pub fn ui(&mut self, ctx: &egui::Context, paths: &GamePaths, colours: &Colours) {
        self.poll(ctx);
        if !self.open {
            return;
        }
        let mut open = true;
        let mut chosen = None;
        let mut wanted: Vec<(FormKey, String)> = Vec::new();
        let mut toggled: Vec<FormKey> = Vec::new();
        let mut enlarge: Option<HairRow> = None;
        let (mut set_shown, mut set_hidden, mut apply, mut revert) = (false, false, false, false);
        egui::Window::new("Hair")
            .id(egui::Id::new("hair-picker"))
            .default_size([900.0, 600.0])
            .default_pos(ctx.content_rect().center() - egui::vec2(450.0, 300.0))
            .collapsible(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Panel::left("hair-plugins").default_size(220.0).resizable(true).show(ui, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.plugin_filter).hint_text("Filter plugins"));
                    ui.add_space(4.0);
                    match &self.loaded {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.weak("Reading the load order…");
                            });
                        }
                        Some(Err(e)) => {
                            ui.colored_label(ui.visuals().error_fg_color, e);
                        }
                        Some(Ok(l)) => {
                            let needle = self.plugin_filter.to_lowercase();
                            egui::ScrollArea::vertical().id_salt("hair-plugin-list").auto_shrink(false).show(ui, |ui| {
                                for (name, n) in l.plugins.iter().rev().filter(|(name, _)| name.to_lowercase().contains(&needle)) {
                                    let on = self.chosen.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(name));
                                    let hidden_here = l.rows.iter().filter(|r| r.plugin.eq_ignore_ascii_case(name) && self.hide.contains(&r.key)).count();
                                    let text = if hidden_here > 0 { format!("{name}  ({n}, {hidden_here} hidden)") } else { format!("{name}  ({n})") };
                                    if ui.add(egui::Button::selectable(on, text).truncate()).clicked() && !on {
                                        chosen = Some(name.clone());
                                    }
                                }
                                for p in &l.problems {
                                    ui.weak(p);
                                }
                            });
                        }
                    }
                });
                // The controls over the grid.
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter by name").desired_width(180.0));
                    for (show, name) in [(Show::Female, "Female"), (Show::Male, "Male"), (Show::All, "Both")] {
                        ui.selectable_value(&mut self.show, show, name);
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.facial, false, "Hair");
                    ui.selectable_value(&mut self.facial, true, "Beards");
                });
                let Some(Ok(loaded)) = &self.loaded else {
                    return;
                };
                let shown = self.shown_rows(loaded);
                let ticked = shown.iter().filter(|r| !self.hide.contains(&r.key)).count();
                ui.horizontal(|ui| {
                    ui.weak(format!("{ticked} of {} shown in game", shown.len()));
                    set_shown = ui.button("Show All").on_hover_text("Tick every hair listed").clicked();
                    set_hidden = ui.button("Hide All").on_hover_text("Untick every hair listed").clicked();
                });
                ui.horizontal(|ui| {
                    let changes = self.changes();
                    let label = if changes == 0 { "Apply".to_string() } else { format!("Apply ({changes})") };
                    apply = ui.add_enabled(changes > 0 && !self.applying, egui::Button::new(label)).on_hover_text(format!("Write {} and switch it on", PATCH_NAME)).clicked();
                    revert = ui.add_enabled(changes > 0, egui::Button::new("Revert")).clicked();
                    if self.applying {
                        ui.spinner();
                    }
                    match &self.status {
                        Some(Ok(done)) => {
                            ui.colored_label(colours.installed, done);
                        }
                        Some(Err(e)) => {
                            ui.colored_label(ui.visuals().error_fg_color, e);
                        }
                        None => {}
                    }
                });
                ui.separator();
                if self.chosen.is_none() {
                    ui.weak("Choose a plugin.");
                    return;
                }
                if shown.is_empty() {
                    ui.weak("No hair matches.");
                    return;
                }
                let cols = (((ui.available_width() + GAP) / (CARD_W + GAP)) as usize).max(1);
                let rows = shown.len().div_ceil(cols);
                egui::ScrollArea::vertical().id_salt("hair-grid").auto_shrink(false).show_rows(ui, CARD_H + GAP, rows, |ui, range| {
                    ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
                    for row in range {
                        ui.horizontal(|ui| {
                            for hair in shown.iter().skip(row * cols).take(cols) {
                                let hidden = self.hide.contains(&hair.key);
                                let thumb = self.thumbs.get(&hair.key);
                                if thumb.is_none() && !hair.model.is_empty() {
                                    wanted.push((hair.key.clone(), hair.model.clone()));
                                }
                                let (clicked, double) = card(ui, hair, thumb, hidden, colours);
                                if clicked {
                                    toggled.push(hair.key.clone());
                                }
                                if double {
                                    enlarge = Some((*hair).clone());
                                }
                            }
                        });
                    }
                });
                // The hairs in these rows to hide or show, for the buttons above.
                let keys: Vec<FormKey> = shown.iter().map(|r| r.key.clone()).collect();
                if set_shown {
                    for k in &keys {
                        self.hide.remove(k);
                    }
                }
                if set_hidden {
                    self.hide.extend(keys);
                }
            });
        if let Some(name) = chosen {
            self.chosen = Some(name);
        }
        if let Some(hair) = enlarge {
            self.open_big(&hair, paths);
        }
        if let (Some(label), Some(Ok(loaded))) = (self.open_big.clone(), &self.loaded) {
            self.open_big = None;
            if let Some(hair) = loaded.rows.iter().find(|r| r.label == label).cloned() {
                self.open_big(&hair, paths);
            }
        }
        self.big_ui(ctx, paths, colours);
        for key in toggled {
            if !self.hide.remove(&key) {
                self.hide.insert(key);
            }
        }
        if revert {
            self.hide = self.applied.clone();
            self.status = None;
        }
        if apply {
            self.apply(paths);
        }
        // Ask for the pictures on screen that don't exist yet, replacing the last request.
        let keys: Vec<FormKey> = wanted.iter().map(|(k, _)| k.clone()).collect();
        if keys != self.asked {
            self.asked = keys;
            let _ = self.jobs.send(Job::Thumbs { paths: Box::new(paths.clone()), wanted });
        }
        self.open = open;
    }
}

impl HairPicker {
    /// The big picture's window: turn it by dragging or with the view buttons, and tick it.
    fn big_ui(&mut self, ctx: &egui::Context, paths: &GamePaths, colours: &Colours) {
        let Some(mut big) = self.big.take() else { return };
        let mut open = true;
        let mut ask = false;
        let mut shown = !self.hide.contains(&big.key);
        let was_shown = shown;
        egui::Window::new(&big.label)
            .id(egui::Id::new("hair-big"))
            .default_size([BIG as f32 + 24.0, BIG as f32 + 90.0])
            .default_pos(ctx.content_rect().center() - egui::vec2(BIG as f32 / 2.0, BIG as f32 / 2.0 + 20.0))
            .collapsible(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (name, azimuth) in VIEWS {
                        if ui.button(name).clicked() {
                            big.camera = Camera { azimuth, elevation: 5.0, zoom: 1.0 };
                            ask = true;
                        }
                    }
                    ui.separator();
                    ui.checkbox(&mut shown, "Shown in game").on_hover_text("Offered on the character creation screen after Apply");
                    if big.loading {
                        ui.spinner();
                    }
                });
                if let Some(e) = &big.error {
                    ui.colored_label(colours.missing, e);
                }
                if let Some(texture) = &big.picture {
                    let picture = ui.add(egui::Image::new(egui::load::SizedTexture::new(texture.id(), egui::Vec2::splat(BIG as f32))).sense(egui::Sense::drag()));
                    if picture.dragged() {
                        let d = picture.drag_delta();
                        big.camera.azimuth = (big.camera.azimuth + d.x * 0.5).rem_euclid(360.0);
                        big.camera.elevation = (big.camera.elevation + d.y * 0.4).clamp(-85.0, 85.0);
                        ask = true;
                    }
                    picture.on_hover_cursor(egui::CursorIcon::Grab);
                }
                ui.weak(format!("{}   {}", big.editor_id, big.model));
            });
        if shown != was_shown {
            if shown {
                self.hide.remove(&big.key);
            } else {
                self.hide.insert(big.key.clone());
            }
        }
        self.big = open.then_some(big);
        if ask {
            self.ask_big(paths);
        }
    }
}

/// One hair's card: its picture, whether it's shown in game, and its name. Whether it was clicked,
/// and whether that click was the second of a double-click (which also toggles it, so a
/// double-click leaves it as it was).
fn card(ui: &mut egui::Ui, hair: &HairRow, thumb: Option<&Thumb>, hidden: bool, colours: &Colours) -> (bool, bool) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(CARD_W, CARD_H), egui::Sense::click());
    if !ui.is_rect_visible(rect) {
        return (false, false);
    }
    let visuals = ui.visuals();
    let fill = if hidden { visuals.faint_bg_color } else { visuals.selection.bg_fill.gamma_multiply(0.35) };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, fill);
    if response.hovered() {
        painter.rect_stroke(rect, 4.0, visuals.widgets.hovered.bg_stroke, egui::StrokeKind::Inside);
    }
    let picture = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::Vec2::splat(THUMB as f32));
    let tint = if hidden { egui::Color32::from_gray(90) } else { egui::Color32::WHITE };
    match thumb {
        Some(Thumb::Ready(texture)) => {
            painter.image(texture.id(), picture, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
        }
        Some(Thumb::Failed(_)) => {
            painter.text(picture.center(), egui::Align2::CENTER_CENTER, "can't draw", egui::FontId::proportional(11.0), colours.missing);
        }
        None => {
            painter.text(picture.center(), egui::Align2::CENTER_CENTER, "…", egui::FontId::proportional(14.0), visuals.weak_text_color());
        }
    }
    // A tick in the corner when it's shown in game.
    let badge = egui::pos2(rect.min.x + 14.0, rect.min.y + 14.0);
    if hidden {
        painter.circle_stroke(badge, 8.0, egui::Stroke::new(1.5, visuals.weak_text_color()));
    } else {
        painter.circle_filled(badge, 8.0, colours.installed);
        let stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
        painter.line_segment([badge + egui::vec2(-4.0, 0.0), badge + egui::vec2(-1.0, 3.0)], stroke);
        painter.line_segment([badge + egui::vec2(-1.0, 3.0), badge + egui::vec2(4.5, -3.5)], stroke);
    }
    let text_color = if hidden { visuals.weak_text_color() } else { visuals.text_color() };
    let label = egui::Rect::from_min_size(egui::pos2(rect.min.x + 4.0, picture.max.y + 2.0), egui::vec2(THUMB as f32, 18.0));
    painter.with_clip_rect(label).text(label.left_center(), egui::Align2::LEFT_CENTER, &hair.label, egui::FontId::proportional(12.0), text_color);
    let mut hover = format!("{}\n{}\n{}", hair.label, hair.editor_id, hair.model);
    if let Some(Thumb::Failed(why)) = thumb {
        hover.push_str(&format!("\n\n{why}"));
    }
    let (clicked, double) = (response.clicked(), response.double_clicked());
    response.on_hover_text(hover);
    (clicked, double)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn key(id: u32) -> FormKey {
        FormKey { plugin: "hairs.esp".into(), id }
    }

    fn row(id: u32, label: &str, male: bool, female: bool, facial: bool) -> HairRow {
        HairRow { key: key(id), label: label.into(), editor_id: format!("Ed{id}"), plugin: "Hairs.esp".into(), model: "meshes/x.nif".into(), male, female, facial }
    }

    /// The preview fixture's game folder with a plugin of four head parts (a woman's hair with an
    /// extra part, a man's hair, a beard), enabled after the fixture's own plugin.
    pub(crate) fn hair_install(name: &str) -> GamePaths {
        let paths = crate::preview::tests::fixture_install(name);
        std::fs::write(paths.data_dir.join("FixtureHairs.esp"), include_bytes!("../tests/fixtures/FixtureHairs.esp")).unwrap();
        std::fs::write(&paths.plugins_txt, "*Fixture.esp\n*FixtureHairs.esp\n").unwrap();
        paths
    }

    fn next(replies: &Receiver<Reply>) -> Reply {
        replies.recv_timeout(std::time::Duration::from_secs(10)).expect("the worker answers")
    }

    #[test]
    fn the_worker_reads_draws_and_writes_the_patch() {
        let paths = hair_install("hair_worker");
        let (jobs, inbox) = channel();
        let (outbox, replies) = channel();
        std::thread::spawn(move || run(inbox, outbox, egui::Context::default()));
        let load = |jobs: &Sender<Job>| jobs.send(Job::Load { paths: Box::new(paths.clone()) }).unwrap();

        load(&jobs);
        let Reply::Loaded(Ok(l)) = next(&replies) else { panic!("expected the hair") };
        let names: Vec<&str> = l.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(names, ["Zod", "Zac", "Stubble"], "the extra part isn't listed");
        assert_eq!(l.plugins, [("FixtureHairs.esp".to_string(), 3)]);
        assert!(l.hidden.is_empty() && l.problems.is_empty(), "{:?}", l.problems);
        let zod = l.rows[0].clone();
        assert!(zod.female && !zod.male && !zod.facial && l.rows[2].facial);
        assert_eq!(zod.model, "meshes/fixture/quad.nif");

        // A picture is drawn from the model.
        jobs.send(Job::Thumbs { paths: Box::new(paths.clone()), wanted: vec![(zod.key.clone(), zod.model.clone())] }).unwrap();
        let Reply::Thumb { key, result: Ok(image) } = next(&replies) else { panic!("expected a picture") };
        assert_eq!(key, zod.key);
        assert_eq!((image.width, image.height), (THUMB, THUMB));
        assert!(image.pixels.chunks_exact(4).any(|p| p[3] == 255), "something is drawn");
        jobs.send(Job::Thumbs { paths: Box::new(paths.clone()), wanted: vec![(l.rows[1].key.clone(), "meshes/fixture/gone.nif".into())] }).unwrap();
        let Reply::Thumb { result: Err(why), .. } = next(&replies) else { panic!("expected a failure") };
        assert!(why.contains("no such model"), "{why}");

        // Drawn large, from where it's asked: the same hair seen from the back is a different picture.
        let front = Camera { azimuth: 0.0, elevation: 5.0, zoom: 1.0 };
        let big = |id: u64, camera: Camera| Job::Big { id, paths: Box::new(paths.clone()), model: zod.model.clone(), camera };
        jobs.send(big(7, front)).unwrap();
        let Reply::Big { id: 7, result: Ok(a) } = next(&replies) else { panic!("expected a big picture") };
        assert_eq!((a.width, a.height), (BIG, BIG));
        // Two asked at once: only the newest is drawn.
        jobs.send(big(8, front)).unwrap();
        jobs.send(big(9, Camera { azimuth: 180.0, ..front })).unwrap();
        let Reply::Big { id, result: Ok(b) } = next(&replies) else { panic!("expected a big picture") };
        assert!(id == 9 || id == 8, "{id}");
        if id == 9 {
            assert!(replies.recv_timeout(std::time::Duration::from_millis(200)).is_err(), "nothing drawn for 8");
        }
        assert_eq!((b.width, b.height), (BIG, BIG));

        // Hiding writes the patch and switches it on.
        jobs.send(Job::Apply { paths: Box::new(paths.clone()), hidden: vec![zod.key.clone()] }).unwrap();
        let Reply::Applied(Ok(done)) = next(&replies) else { panic!("expected it applied") };
        assert!(done.starts_with("1 hidden"), "{done}");
        let patch = paths.data_dir.join(PATCH_NAME);
        assert_eq!(hidden_by_patch(&patch).unwrap(), std::slice::from_ref(&zod.key));
        let txt = std::fs::read_to_string(&paths.plugins_txt).unwrap();
        assert!(txt.lines().any(|l| l == format!("*{PATCH_NAME}")), "{txt}");
        assert!(txt.find("FixtureHairs.esp").unwrap() < txt.find(PATCH_NAME).unwrap(), "after what it patches: {txt}");

        // Read again, the patch's picks come back and the hairs are the mod's, not the patch's.
        load(&jobs);
        let Reply::Loaded(Ok(l)) = next(&replies) else { panic!("expected the hair") };
        assert_eq!(l.hidden, [zod.key.clone()].into());
        assert_eq!(l.rows.len(), 3);
        assert!(l.rows.iter().all(|r| r.plugin == "FixtureHairs.esp"));

        // Showing everything again removes the patch and switches it off.
        jobs.send(Job::Apply { paths: Box::new(paths.clone()), hidden: vec![] }).unwrap();
        let Reply::Applied(Ok(done)) = next(&replies) else { panic!("expected it applied") };
        assert!(done.contains("shown again"), "{done}");
        assert!(!patch.exists());
        let txt = std::fs::read_to_string(&paths.plugins_txt).unwrap();
        assert!(txt.lines().any(|l| l == PATCH_NAME), "no longer enabled: {txt}");
    }

    fn picker() -> HairPicker {
        HairPicker::start(&egui::Context::default())
    }

    #[test]
    fn changes_count_what_differs_from_the_patch_on_disk() {
        let mut p = picker();
        p.applied = [key(1), key(2)].into();
        p.hide = [key(1), key(2)].into();
        assert_eq!(p.changes(), 0);
        p.hide.remove(&key(2)); // shown again
        p.hide.insert(key(3)); // newly hidden
        assert_eq!(p.changes(), 2);
        p.hide = p.applied.clone();
        assert_eq!(p.changes(), 0);
    }

    #[test]
    fn the_filters_pick_by_plugin_sex_kind_and_text() {
        let mut p = picker();
        let mut other = row(9, "Other", true, true, false);
        other.plugin = "Elsewhere.esp".into();
        let loaded = Loaded {
            rows: vec![row(1, "Zod", false, true, false), row(2, "Zac", true, false, false), row(3, "Beard", true, false, true), row(4, "Unisex", true, true, false), other],
            plugins: vec![("Hairs.esp".into(), 4), ("Elsewhere.esp".into(), 1)],
            ..Loaded::default()
        };
        p.chosen = Some("hairs.esp".into());
        let labels = |p: &HairPicker| p.shown_rows(&loaded).iter().map(|r| r.label.clone()).collect::<Vec<_>>();
        assert_eq!(labels(&p), ["Zod", "Unisex"], "women's hairstyles of the chosen plugin");
        p.show = Show::Male;
        assert_eq!(labels(&p), ["Zac", "Unisex"]);
        p.show = Show::All;
        assert_eq!(labels(&p), ["Zod", "Zac", "Unisex"], "no beards among hairstyles");
        p.facial = true;
        assert_eq!(labels(&p), ["Beard"]);
        p.facial = false;
        p.filter = "ZA".into();
        assert_eq!(labels(&p), ["Zac"], "any case, and by editor ID too");
        p.filter = "ed4".into();
        assert_eq!(labels(&p), ["Unisex"]);
    }
    /// Times the hair pictures on the GPU against the old CPU renderer on a real install, and saves the GPU
    /// ones as a contact sheet. Reads only. Same variables as `gpu_shot` in preview.rs (`ARCANAEUM_SHOT_DATA`,
    /// `_PLUGINS`, `_OUT`), plus `ARCANAEUM_SHOT_N` (how many hairs, default 24).
    #[test]
    #[ignore = "needs a real install and a GPU"]
    fn hair_thumbs_on_the_gpu_against_the_cpu() {
        use skyplug_core::render::{render, Options};
        use std::time::Instant;
        let var = |k: &str| std::env::var(k).ok();
        let (Some(data), Some(plugins), Some(out)) = (var("ARCANAEUM_SHOT_DATA"), var("ARCANAEUM_SHOT_PLUGINS"), var("ARCANAEUM_SHOT_OUT")) else {
            panic!("set ARCANAEUM_SHOT_DATA, _PLUGINS and _OUT");
        };
        let n: usize = var("ARCANAEUM_SHOT_N").and_then(|s| s.parse().ok()).unwrap_or(24);
        let data = std::path::PathBuf::from(data);
        let game = data.parent().unwrap().to_path_buf();
        let paths = GamePaths {
            data_dir: data,
            plugins_txt: std::path::PathBuf::from(plugins),
            ccc_path: game.join("Skyrim.ccc"),
            saves_dir: game.join("no-saves/Saves"),
            edition: skyplug_core::steam::Edition::Gog,
            prefix: None,
        };
        let mut worker = Worker { big: None, catalog: None, resolver: None, gpu: None, textures: Default::default() };
        let loaded = worker.load(&paths).expect("the hair reads");
        let models: Vec<String> = loaded.rows.iter().filter(|r| !r.model.is_empty()).map(|r| r.model.clone()).take(n).collect();
        println!("{} hairs in the load order, timing {}", loaded.rows.len(), models.len());
        // Loading alone, so the drawing can be told apart from it.
        let started = Instant::now();
        let resolver = worker.resolver(&paths).unwrap();
        let mut loaded_models = Vec::new();
        for m in &models {
            if let Ok(Some(model)) = Model::from_asset(resolver, m, THUMB_TEXTURE_SIDE) {
                loaded_models.push(model);
            }
        }
        println!("reading {} models and decoding their textures: {:?} ({:?} each)", loaded_models.len(), started.elapsed(), started.elapsed() / loaded_models.len().max(1) as u32);
        // How many of the textures are the same file as another hair's.
        {
            let all: Vec<&str> = loaded_models.iter().flat_map(|m| m.textures.iter().map(|t| t.path.as_str())).collect();
            let unique: std::collections::HashSet<&str> = all.iter().copied().collect();
            println!("  textures named by those models: {} in all, {} different", all.len(), unique.len());
        }
        // Where that goes: the model file alone, then the rest (finding and decoding the textures).
        let started = Instant::now();
        for m in &models {
            if let Ok(Some(bytes)) = resolver.read_asset(m) {
                let _ = skyplug_core::nif::parse(&bytes);
            }
        }
        println!("  of which reading and parsing the NIFs: {:?} ({:?} each)", started.elapsed(), started.elapsed() / models.len().max(1) as u32);
        let started = Instant::now();
        let options = Options { width: THUMB, height: THUMB, supersample: 2, background: [0, 0, 0, 0], ..Options::default() };
        for m in &loaded_models {
            let _ = render(m, &Camera { azimuth: 30.0, elevation: 5.0, zoom: 1.0 }, &options);
        }
        println!("CPU: drawing them: {:?} ({:?} each)", started.elapsed(), started.elapsed() / loaded_models.len().max(1) as u32);
        // The GPU path as the worker runs it (model read included), after one to open the device.
        worker.thumb(&paths, &models[0]).expect("the first thumbnail");
        let started = Instant::now();
        let mut pics = Vec::new();
        for m in &models {
            if let Ok(p) = worker.thumb(&paths, m) {
                pics.push(p);
            }
        }
        println!("GPU: whole worker path (read + draw) for {}: {:?} ({:?} each)", pics.len(), started.elapsed(), started.elapsed() / pics.len().max(1) as u32);
        let cols = 6usize;
        let rows = pics.len().div_ceil(cols);
        let mut sheet = image::RgbaImage::from_pixel((cols * THUMB) as u32, (rows * THUMB) as u32, image::Rgba([38, 41, 50, 255]));
        for (i, p) in pics.iter().enumerate() {
            let img = image::RgbaImage::from_raw(p.width as u32, p.height as u32, p.pixels.clone()).unwrap();
            let mut tile = image::RgbaImage::from_pixel(p.width as u32, p.height as u32, image::Rgba([38, 41, 50, 255]));
            for (x, y, px) in img.enumerate_pixels() {
                let a = px[3] as f32 / 255.0;
                let bg = tile.get_pixel(x, y).0;
                tile.put_pixel(x, y, image::Rgba([0, 1, 2].map(|k| (px[k] as f32 * a + bg[k] as f32 * (1.0 - a)) as u8).into_iter().chain([255]).collect::<Vec<_>>().try_into().unwrap()));
            }
            image::imageops::replace(&mut sheet, &tile, ((i % cols) * THUMB) as u32, ((i / cols) * THUMB) as u32);
        }
        sheet.save(&out).expect("write the PNG");
    }
}
