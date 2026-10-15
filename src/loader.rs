//! Reading saves off the UI thread. A save folder holds dozens of
//! multi-megabyte files, and opening one also scans the game's `Data`
//! folder, so all of it happens on a worker that reports back by channel.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::analysis::Analysis;
use crate::mod_search::data_scanner::scan_installed_plugins;
use crate::mod_search::vortex_scanner::{get_masterlist_data, Plugin};
use crate::save_file_parser::header::Header;
use crate::save_file_parser::{parse, parse_header_only};
use crate::utils::skyrim_se::find_skyrim_data_path;

/// One `.ess` file, with just its header read.
#[derive(Clone)]
pub struct SaveFile {
    pub path: PathBuf,
    pub file_name: String,
    pub header: Header,
}

/// A save folder's saves, by character name, newest save first.
pub type Characters = BTreeMap<String, Vec<SaveFile>>;

pub enum Command {
    ScanFolder(PathBuf),
    OpenSave(PathBuf),
}

pub enum Update {
    Folder {
        path: PathBuf,
        characters: Characters,
        unreadable: usize,
    },
    Save(Result<Box<Analysis>, String>),
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
            for command in inbox {
                let update = match command {
                    Command::ScanFolder(path) => {
                        let (characters, unreadable) = scan_folder(&path);
                        Update::Folder {
                            path,
                            characters,
                            unreadable,
                        }
                    }
                    Command::OpenSave(path) => Update::Save(open_save(&path).map(Box::new)),
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

fn read_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}

/// The parser indexes into the file without bounds checks, so a truncated
/// or foreign file panics; treat that as unreadable rather than losing the
/// worker.
fn guarded<T>(f: impl FnOnce() -> T) -> Option<T> {
    catch_unwind(AssertUnwindSafe(f)).ok()
}

/// Every readable `.ess` in `folder`, grouped by character, and how many
/// couldn't be read.
pub fn scan_folder(folder: &Path) -> (Characters, usize) {
    let mut characters = Characters::new();
    let mut unreadable = 0;
    let Ok(entries) = std::fs::read_dir(folder) else {
        tracing::error!("Could not read save folder {:?}", folder);
        return (characters, 0);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|e| e != "ess") {
            continue;
        }
        let header = read_file(&path)
            .ok()
            .and_then(|buf| guarded(|| parse_header_only(buf)));
        let Some(header) = header else {
            unreadable += 1;
            continue;
        };
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        characters
            .entry(header.player_name.clone())
            .or_default()
            .push(SaveFile {
                path,
                file_name,
                header,
            });
    }
    for saves in characters.values_mut() {
        saves.sort_by_key(|s| std::cmp::Reverse(s.header.save_number));
    }
    (characters, unreadable)
}

fn load_mod_map(game: &str) -> HashMap<String, Plugin> {
    get_masterlist_data(game)
        .map(|plugins| plugins.into_iter().map(|p| (p.name.clone(), p)).collect())
        .unwrap_or_default()
}

/// Plugins actually present on disk, found by scanning the game's `Data`
/// directory directly rather than reading a mod manager's config - this
/// picks up manually installed mods, not just ones a manager tracks.
fn load_installed() -> HashSet<String> {
    match find_skyrim_data_path() {
        Some(data_path) => scan_installed_plugins(&data_path),
        None => {
            tracing::error!("Could not find Skyrim Data directory to scan for installed plugins.");
            HashSet::new()
        }
    }
}

fn open_save(path: &Path) -> Result<Analysis, String> {
    tracing::info!("Loading file: {:?}", path);
    let buf = read_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let save = guarded(|| parse(buf))
        .ok_or_else(|| format!("{} isn't a save Arcanaeum can read", path.display()))?;
    let mod_map = load_mod_map(if save.header.is_se {
        "skyrimse"
    } else {
        "skyrim"
    });
    let installed = load_installed();
    Ok(Analysis::new(
        path.display().to_string(),
        save,
        &mod_map,
        &installed,
    ))
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
