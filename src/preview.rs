//! Previewing models (ADR-0009): a view that draws a Skyrim model with the
//! textures the game would use, and says where each texture comes from.
//!
//! Reading models, decoding textures and drawing all happen on a worker
//! thread of their own (ADR-0002), driven by [`Job`]s and answering with
//! [`Reply`]s. Orbiting asks for a new picture on every frame, so the worker
//! keeps only the newest picture asked for. It also watches the files a model
//! uses, and says when the file the game would use for one of them changes,
//! so a recoloured texture shows up as it is saved.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, SystemTime};

use skyplug_core::armour::{self, AltTexture, ArmourCatalog, BodySex};
use skyplug_core::assets::{AssetResolver, Source};
use skyplug_core::dds::Rgba;
use skyplug_core::nif::Skeleton;
use skyplug_core::render::{self, Camera, Model, Options, PoseReport, TextureResult, TextureStatus};
use skyplug_core::GamePaths;

use crate::table::Colours;

/// Plugins that have armour records, with how many, and what couldn't be read.
type PluginList = (Vec<(String, usize)>, Vec<String>);

/// How often the worker looks for changed files.
const POLL: Duration = Duration::from_millis(1000);
/// Textures are decoded at a mip level of about this many pixels a side.
const MAX_TEXTURE_SIDE: usize = 1024;

/// One model to draw: a game path or a file on disk, and any texture swaps an armour that wears
/// it applies (an armour's Tartan variant is the same model with another texture set).
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub path: String,
    pub alts: Vec<AltTexture>,
}

impl From<&str> for Item {
    fn from(path: &str) -> Self {
        Item { path: path.to_string(), alts: Vec::new() }
    }
}

impl Item {
    /// The file name, with a mark if it has texture swaps.
    fn label(&self) -> String {
        let name = self.path.rsplit(['/', '\\']).next().unwrap_or(&self.path);
        if self.alts.is_empty() { name.to_string() } else { format!("{name} (alt textures)") }
    }
}

/// One thing added to the preview: a model typed or picked as a file, or an armour, which is
/// worn as other models on a woman and on a man. Switching the body swaps the models without
/// reading anything again.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub female: Vec<Item>,
    pub male: Vec<Item>,
}

impl Piece {
    /// A model file: the same on either body.
    pub fn file(item: Item) -> Self {
        Piece { female: vec![item.clone()], male: vec![item] }
    }

    /// The models to draw on `sex`'s body.
    pub fn items(&self, sex: BodySex) -> &[Item] {
        match sex {
            BodySex::Female => &self.female,
            BodySex::Male => &self.male,
        }
    }
}

/// What to draw: models, as game paths or files on disk, and whether to put
/// the installed body under them.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub items: Vec<Item>,
    pub body: bool,
    /// Pose skinned models on the game's skeleton, as the game does; off draws them as stored.
    pub pose: bool,
    /// The body to put under the models, and the skeleton to pose them on.
    pub sex: BodySex,
}

/// How to draw it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewParams {
    pub camera: Camera,
    pub width: usize,
    pub height: usize,
    pub supersample: usize,
}

pub enum Job {
    /// Read the models. `fresh` starts over with the archives too, instead of
    /// only looking at `Data` again.
    Load { id: u64, paths: Box<GamePaths>, request: Request, fresh: bool },
    /// Draw the models loaded with this `id`; dropped if another load has
    /// happened since.
    Render { id: u64, view: ViewParams },
    /// Read the load order's armours (the catalog the picker lists from).
    Plugins { paths: Box<GamePaths>, fresh: bool },
    /// The armours of one plugin; `sex` is whose models are checked for what the game won't draw.
    Armours { plugin: String, sex: BodySex },
    /// What the character of a save wears, as the models the load order has for `sex`.
    Outfit { path: PathBuf, paths: Box<GamePaths>, sex: BodySex },
}

/// What a save's character wears, ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Dressed {
    pub items: Vec<Item>,
    /// What the picture leaves out and why.
    pub notes: Vec<String>,
}

/// One armour as the picker shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmourRow {
    pub label: String,
    pub editor_id: String,
    pub slots: String,
    /// The models it's worn as by women and by men, with their texture swaps.
    pub female: Vec<Item>,
    pub male: Vec<Item>,
    /// For an armour the plugin only changes: the plugin that adds it.
    pub changes: Option<String>,
    /// What the game won't draw of its models (for the body asked for): a shape whose skin
    /// partition is on a slot the armature isn't worn on.
    pub problems: Vec<String>,
}

impl ArmourRow {
    /// The models worn on `sex`'s body.
    pub fn models(&self, sex: BodySex) -> &[Item] {
        match sex {
            BodySex::Female => &self.female,
            BodySex::Male => &self.male,
        }
    }

    /// This armour as something to add to the preview.
    pub fn piece(&self) -> Piece {
        Piece { female: self.female.clone(), male: self.male.clone() }
    }
}

/// What loading a request found.
#[derive(Debug, Clone)]
pub struct Loaded {
    pub meshes: usize,
    /// What posing on the skeleton did, or `None` if it was off or the install has no skeleton.
    pub pose: Option<PoseReport>,
    pub textures: Vec<TextureStatus>,
    /// Problems worth showing: models that couldn't be read, and what the
    /// model reader and the texture loader complained about.
    pub warnings: Vec<String>,
}

pub enum Reply {
    Loaded { id: u64, result: Result<Loaded, String> },
    Image { id: u64, view: ViewParams, image: Rgba },
    /// A file the game would use for something the model needs has changed.
    Changed { id: u64 },
    /// Plugins with armour records and how many; with what couldn't be read.
    Plugins { result: Result<PluginList, String> },
    Armours { plugin: String, result: Result<Vec<ArmourRow>, String> },
    Outfit { path: PathBuf, result: Result<Dressed, String> },
}

// ------------------------------------------------------------------ worker

/// A file the model depends on: a game asset path (the winner can change) or
/// a file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Watch {
    Asset(String),
    File(PathBuf),
}

/// Which file wins for a watched path and when it was last changed.
type Print = Option<(Source, Option<SystemTime>)>;

struct Current {
    id: u64,
    model: Model,
    watched: Vec<(Watch, Print)>,
}

struct Worker {
    poll: Duration,
    /// The game's files, and its female and male skeletons (read once with them).
    resolver: Option<(PathBuf, AssetResolver, [Option<Skeleton>; 2])>,
    current: Option<Current>,
    catalog: Option<ArmourCatalog>,
}

/// A model from the game's files with an armour's texture swaps applied, textures resolved.
fn read_asset(resolver: &AssetResolver, path: &str, alts: &[AltTexture]) -> Result<Option<Model>, String> {
    let Some(bytes) = resolver.read_asset(path).map_err(|e| format!("{e:#}"))? else { return Ok(None) };
    let mut nif = skyplug_core::nif::parse(&bytes).map_err(|e| format!("{e:#}"))?;
    armour::apply_alternate_textures(&mut nif, alts);
    Ok(Some(Model::load(resolver, &nif, MAX_TEXTURE_SIDE)))
}

/// A model file on disk, with its textures resolved through the game's files.
fn read_file(resolver: &AssetResolver, path: &Path) -> Result<Model, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let nif = skyplug_core::nif::parse(&bytes).map_err(|e| format!("{e:#}"))?;
    Ok(Model::load(resolver, &nif, MAX_TEXTURE_SIDE))
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// A model set as items: each model with the texture swaps its armature applies.
fn items_of(set: armour::ModelSet) -> Vec<Item> {
    set.models.into_iter().zip(set.alt_textures).map(|(path, alts)| Item { path, alts }).collect()
}

