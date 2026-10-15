//! Reading and writing the game's files off the UI thread (ADR-0002). A
//! save folder holds dozens of multi-megabyte files, reading the setup
//! scans every plugin in `Data`, and the masterlist is a large YAML file,
//! so all of it happens on a worker that reports back by channel.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use skyplug_core::saves::{self, SaveEntry};
use skyplug_core::GamePaths;

use crate::analysis::{Analysis, Links, Setup};
use crate::game;
use crate::mod_search::vortex_scanner::get_masterlist_data;

/// A save folder's saves, by character name, newest save first.
pub type Characters = BTreeMap<String, Vec<SaveEntry>>;

pub enum Command {
    /// Find the game: the install in this folder, or Steam's.
    FindGame(Option<PathBuf>),
    ScanFolder(PathBuf),
    /// Read Data, Plugins.txt and Skyrim.ccc.
    ReadSetup(GamePaths),
    OpenSave(PathBuf),
    /// Write these (exact name, on) changes to this Plugins.txt.
    Apply(PathBuf, Vec<(String, bool)>),
}

pub enum Update {
    Game(Result<GamePaths, String>),
    Folder {
        path: PathBuf,
        characters: Characters,
        unreadable: usize,
    },
    Setup(Result<Setup, String>),
    Save(Result<Box<Analysis>, String>),
    /// How many lines of Plugins.txt changed.
    Applied(Result<usize, String>),
}

pub struct Loader {
    commands: Sender<Command>,
    updates: Receiver<Update>,
    /// Commands sent and not yet answered.
    pub pending: usize,
}

impl Loader {
    pub fn start(ctx: &egui::Context) -> Self {
        let (commands, inbox) = channel::<Command>();
        let (outbox, updates) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut masterlists = Masterlists::default();
            for command in inbox {
                let update = match command {
                    Command::FindGame(folder) => Update::Game(
                        game::find(folder.as_deref()).map(|install| game::paths(&install)),
                    ),
                    Command::ScanFolder(path) => {
                        let (characters, unreadable) = scan_folder(&path);
                        Update::Folder {
                            path,
                            characters,
                            unreadable,
                        }
                    }
                    Command::ReadSetup(paths) => {
                        Update::Setup(read_setup(&paths, &mut masterlists))
                    }
                    Command::OpenSave(path) => {
                        Update::Save(open_save(&path, &mut masterlists).map(Box::new))
                    }
                    Command::Apply(plugins_txt, changes) => Update::Applied(
                        skyplug_core::plugins::apply_changes(&plugins_txt, &changes)
                            .map_err(|e| format!("{e:#}")),
                    ),
                };
                if outbox.send(update).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            commands,
            updates,
            pending: 0,
        }
    }

    pub fn send(&mut self, command: Command) {
        if self.commands.send(command).is_ok() {
            self.pending += 1;
        }
    }

    pub fn poll(&mut self) -> Vec<Update> {
        let updates: Vec<_> = self.updates.try_iter().collect();
        self.pending = self.pending.saturating_sub(updates.len());
        updates
    }

    pub fn busy(&self) -> bool {
        self.pending > 0
    }
}

/// The Vortex masterlists, read once per game.
#[derive(Default)]
struct Masterlists(HashMap<&'static str, Links>);

impl Masterlists {
    fn links(&mut self, game: &'static str) -> &Links {
        self.0.entry(game).or_insert_with(|| {
            get_masterlist_data(game)
                .map(|plugins| {
                    plugins
                        .into_iter()
                        .filter(|p| !p.urls.is_empty())
                        .map(|p| (p.name.to_lowercase(), p.urls))
                        .collect()
                })
                .unwrap_or_default()
        })
    }
}

/// Every readable `.ess` in `folder`, grouped by character, and how many
/// couldn't be read.
pub fn scan_folder(folder: &Path) -> (Characters, usize) {
    let mut characters = Characters::new();
    let list = match saves::list_saves(folder) {
        Ok(list) => list,
        Err(e) => {
            tracing::error!("{e:#}");
            return (characters, 0);
        }
    };
    // Already newest first, by the time each save was written.
    for entry in list.entries {
        characters
            .entry(entry.header.player_name.clone())
            .or_default()
            .push(entry);
    }
    (characters, list.skipped.len())
}

fn read_setup(paths: &GamePaths, masterlists: &mut Masterlists) -> Result<Setup, String> {
    let statuses = paths.plugin_status().map_err(|e| format!("{e:#}"))?;
    Ok(Setup {
        statuses,
        links: masterlists.links("skyrimse").clone(),
    })
}

fn open_save(path: &Path, masterlists: &mut Masterlists) -> Result<Analysis, String> {
    tracing::info!("Loading file: {:?}", path);
    let save = saves::read_save_from_file(path).map_err(|e| format!("{e:#}"))?;
    let links = masterlists
        .links(if save.header.is_se {
            "skyrimse"
        } else {
            "skyrim"
        })
        .clone();
    Ok(Analysis {
        path: path.to_path_buf(),
        save,
        links,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_saves_are_counted_not_fatal() {
        let dir = std::env::temp_dir().join("arcanaeum_loader_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("broken.ess"), b"TESV_SAVEGAME").unwrap();
        std::fs::write(dir.join("notes.txt"), b"not a save").unwrap();

        let (characters, unreadable) = scan_folder(&dir);
        assert!(characters.is_empty());
        assert_eq!(unreadable, 1);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