impl Worker {
    fn resolver(&mut self, paths: &GamePaths, fresh: bool) -> Result<(&AssetResolver, &[Option<Skeleton>; 2]), String> {
        let stale = fresh || self.resolver.as_ref().is_none_or(|(data, _, _)| data != &paths.data_dir);
        if stale {
            let resolver = AssetResolver::for_game(paths).map_err(|e| format!("{e:#}"))?;
            let skeletons = [render::load_skeleton_for(&resolver, BodySex::Female), render::load_skeleton_for(&resolver, BodySex::Male)];
            self.resolver = Some((paths.data_dir.clone(), resolver, skeletons));
        } else if let Some((_, r, _)) = &self.resolver {
            r.clear_cache();
        }
        let (_, resolver, skeletons) = self.resolver.as_ref().expect("set above");
        Ok((resolver, skeletons))
    }

    fn print(resolver: &AssetResolver, watch: &Watch) -> Print {
        match watch {
            Watch::File(path) => modified(path).map(|t| (Source::Loose(path.clone()), Some(t))),
            Watch::Asset(name) => resolver.resolve(name).map(|r| {
                let when = match &r.winner {
                    Source::Loose(file) => modified(file),
                    Source::Archive { .. } => None,
                };
                (r.winner, when)
            }),
        }
    }

    fn load(&mut self, id: u64, paths: &GamePaths, request: &Request, fresh: bool) -> Result<Loaded, String> {
        let (resolver, skeletons) = self.resolver(paths, fresh)?;
        let skeleton = skeletons[request.sex as usize].as_ref();
        let mut model = Model::default();
        let mut problems = Vec::new();
        let mut watched = Vec::new();
        for item in &request.items {
            let on_disk = Path::new(&item.path).is_file();
            let loaded = if on_disk {
                read_file(resolver, Path::new(&item.path)).map(Some)
            } else {
                read_asset(resolver, &item.path, &item.alts)
            };
            match loaded {
                Ok(Some(m)) => model.merge(m),
                Ok(None) => problems.push(format!("{}: the game has no such file", item.path)),
                Err(e) => problems.push(format!("{}: {e}", item.path)),
            }
            watched.push(if on_disk { Watch::File(PathBuf::from(&item.path)) } else { Watch::Asset(item.path.clone()) });
        }
        if request.body {
            for asset in request.sex.body() {
                if let Ok(Some(body)) = Model::from_asset(resolver, asset, MAX_TEXTURE_SIDE) {
                    model.merge_underlay(body);
                    watched.push(Watch::Asset(asset.to_string()));
                }
            }
        }
        if model.mesh_count() == 0 {
            return Err(if problems.is_empty() { "nothing to draw: no meshes it can read".into() } else { problems.join("\n") });
        }
        let pose = if request.pose { skeleton.map(|sk| model.pose(sk)) } else { None };
        for t in &model.textures {
            let w = Watch::Asset(t.path.clone());
            if !watched.contains(&w) {
                watched.push(w);
            }
        }
        let watched = watched
            .into_iter()
            .map(|w| {
                let print = Self::print(resolver, &w);
                (w, print)
            })
            .collect();
        let loaded = Loaded {
            meshes: model.mesh_count(),
            pose,
            textures: model.textures.clone(),
            warnings: problems.into_iter().chain(model.warnings.iter().cloned()).collect(),
        };
        self.current = Some(Current { id, model, watched });
        Ok(loaded)
    }

    fn plugins(&mut self, paths: &GamePaths, fresh: bool) -> Result<PluginList, String> {
        self.resolver(paths, fresh)?;
        let catalog = ArmourCatalog::for_game(paths).map_err(|e| format!("{e:#}"))?;
        let result = (catalog.plugins_with_armours(), catalog.problems().to_vec());
        self.catalog = Some(catalog);
        Ok(result)
    }

    fn armours(&self, plugin: &str, sex: BodySex) -> Result<Vec<ArmourRow>, String> {
        let catalog = self.catalog.as_ref().ok_or("the armours haven't been read yet")?;
        let resolver = self.resolver.as_ref().map(|(_, r, _)| r);
        let armours = catalog.armours_in(plugin, resolver).map_err(|e| format!("{e:#}"))?;
        Ok(armours
            .into_iter()
            .map(|a| ArmourRow {
                label: a.label().to_string(),
                slots: a.slot_names().join(", "),
                changes: a.is_override().then(|| a.defined_in.clone()),
                problems: resolver.map(|r| a.visibility_problems(sex, r)).unwrap_or_default(),
                editor_id: a.editor_id,
                female: items_of(a.female),
                male: items_of(a.male),
            })
            .collect())
    }

    /// The models of what the character in a save wears. Reads the armours too if nothing has yet.
    fn outfit(&mut self, path: &Path, paths: &GamePaths, sex: BodySex) -> Result<Dressed, String> {
        let worn = skyplug_core::outfit::read_outfit_from_file(path).map_err(|e| format!("{e:#}"))?;
        self.resolver(paths, false)?;
        if self.catalog.is_none() {
            self.catalog = Some(ArmourCatalog::for_game(paths).map_err(|e| format!("{e:#}"))?);
        }
        let catalog = self.catalog.as_ref().ok_or("the armours haven't been read yet")?;
        let resolver = self.resolver.as_ref().map(|(_, r, _)| r);
        let keys: Vec<_> = worn.worn.iter().map(|w| w.key.clone()).collect();
        let armours = catalog.armours_for(&keys, resolver);
        let mut notes = Vec::new();
        if worn.skipped > 0 {
            notes.push(format!("Made in game and not shown: {} (enchanted or renamed copies)", worn.skipped));
        }
        let others = keys.len() - armours.len();
        if others > 0 {
            notes.push(format!("Worn but not armour, so not shown: {others} (a weapon or a torch)"));
        }
        let mut items: Vec<Item> = Vec::new();
        for a in armours {
            let label = a.label().to_string();
            let set = if sex == BodySex::Male { a.male } else { a.female };
            if set.models.is_empty() {
                notes.push(format!("{label} has no model to show"));
            }
            for item in items_of(set) {
                if !items.contains(&item) {
                    items.push(item);
                }
            }
        }
        if items.is_empty() {
            notes.push("nothing the character wears has a model to show".to_string());
        }
        Ok(Dressed { items, notes })
    }

    /// The id of the loaded model if a file it uses has changed since last looked.
    fn changed(&mut self) -> Option<u64> {
        let (_, resolver, _) = self.resolver.as_ref()?;
        let current = self.current.as_mut()?;
        resolver.clear_cache();
        let mut any = false;
        for (watch, print) in &mut current.watched {
            let now = Self::print(resolver, watch);
            if now != *print {
                *print = now;
                any = true;
            }
        }
        any.then_some(current.id)
    }
}

/// Keep only the newest `Render` among `jobs`, everything else in order.
fn coalesce(jobs: Vec<Job>) -> Vec<Job> {
    let last = jobs.iter().rposition(|j| matches!(j, Job::Render { .. }));
    jobs.into_iter()
        .enumerate()
        .filter(|(i, j)| !matches!(j, Job::Render { .. }) || Some(*i) == last)
        .map(|(_, j)| j)
        .collect()
}

fn run(jobs: Receiver<Job>, replies: Sender<Reply>, ctx: egui::Context, poll: Duration) {
    let mut worker = Worker { poll, resolver: None, current: None, catalog: None };
    loop {
        let mut batch = match jobs.recv_timeout(worker.poll) {
            Ok(job) => vec![job],
            Err(RecvTimeoutError::Timeout) => {
                if let Some(id) = worker.changed() {
                    if replies.send(Reply::Changed { id }).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => return,
        };
        batch.extend(jobs.try_iter());
        for job in coalesce(batch) {
            let crashed = "this model crashed the reader or the renderer".to_string();
            let on_panic = match &job {
                Job::Load { id, .. } | Job::Render { id, .. } => Reply::Loaded { id: *id, result: Err(crashed) },
                Job::Plugins { .. } => Reply::Plugins { result: Err("reading the plugins crashed".into()) },
                Job::Armours { plugin, .. } => Reply::Armours { plugin: plugin.clone(), result: Err("reading the armours crashed".into()) },
                Job::Outfit { path, .. } => Reply::Outfit { path: path.clone(), result: Err("reading the outfit crashed".into()) },
            };
            // A model that panics the reader or the renderer is an error to show, not the end of the worker.
            let reply = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job {
                Job::Load { id, paths, request, fresh } => {
                    Some(Reply::Loaded { id, result: worker.load(id, &paths, &request, fresh) })
                }
                Job::Render { id, view } => {
                    let current = worker.current.as_ref().filter(|c| c.id == id)?;
                    let options = Options {
                        width: view.width,
                        height: view.height,
                        supersample: view.supersample,
                        background: [0, 0, 0, 0],
                        ..Options::default()
                    };
                    Some(Reply::Image { id, view, image: render::render(&current.model, &view.camera, &options) })
                }
                Job::Plugins { paths, fresh } => Some(Reply::Plugins { result: worker.plugins(&paths, fresh) }),
                Job::Armours { plugin, sex } => {
                    let result = worker.armours(&plugin, sex);
                    Some(Reply::Armours { plugin, result })
                }
                Job::Outfit { path, paths, sex } => {
                    let result = worker.outfit(&path, &paths, sex);
                    Some(Reply::Outfit { path, result })
                }
            }));
            let reply = match reply {
                Ok(Some(reply)) => reply,
                Ok(None) => continue,
                Err(_) => {
                    worker.current = None;
                    on_panic
                }
            };
            if replies.send(reply).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    }
}

// -------------------------------------------------------------------- view

/// The preview's state and drawing: what to show, the camera, and the last
/// picture the worker made.
pub struct Preview {
    jobs: Sender<Job>,
    replies: Receiver<Reply>,
    items: Vec<Piece>,
    body: bool,
    /// Whose body, models and skeleton are drawn.
    sex: BodySex,
    /// The sex is the character's, from the save the outfit came from, and can't be changed
    /// until that outfit is cleared.
    locked: bool,
    /// Pose skinned models on the game's skeleton (on unless turned off).
    pose: bool,
    /// The text being typed to add a model.
    input: String,
    /// Which load the worker's answers belong to.
    id: u64,
    loading: bool,
    loaded: Option<Result<Loaded, String>>,
    camera: Camera,
    picture: Option<egui::TextureHandle>,
    drawn: Option<ViewParams>,
    in_flight: Option<ViewParams>,
    /// The armour picker, while its window is open.
    picker: Option<Picker>,
    /// The save whose outfit is being read.
    dressing: Option<PathBuf>,
    /// What to tell about the models: what an outfit leaves out, or why it couldn't be read.
    notices: Vec<String>,
}

/// The armour picker's state: plugins on the left, the chosen one's armours on the right.
#[derive(Default)]
struct Picker {
    /// Plugins with armour records and how many, and what couldn't be read; `None` while reading.
    plugins: Option<Result<PluginList, String>>,
    plugin_filter: String,
    chosen: Option<String>,
    armours: Option<Result<Vec<ArmourRow>, String>>,
    armour_filter: String,
}

const VIEWS: [(&str, f32); 5] = [("Front", 0.0), ("¾", 35.0), ("Right", 90.0), ("Back", 180.0), ("Left", 270.0)];

fn slot_name(slot: usize) -> String {
    ["diffuse", "normal", "glow / skin tint", "height", "environment", "environment mask", "subsurface", "specular"]
        .get(slot)
        .map_or_else(|| format!("slot {slot}"), |s| (*s).to_string())
}

/// `meshes/a/b.nif` for a file picked under `data_dir`, else the path as is.
fn game_path(data_dir: &Path, picked: &Path) -> String {
    match picked.strip_prefix(data_dir) {
        Ok(rel) => rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"),
        Err(_) => picked.display().to_string(),
    }
}

impl Preview {
    pub fn start(ctx: &egui::Context) -> Self {
        Self::start_polling(ctx, POLL)
    }

    fn start_polling(ctx: &egui::Context, poll: Duration) -> Self {
        let (jobs, inbox) = channel();
        let (outbox, replies) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || run(inbox, outbox, ctx, poll));
        Preview {
            jobs,
            replies,
            items: Vec::new(),
            body: false,
            sex: BodySex::Female,
            locked: false,
            pose: true,
            input: String::new(),
            id: 0,
            loading: false,
            loaded: None,
            camera: Camera::default(),
            picture: None,
            drawn: None,
            in_flight: None,
            picker: None,
            dressing: None,
            notices: Vec::new(),
        }
    }

    /// Anything being read or drawn, for the status bar's spinner.
    pub fn busy(&self) -> bool {
        self.loading || self.in_flight.is_some() || self.dressing.is_some()
    }

    /// A picture of the current models is on screen.
    pub fn is_drawn(&self) -> bool {
        self.picture.is_some() && self.drawn.is_some() && !self.busy()
    }

    /// The models being shown.
    #[cfg(test)]
    pub fn items(&self) -> Vec<&str> {
        self.items.iter().flat_map(|p| p.items(self.sex)).map(|i| i.path.as_str()).collect()
    }

    /// Where the camera is.
    #[cfg(test)]
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// Start over, for another copy of the game.
    pub fn reset(&mut self) {
        self.items.clear();
        self.input.clear();
        self.body = false;
        self.sex = BodySex::Female;
        self.locked = false;
        self.pose = true;
        self.id += 1;
        self.loading = false;
        self.loaded = None;
        self.picture = None;
        self.drawn = None;
        self.in_flight = None;
        self.camera = Camera::default();
        self.picker = None;
        self.dressing = None;
        self.notices.clear();
    }

    /// Set what to show, for screenshot mode.
    pub fn show(&mut self, items: Vec<String>, body: bool, paths: &GamePaths) {
        self.items = items.iter().map(|i| Piece::file(Item::from(i.as_str()))).collect();
        self.body = body;
        self.load(paths, false);
    }

    /// Add a model (a game path or a file) and load.
    pub fn add(&mut self, item: &str, paths: &GamePaths) {
        let item = item.trim();
        let piece = Piece::file(Item::from(item));
        if !item.is_empty() && !self.items.contains(&piece) {
            self.items.push(piece);
            self.load(paths, false);
        }
    }

    /// Show what the character in a save wears: the worker reads the save and the armours, then
    /// what it finds replaces the models, with the body under them. The body is the character's
    /// `sex`, and stays so until the outfit is cleared; `beast` is for an Argonian or a Khajiit,
    /// who are drawn with the human models.
    pub fn show_outfit(&mut self, save: &Path, sex: BodySex, beast: bool, paths: &GamePaths) {
        self.items.clear();
        self.body = true;
        self.sex = sex;
        self.locked = true;
        self.notices = if beast { vec!["Argonians and Khajiit are drawn with the human models and body".to_string()] } else { Vec::new() };
        self.dressing = Some(save.to_path_buf());
        self.load(paths, false);
        let job = Job::Outfit { path: save.to_path_buf(), paths: Box::new(paths.clone()), sex };
        if self.jobs.send(job).is_err() {
            self.dressing = None;
            self.notices.push("the preview worker stopped".to_string());
        }
    }

    /// Read everything again, archives included.
    pub fn reload(&mut self, paths: &GamePaths) {
        if !self.items.is_empty() {
            self.load(paths, true);
        }
        if self.picker.is_some() {
            self.open_picker(paths, true);
        }
    }

    /// Screenshot mode: the picker open on `plugin`.
    pub fn show_picker(&mut self, plugin: &str, filter: &str, paths: &GamePaths) {
        self.picker = Some(Picker { chosen: Some(plugin.to_string()), armour_filter: filter.to_string(), ..Picker::default() });
        self.open_picker(paths, false);
    }

    /// The picker is open and has read the armours of the plugin chosen in it.
    pub fn picker_ready(&self) -> bool {
        self.picker.as_ref().is_some_and(|p| matches!(p.armours, Some(Ok(_))))
    }

    /// Open the armour picker and read the load order's armours.
    fn open_picker(&mut self, paths: &GamePaths, fresh: bool) {
        let picker = self.picker.get_or_insert_with(Picker::default);
        picker.plugins = None;
        picker.armours = None;
        let chosen = picker.chosen.clone();
        let _ = self.jobs.send(Job::Plugins { paths: Box::new(paths.clone()), fresh });
        if let Some(plugin) = chosen {
            let _ = self.jobs.send(Job::Armours { plugin, sex: self.sex });
        }
    }

    /// Add pieces not already there, and load once.
    fn add_all(&mut self, pieces: &[Piece], paths: &GamePaths) {
        let before = self.items.len();
        for m in pieces {
            if !self.items.contains(m) {
                self.items.push(m.clone());
            }
        }
        if self.items.len() != before {
            self.load(paths, false);
        }
    }

    /// Take away pieces that are there.
    fn remove_all(&mut self, pieces: &[Piece], paths: &GamePaths) {
        let before = self.items.len();
        self.items.retain(|i| !pieces.contains(i));
        if self.items.len() != before {
            self.unlock_if_empty();
            self.load(paths, false);
        }
    }

    /// With nothing left of a character's outfit, the body can be chosen again.
    fn unlock_if_empty(&mut self) {
        if self.items.is_empty() {
            self.locked = false;
        }
    }

    /// The models to draw: each piece's, on the chosen body.
    fn models(&self) -> Vec<Item> {
        self.items.iter().flat_map(|p| p.items(self.sex).iter().cloned()).collect()
    }

    fn load(&mut self, paths: &GamePaths, fresh: bool) {
        self.id += 1;
        self.loading = true;
        self.in_flight = None;
        self.drawn = None;
        if self.items.is_empty() {
            self.loading = false;
            self.loaded = None;
            self.picture = None;
            return;
        }
        let job = Job::Load {
            id: self.id,
            paths: Box::new(paths.clone()),
            request: Request { items: self.models(), body: self.body, pose: self.pose, sex: self.sex },
            fresh,
        };
        if self.jobs.send(job).is_err() {
            self.loading = false;
            self.loaded = Some(Err("the preview worker stopped".into()));
        }
    }

    fn poll(&mut self, ctx: &egui::Context, paths: &GamePaths) {
        let replies: Vec<_> = self.replies.try_iter().collect();
        for reply in replies {
            match reply {
                Reply::Loaded { id, result } if id == self.id => {
                    self.loading = false;
                    self.in_flight = None;
                    self.loaded = Some(result);
                }
                Reply::Image { id, view, image } if id == self.id => {
                    let color = egui::ColorImage::from_rgba_unmultiplied([image.width, image.height], &image.pixels);
                    match &mut self.picture {
                        Some(texture) => texture.set(color, egui::TextureOptions::LINEAR),
                        None => self.picture = Some(ctx.load_texture("model-preview", color, egui::TextureOptions::LINEAR)),
                    }
                    self.drawn = Some(view);
                    self.in_flight = None;
                }
                Reply::Changed { id } if id == self.id && !self.loading => self.load(paths, false),
                Reply::Plugins { result } => {
                    if let Some(picker) = &mut self.picker {
                        picker.plugins = Some(result);
                    }
                }
                Reply::Outfit { path, result } if self.dressing.as_ref() == Some(&path) => {
                    self.dressing = None;
                    match result {
                        Ok(dressed) => {
                            self.items = dressed.items.into_iter().map(Piece::file).collect();
                            self.notices.extend(dressed.notes);
                            self.load(paths, false);
                        }
                        Err(e) => self.notices.push(format!("Couldn't read what the character wears: {e}")),
                    }
                }
                Reply::Armours { plugin, result } => {
                    if let Some(picker) = self.picker.as_mut().filter(|p| p.chosen.as_deref() == Some(plugin.as_str())) {
                        picker.armours = Some(result);
                    }
                }
                _ => {}
            }
        }
    }

    /// The textures and problems as plain text, for Copy Report.
    pub fn report(&self) -> Option<String> {
        let loaded = self.loaded.as_ref()?.as_ref().ok()?;
        let mut out = format!("Model preview ({})\n{}\n", if self.sex == BodySex::Male { "male" } else { "female" }, self.models().iter().map(Item::label).collect::<Vec<_>>().join("\n"));
        if let Some(p) = loaded.pose.filter(|p| p.posed > 0) {
            out.push_str(&format!("posed on the game's skeleton: {} of {} skinned meshes move, by up to {:.1} units\n", p.moved, p.posed, p.largest));
        }
        for t in &loaded.textures {
            let line = match &t.result {
                TextureResult::Found { source, hides } if hides.is_empty() => format!("from {source}"),
                TextureResult::Found { source, hides } => format!("from {source} (hides {})", hides.join(", ")),
                TextureResult::Missing => "MISSING".to_string(),
                TextureResult::Unreadable(why) => format!("UNREADABLE ({why})"),
            };
            out.push_str(&format!("{} {}: {} {line}\n", t.mesh, slot_name(t.slot), t.path));
        }
        for w in &loaded.warnings {
            out.push_str(&format!("warning: {w}\n"));
        }
        Some(out)
    }

    /// Draw the view in `ui`.
    pub fn ui(&mut self, ui: &mut egui::Ui, paths: &GamePaths, colours: &Colours) {
        self.poll(ui.ctx(), paths);
        // Files dropped on the window are added.
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        for file in dropped.iter().filter(|f| f.extension().is_some_and(|e| e.eq_ignore_ascii_case("nif"))) {
            self.add(&game_path(&paths.data_dir, file), paths);
        }

        self.picker_ui(ui.ctx(), paths, colours);
        ui.add_space(6.0);
        ui.heading("Model Preview");
        ui.add_space(4.0);
        for notice in &self.notices {
            ui.weak(notice);
        }
        let mut to_add = None;
        let mut remove = None;
        let mut changed_body = false;
        let mut changed_sex = false;
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.input)
                    .hint_text("meshes/armor/iron/f/cuirass_1.nif")
                    .desired_width((ui.available_width() - 580.0).max(120.0)),
            );
            let entered = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Add").clicked() || entered {
                to_add = Some(std::mem::take(&mut self.input));
            }
            if ui.button("Add Files…").clicked() {
                let meshes = paths.data_dir.join("meshes");
                let start = if meshes.is_dir() { meshes } else { paths.data_dir.clone() };
                let picked = rfd::FileDialog::new()
                    .set_title("Add Models")
                    .add_filter("Skyrim models", &["nif"])
                    .set_directory(start)
                    .pick_files();
                for file in picked.unwrap_or_default() {
                    self.add(&game_path(&paths.data_dir, &file), paths);
                }
            }
            if ui.button("From a Plugin…").on_hover_text("Pick armours from a plugin's records").clicked() {
                self.open_picker(paths, false);
            }
            changed_body = ui.checkbox(&mut self.body, "Body").on_hover_text("Draw the installed female body under the model, for scale").changed();
            changed_body |= ui
                .checkbox(&mut self.pose, "Pose")
                .on_hover_text("Pose skinned models on the game's skeleton, as the game does. Off draws them as stored in the file.")
                .changed();
            ui.separator();
            let why_locked = "Set by the character in the save. Remove the outfit to choose again.";
            for (sex, name) in [(BodySex::Female, "Female"), (BodySex::Male, "Male")] {
                let button = ui.add_enabled(!self.locked, egui::Button::selectable(self.sex == sex, name));
                let button = if self.locked { button.on_disabled_hover_text(why_locked) } else { button.on_hover_text("Draw armours and the body as this sex wears them") };
                if button.clicked() && self.sex != sex {
                    self.sex = sex;
                    changed_sex = true;
                }
            }
        });
        if !self.items.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for (i, piece) in self.items.iter().enumerate() {
                    for item in piece.items(self.sex) {
                        ui.label(item.label()).on_hover_text(&item.path);
                    }
                    if ui.small_button("×").on_hover_text("Remove").clicked() {
                        remove = Some(i);
                    }
                    ui.add_space(6.0);
                }
            });
        }
        if let Some(text) = to_add {
            self.add(&text, paths);
        }
        if let Some(i) = remove {
            self.items.remove(i);
            self.unlock_if_empty();
            self.load(paths, false);
        }
        if changed_body || changed_sex {
            self.load(paths, false);
        }
        if changed_sex && self.picker.is_some() {
            // The picker's slot checks are for one body.
            self.open_picker(paths, false);
        }
        ui.separator();

        if self.dressing.is_some() {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.weak("Reading what the character wears…");
            });
            return;
        }
        if self.items.is_empty() {
            if self.notices.is_empty() {
                ui.add_space(8.0);
                ui.weak("Type a model's path inside the game's files, or drop .nif files here.");
                ui.weak("Add the pieces of an outfit one after another to see them together.");
            }
            return;
        }

        egui::Panel::right("preview-report")
            .default_size(340.0)
            .resizable(true)
            .show(ui, |ui| self.report_ui(ui, colours));
        self.picture_ui(ui);
    }

    /// The armour picker: plugins on the left; click an armour on the right to add its models,
    /// click it again to take them away.
    fn picker_ui(&mut self, ctx: &egui::Context, paths: &GamePaths, colours: &Colours) {
        let Some(mut picker) = self.picker.take() else { return };
        let mut open = true;
        let mut choose = None;
        let mut add: Vec<Piece> = Vec::new();
        let mut remove: Vec<Piece> = Vec::new();
        egui::Window::new("Add Armour from a Plugin")
            .id(egui::Id::new("armour-picker"))
            .default_size([760.0, 520.0])
            .default_pos(ctx.content_rect().center() - egui::vec2(380.0, 260.0))
            .collapsible(false)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Panel::left("picker-plugins").default_size(250.0).resizable(true).show(ui, |ui| {
                    ui.add(egui::TextEdit::singleline(&mut picker.plugin_filter).hint_text("Filter plugins"));
                    ui.add_space(4.0);
                    match &picker.plugins {
                        None => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.weak("Reading the load order…");
                            });
                        }
                        Some(Err(e)) => {
                            ui.colored_label(ui.visuals().error_fg_color, e);
                        }
                        Some(Ok((plugins, problems))) => {
                            let needle = picker.plugin_filter.to_lowercase();
                            egui::ScrollArea::vertical().id_salt("picker-plugin-list").auto_shrink(false).show(ui, |ui| {
                                // Last loaded first: the mods are what's being looked for, not the base game.
                                for (name, n) in plugins.iter().rev().filter(|(name, _)| name.to_lowercase().contains(&needle)) {
                                    let on = picker.chosen.as_deref() == Some(name.as_str());
                                    let row = ui.add(egui::Button::selectable(on, format!("{name}  ({n})")).truncate());
                                    if row.clicked() && !on {
                                        choose = Some(name.clone());
                                    }
                                }
                                for p in problems {
                                    ui.weak(p);
                                }
                            });
                        }
                    }
                });
                ui.add(egui::TextEdit::singleline(&mut picker.armour_filter).hint_text("Filter armours by name"));
                ui.add_space(4.0);
                match (&picker.chosen, &picker.armours) {
                    (None, _) => {
                        ui.weak("Choose a plugin.");
                    }
                    (Some(_), None) => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.weak("Reading…");
                        });
                    }
                    (Some(_), Some(Err(e))) => {
                        ui.colored_label(ui.visuals().error_fg_color, e);
                    }
                    (Some(plugin), Some(Ok(rows))) => {
                        let needle = picker.armour_filter.to_lowercase();
                        let shown: Vec<&ArmourRow> = rows
                            .iter()
                            .filter(|r| r.label.to_lowercase().contains(&needle) || r.editor_id.to_lowercase().contains(&needle))
                            .collect();
                        ui.horizontal(|ui| {
                            ui.weak(format!("{} of {} in {plugin}", shown.len(), rows.len()));
                            let all: Vec<Piece> = shown.iter().filter(|r| !r.models(self.sex).is_empty()).map(|r| r.piece()).collect();
                            if ui.button("Add All Shown").on_hover_text("Add every armour listed, for a whole outfit").clicked() {
                                add = all;
                            }
                        });
                        egui::ScrollArea::vertical().id_salt("picker-armour-list").auto_shrink(false).show(ui, |ui| {
                            for r in shown {
                                let models = r.models(self.sex);
                                let in_use = !models.is_empty() && self.items.contains(&r.piece());
                                let mut text = r.label.clone();
                                if !r.slots.is_empty() {
                                    text.push_str(&format!("   {}", r.slots));
                                }
                                if let Some(base) = &r.changes {
                                    text.push_str(&format!("   changes {base}"));
                                }
                                if models.is_empty() {
                                    text.push_str("   no models");
                                }
                                let broken = !r.problems.is_empty();
                                if broken {
                                    text.push_str("   SLOT MISMATCH");
                                }
                                let label = if broken { egui::RichText::new(text).color(colours.missing) } else { egui::RichText::new(text) };
                                let paths_text: Vec<&str> = models.iter().map(|m| m.path.as_str()).collect();
                                let mut hover = format!("{}\n{}", r.editor_id, paths_text.join("\n"));
                                if models.iter().any(|m| !m.alts.is_empty()) {
                                    hover.push_str("\n(wears alternate textures)");
                                }
                                if broken {
                                    hover.push_str(&format!("\n\n{}", r.problems.join("\n")));
                                }
                                let row = ui
                                    .add_enabled(!models.is_empty(), egui::Button::selectable(in_use, label).truncate())
                                    .on_hover_text(hover);
                                if row.clicked() {
                                    if in_use { remove = vec![r.piece()] } else { add = vec![r.piece()] }
                                }
                            }
                        });
                    }
                }
            });
        if let Some(name) = choose {
            picker.chosen = Some(name.clone());
            picker.armours = None;
            let _ = self.jobs.send(Job::Armours { plugin: name, sex: self.sex });
        }
        if open {
            self.picker = Some(picker);
        }
        self.add_all(&add, paths);
        self.remove_all(&remove, paths);
    }

    fn report_ui(&self, ui: &mut egui::Ui, colours: &Colours) {
        ui.add_space(4.0);
        ui.heading("Textures");
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let loaded = match &self.loaded {
                Some(Ok(loaded)) => loaded,
                Some(Err(e)) => {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                    return;
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.weak("Reading…");
                    });
                    return;
                }
            };
            ui.weak(format!("{} meshes", loaded.meshes));
            match loaded.pose {
                Some(p) if p.moved > 0 => {
                    ui.weak(format!("Posed on the game's skeleton: {} of {} skinned meshes move, by up to {:.1} units", p.moved, p.posed, p.largest));
                }
                Some(p) if p.posed > 0 => {
                    ui.weak(format!("Posed on the game's skeleton: all {} skinned meshes already fit it", p.posed));
                }
                _ => {}
            }
            let mut last = "";
            for t in &loaded.textures {
                if t.mesh != last {
                    ui.add_space(4.0);
                    ui.strong(&t.mesh);
                    last = &t.mesh;
                }
                let file = t.path.rsplit(['/', '\\']).next().unwrap_or(&t.path);
                match &t.result {
                    TextureResult::Found { source, hides } => {
                        ui.label(format!("{}: {file}", slot_name(t.slot))).on_hover_text(&t.path);
                        ui.colored_label(colours.installed, format!("    {source}"));
                        if !hides.is_empty() {
                            ui.colored_label(colours.disabled, format!("    hides {}", hides.join(", ")));
                        }
                    }
                    TextureResult::Missing => {
                        ui.colored_label(colours.missing, format!("{}: {file}  MISSING", slot_name(t.slot))).on_hover_text(&t.path);
                    }
                    TextureResult::Unreadable(why) => {
                        ui.colored_label(colours.missing, format!("{}: {file}  unreadable: {why}", slot_name(t.slot)));
                    }
                }
            }
            if !loaded.warnings.is_empty() {
                ui.add_space(8.0);
                ui.strong("Problems");
                for w in &loaded.warnings {
                    ui.colored_label(colours.disabled, w);
                }
            }
        });
    }

    fn picture_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (label, azimuth) in VIEWS {
                if ui.button(label).clicked() {
                    self.camera.azimuth = azimuth;
                }
            }
            if ui.button("Reset").on_hover_text("Also: double-click the picture").clicked() {
                self.camera = Camera::default();
            }
            ui.weak("Drag to turn, scroll to zoom");
        });
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        let ppp = ui.pixels_per_point();

        let mut moving = false;
        if response.dragged() {
            let d = response.drag_delta();
            self.camera.azimuth = (self.camera.azimuth + d.x * 0.5).rem_euclid(360.0);
            self.camera.elevation = (self.camera.elevation + d.y * 0.4).clamp(-85.0, 85.0);
            moving = true;
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.camera.zoom = (self.camera.zoom * (1.0 + scroll * 0.002)).clamp(0.3, 8.0);
                moving = true;
            }
        }
        if response.double_clicked() {
            self.camera = Camera::default();
        }

        // Ask for a picture of the size on screen, in steps of 16 pixels so a
        // window being resized doesn't draw one per pixel.
        let steps = |points: f32| (((points * ppp) / 16.0).round() as usize * 16).clamp(64, 1600);
        let want = ViewParams {
            camera: self.camera,
            width: steps(rect.width()),
            height: steps(rect.height()),
            supersample: if moving { 1 } else { 2 },
        };
        let ready = matches!(self.loaded, Some(Ok(_))) && !self.loading;
        if ready && self.in_flight.is_none() && self.drawn != Some(want) {
            let job = Job::Render { id: self.id, view: want };
            if self.jobs.send(job).is_ok() {
                self.in_flight = Some(want);
            }
        }

        match &self.picture {
            Some(texture) if self.drawn.is_some() => {
                let drawn = self.drawn.unwrap();
                let shown = egui::vec2(drawn.width as f32, drawn.height as f32) / ppp;
                let at = egui::Rect::from_center_size(rect.center(), shown);
                ui.painter().image(texture.id(), at, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            }
            _ if matches!(self.loaded, Some(Err(_))) => {}
            _ => {
                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.weak("Reading the model…");
                        });
                    });
                });
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use skyplug_core::steam::Edition;
    use std::time::Instant;

    const QUAD: &[u8] = include_bytes!("../tests/fixtures/quad.nif");
    const RED: &[u8] = include_bytes!("../tests/fixtures/red.dds");
    const PLUGIN: &[u8] = include_bytes!("../tests/fixtures/Fixture.esp");
    const BLUE: &[u8] = include_bytes!("../tests/fixtures/blue.dds");
    const SKELETON: &[u8] = include_bytes!("../tests/fixtures/skeleton_female.nif");

    /// A game folder in a temp dir whose Data holds the fixture quad and its red texture.
    pub(crate) fn fixture_install(name: &str) -> GamePaths {
        let root = std::env::temp_dir().join(format!("arcanaeum_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let data = root.join("game/Data");
        std::fs::create_dir_all(data.join("meshes/fixture")).unwrap();
        std::fs::create_dir_all(data.join("textures/fixture")).unwrap();
        std::fs::create_dir_all(root.join("docs/Saves")).unwrap();
        std::fs::write(data.join("meshes/fixture/quad.nif"), QUAD).unwrap();
        std::fs::write(data.join("textures/fixture/red.dds"), RED).unwrap();
        std::fs::write(data.join("textures/fixture/blue.dds"), BLUE).unwrap();
        // The game's female skeleton, as a pair of bones.
        let skeleton = data.join("meshes/actors/character/character assets female");
        std::fs::create_dir_all(&skeleton).unwrap();
        std::fs::write(skeleton.join("skeleton_female.nif"), SKELETON).unwrap();
        // A plugin with one armour, worn as the fixture quad, and enabled.
        std::fs::write(data.join("Fixture.esp"), PLUGIN).unwrap();
        std::fs::write(root.join("Plugins.txt"), "*Fixture.esp\n").unwrap();
        GamePaths {
            data_dir: data,
            plugins_txt: root.join("Plugins.txt"),
            ccc_path: root.join("game/Skyrim.ccc"),
            saves_dir: root.join("docs/Saves"),
            edition: Edition::Steam,
            prefix: None,
        }
    }

    fn start(poll_ms: u64) -> (Sender<Job>, Receiver<Reply>) {
        let (jobs, inbox) = channel();
        let (outbox, replies) = channel();
        let poll = Duration::from_millis(poll_ms);
        std::thread::spawn(move || run(inbox, outbox, egui::Context::default(), poll));
        (jobs, replies)
    }

    fn load(jobs: &Sender<Job>, paths: &GamePaths, id: u64, items: &[&str]) {
        let request = Request { items: items.iter().map(|s| Item::from(*s)).collect(), body: false, pose: true, sex: BodySex::Female };
        jobs.send(Job::Load { id, paths: Box::new(paths.clone()), request, fresh: false }).unwrap();
    }

    fn view(az: f32) -> ViewParams {
        ViewParams { camera: Camera { azimuth: az, elevation: 0.0, zoom: 1.0 }, width: 64, height: 64, supersample: 1 }
    }

    fn next(replies: &Receiver<Reply>) -> Reply {
        replies.recv_timeout(Duration::from_secs(10)).expect("the worker answers")
    }

    #[test]
    fn loads_a_model_reports_its_textures_and_draws_it() {
        let paths = fixture_install("preview_load");
        let (jobs, replies) = start(10_000);
        load(&jobs, &paths, 1, &["meshes/fixture/quad.nif"]);
        let Reply::Loaded { id: 1, result: Ok(loaded) } = next(&replies) else { panic!("expected a loaded model") };
        assert_eq!(loaded.meshes, 1);
        // The skeleton was found and the (unskinned) quad needed no posing.
        assert_eq!(loaded.pose, Some(PoseReport::default()));
        let red = loaded.textures.iter().find(|t| t.path.ends_with("red.dds")).unwrap();
        assert_eq!(red.result, TextureResult::Found { source: "loose: textures/fixture/red.dds".into(), hides: vec![] });
        let gone = loaded.textures.iter().find(|t| t.path.ends_with("gone_n.dds")).unwrap();
        assert_eq!(gone.result, TextureResult::Missing);

        jobs.send(Job::Render { id: 1, view: view(0.0) }).unwrap();
        let Reply::Image { image, .. } = next(&replies) else { panic!("expected a picture") };
        assert_eq!((image.width, image.height), (64, 64));
        let centre = &image.pixels[(32 * 64 + 32) * 4..][..4];
        assert!(centre[0] > 100 && centre[1] < 40 && centre[3] == 255, "the quad is red in the middle: {centre:?}");
        assert_eq!(image.pixels[3], 0, "the corner is transparent background");
    }

    #[test]
    fn a_variant_draws_with_its_own_textures() {
        let paths = fixture_install("preview_variant");
        let (jobs, replies) = start(10_000);
        jobs.send(Job::Plugins { paths: Box::new(paths.clone()), fresh: false }).unwrap();
        let _ = next(&replies);
        jobs.send(Job::Armours { plugin: "Fixture.esp".into(), sex: BodySex::Female }).unwrap();
        let Reply::Armours { result: Ok(rows), .. } = next(&replies) else { panic!() };
        let centre = |item: Item, id: u64| {
            let request = Request { items: vec![item], body: false, pose: false, sex: BodySex::Female };
            jobs.send(Job::Load { id, paths: Box::new(paths.clone()), request, fresh: false }).unwrap();
            let Reply::Loaded { result: Ok(loaded), .. } = next(&replies) else { panic!() };
            jobs.send(Job::Render { id, view: view(0.0) }).unwrap();
            let Reply::Image { image, .. } = next(&replies) else { panic!() };
            (image.pixels[(32 * 64 + 32) * 4..][..4].to_vec(), loaded)
        };
        let (base, _) = centre(rows[0].female[0].clone(), 1);
        let (variant, loaded) = centre(rows[1].female[0].clone(), 2);
        assert!(base[0] > 100 && base[2] < 40, "the base armour is red: {base:?}");
        assert!(variant[2] > 100 && variant[0] < 40, "its variant is blue: {variant:?}");
        // The report names the swapped-in texture, not the model's own.
        let diffuse = loaded.textures.iter().find(|t| t.slot == 0).unwrap();
        assert!(diffuse.path.ends_with("blue.dds"), "{}", diffuse.path);
    }

    #[test]
    fn posing_can_be_turned_off() {
        let paths = fixture_install("preview_pose_off");
        let (jobs, replies) = start(10_000);
        let request = Request { items: vec![Item::from("meshes/fixture/quad.nif")], body: false, pose: false, sex: BodySex::Female };
        jobs.send(Job::Load { id: 1, paths: Box::new(paths), request, fresh: false }).unwrap();
        let Reply::Loaded { result: Ok(loaded), .. } = next(&replies) else { panic!() };
        assert_eq!(loaded.pose, None);
    }

    #[test]
    fn a_missing_or_legacy_model_is_an_error_not_a_crash() {
        let paths = fixture_install("preview_errors");
        let (jobs, replies) = start(10_000);
        load(&jobs, &paths, 1, &["meshes/none.nif"]);
        let Reply::Loaded { result: Err(e), .. } = next(&replies) else { panic!() };
        assert!(e.contains("no such file"), "{e}");

        // The fixture with its stream version changed to Skyrim LE's 83.
        let mut legacy = QUAD.to_vec();
        let nl = legacy.iter().position(|&b| b == b'\n').unwrap();
        let at = nl + 1 + 4 + 1 + 4 + 4;
        legacy[at..at + 4].copy_from_slice(&83u32.to_le_bytes());
        std::fs::write(paths.data_dir.join("meshes/fixture/le.nif"), legacy).unwrap();
        load(&jobs, &paths, 2, &["meshes/fixture/le.nif"]);
        let Reply::Loaded { result: Err(e), .. } = next(&replies) else { panic!() };
        assert!(e.contains("Skyrim LE"), "{e}");

        // One good model and one bad: the good one still draws, with the bad one listed.
        load(&jobs, &paths, 3, &["meshes/fixture/quad.nif", "meshes/none.nif"]);
        let Reply::Loaded { result: Ok(loaded), .. } = next(&replies) else { panic!() };
        assert!(loaded.warnings.iter().any(|w| w.contains("none.nif")), "{:?}", loaded.warnings);
    }

    #[test]
    fn only_the_newest_picture_asked_for_is_drawn_and_stale_ones_are_dropped() {
        let paths = fixture_install("preview_coalesce");
        let (jobs, replies) = start(10_000);
        load(&jobs, &paths, 1, &["meshes/fixture/quad.nif"]);
        let _ = next(&replies);
        // A burst of 40 pictures asked for at once, as a drag does.
        for i in 0..40 {
            jobs.send(Job::Render { id: 1, view: view(i as f32) }).unwrap();
        }
        let mut last = None;
        let mut count = 0;
        while let Ok(Reply::Image { view, .. }) = replies.recv_timeout(Duration::from_millis(1500)) {
            last = Some(view.camera.azimuth);
            count += 1;
        }
        assert_eq!(last, Some(39.0), "the newest request is answered");
        assert!(count < 40, "drew {count} pictures for 40 requests");
        // A picture for a load that isn't current is not drawn.
        jobs.send(Job::Render { id: 99, view: view(0.0) }).unwrap();
        assert!(replies.recv_timeout(Duration::from_millis(300)).is_err());
    }

    #[test]
    fn a_changed_texture_is_noticed_and_so_is_a_new_one_overriding_an_archive() {
        let paths = fixture_install("preview_watch");
        let (jobs, replies) = start(50);
        load(&jobs, &paths, 1, &["meshes/fixture/quad.nif"]);
        assert!(matches!(next(&replies), Reply::Loaded { result: Ok(_), .. }));
        assert!(replies.recv_timeout(Duration::from_millis(300)).is_err(), "nothing changed yet");

        // The texture is saved again, later.
        let file = std::fs::File::options().write(true).open(paths.data_dir.join("textures/fixture/red.dds")).unwrap();
        file.set_modified(SystemTime::now() + Duration::from_secs(60)).unwrap();
        let started = Instant::now();
        loop {
            match replies.recv_timeout(Duration::from_secs(5)) {
                Ok(Reply::Changed { id: 1 }) => break,
                Ok(_) => {}
                Err(_) => panic!("no change noticed after {:?}", started.elapsed()),
            }
        }
        // Once told, it doesn't repeat.
        assert!(replies.recv_timeout(Duration::from_millis(300)).is_err());

        // The missing normal map appears as a new loose file.
        std::fs::write(paths.data_dir.join("textures/fixture/gone_n.dds"), RED).unwrap();
        assert!(matches!(replies.recv_timeout(Duration::from_secs(5)), Ok(Reply::Changed { id: 1 })));
    }

    #[test]
    fn an_outfit_from_a_file_that_is_not_a_save_is_an_error_not_a_crash() {
        let paths = fixture_install("preview_outfit");
        let (jobs, replies) = start(10_000);
        let save = paths.data_dir.join("not-a-save.ess");
        std::fs::write(&save, b"not a save").unwrap();
        jobs.send(Job::Outfit { path: save.clone(), paths: Box::new(paths), sex: BodySex::Female }).unwrap();
        let Reply::Outfit { path, result: Err(e) } = next(&replies) else { panic!() };
        assert_eq!(path, save);
        assert!(e.contains("not a Skyrim save") || e.contains("unexpected end"), "{e}");
    }

    #[test]
    fn the_armours_of_a_plugin_are_listed_with_their_models() {
        let paths = fixture_install("preview_armours");
        let (jobs, replies) = start(10_000);
        // Asked before the catalog is read: an error, not a hang or a crash.
        jobs.send(Job::Armours { plugin: "Fixture.esp".into(), sex: BodySex::Female }).unwrap();
        let Reply::Armours { result: Err(e), .. } = next(&replies) else { panic!() };
        assert!(e.contains("haven't been read"), "{e}");

        jobs.send(Job::Plugins { paths: Box::new(paths.clone()), fresh: false }).unwrap();
        let Reply::Plugins { result: Ok((plugins, problems)) } = next(&replies) else { panic!() };
        assert_eq!(plugins, [("Fixture.esp".to_string(), 2)]);
        assert!(problems.is_empty(), "{problems:?}");

        jobs.send(Job::Armours { plugin: "Fixture.esp".into(), sex: BodySex::Female }).unwrap();
        let Reply::Armours { plugin, result: Ok(rows) } = next(&replies) else { panic!() };
        assert_eq!(plugin, "Fixture.esp");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            ArmourRow {
                label: "Fixture Cuirass".into(),
                editor_id: "FixtureCuirass".into(),
                slots: "Body".into(),
                female: vec![Item::from("meshes/fixture/quad.nif")],
                male: vec![Item::from("meshes/fixture/quad.nif")],
                changes: None,
                problems: vec![]
            }
        );
        // The variant wears the same model with a texture swap, so it is a different item.
        assert_eq!(rows[1].label, "Fixture Cuirass Alt");
        assert_eq!(rows[1].female[0].path, "meshes/fixture/quad.nif");
        assert_eq!(rows[1].female[0].alts.len(), 1);
        assert_eq!(rows[1].female[0].alts[0].shape, "Quad");
        assert_eq!(rows[1].female[0].alts[0].textures[0], "textures/fixture/blue.dds");
        assert_ne!(rows[0].female[0], rows[1].female[0]);
        jobs.send(Job::Armours { plugin: "Nope.esp".into(), sex: BodySex::Female }).unwrap();
        let Reply::Armours { result: Err(e), .. } = next(&replies) else { panic!() };
        assert!(e.contains("isn't in the load order"), "{e}");
    }

    #[test]
    fn coalescing_keeps_the_newest_render_and_everything_else_in_order() {
        let paths = fixture_install("preview_coalesce_unit");
        let req = || Request { items: vec![], body: false, pose: true, sex: BodySex::Female };
        let load = |id| Job::Load { id, paths: Box::new(paths.clone()), request: req(), fresh: false };
        let render = |id, az| Job::Render { id, view: view(az) };
        let armours = || Job::Armours { plugin: "A.esp".into(), sex: BodySex::Female };
        let out = coalesce(vec![render(1, 1.0), load(2), render(2, 2.0), armours(), render(2, 3.0), load(3)]);
        let shape: Vec<_> = out
            .iter()
            .map(|j| match j {
                Job::Load { id, .. } => format!("load {id}"),
                Job::Render { view, .. } => format!("render {}", view.camera.azimuth),
                Job::Plugins { .. } => "plugins".to_string(),
                Job::Outfit { .. } => "outfit".to_string(),
                Job::Armours { plugin, .. } => format!("armours {plugin}"),
            })
            .collect();
        assert_eq!(shape, ["load 2", "armours A.esp", "render 3", "load 3"]);
    }

    #[test]
    fn a_piece_has_the_models_of_each_body() {
        let (f, m) = (Item::from("meshes/f.nif"), Item::from("meshes/m.nif"));
        let armour = Piece { female: vec![f.clone()], male: vec![m.clone()] };
        assert_eq!(armour.items(BodySex::Female), std::slice::from_ref(&f));
        assert_eq!(armour.items(BodySex::Male), [m]);
        // A file is what it is on either body.
        let file = Piece::file(f.clone());
        assert_eq!(file.items(BodySex::Female), file.items(BodySex::Male));
    }

    #[test]
    fn the_body_is_the_characters_until_their_outfit_is_cleared() {
        let paths = fixture_install("preview_sex_lock");
        let mut preview = Preview::start(&egui::Context::default());
        assert!(!preview.locked && preview.sex == BodySex::Female);

        preview.show_outfit(Path::new("not-a-save.ess"), BodySex::Male, false, &paths);
        assert!(preview.locked && preview.sex == BodySex::Male && preview.body);
        assert!(preview.notices.is_empty());

        // Pieces from the picker follow the locked body; taking the last one away unlocks it.
        let piece = Piece { female: vec![Item::from("meshes/f.nif")], male: vec![Item::from("meshes/m.nif")] };
        preview.items = vec![piece.clone()];
        assert_eq!(preview.items(), ["meshes/m.nif"]);
        preview.remove_all(&[piece], &paths);
        assert!(!preview.locked);
        assert_eq!(preview.sex, BodySex::Male, "the body stays as it was, but can be changed now");

        // Argonians and Khajiit are drawn as human, and the view says so.
        preview.show_outfit(Path::new("not-a-save.ess"), BodySex::Female, true, &paths);
        assert_eq!(preview.notices.len(), 1);
        preview.reset();
        assert!(!preview.locked && preview.sex == BodySex::Female);
    }

    #[test]
    fn a_load_draws_the_chosen_bodys_skeleton_and_body() {
        // The fixture install has the female skeleton only: a man's models are drawn unposed
        // (the report says so), a woman's are posed.
        let paths = fixture_install("preview_sex_skeleton");
        let (jobs, replies) = start(10_000);
        for (id, sex, posed) in [(1, BodySex::Female, true), (2, BodySex::Male, false)] {
            let request = Request { items: vec![Item::from("meshes/fixture/quad.nif")], body: false, pose: true, sex };
            jobs.send(Job::Load { id, paths: Box::new(paths.clone()), request, fresh: false }).unwrap();
            let Reply::Loaded { result: Ok(loaded), .. } = next(&replies) else { panic!() };
            assert_eq!(loaded.pose.is_some(), posed, "{sex:?}");
        }
    }

    #[test]
    fn picked_files_become_game_paths() {
        let data = Path::new("/games/Skyrim/Data");
        assert_eq!(game_path(data, Path::new("/games/Skyrim/Data/meshes/a/b.nif")), "meshes/a/b.nif");
        assert_eq!(game_path(data, Path::new("/elsewhere/c.nif")), "/elsewhere/c.nif");
    }
}
