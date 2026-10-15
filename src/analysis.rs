//! What a save needs: each plugin it was made with, and whether that
//! plugin is on disk, findable through the Vortex masterlist, or missing.

use std::collections::{HashMap, HashSet};

use crate::mod_search::vortex_scanner::Plugin;
use crate::save_file_parser::SaveInfo;

/// Where a plugin comes from.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Origin {
    /// The game and its DLC.
    Native,
    CreationClub,
    Mod,
}

pub fn origin_of(name: &str) -> Origin {
    match name {
        "Skyrim.esm" | "Update.esm" | "Dragonborn.esm" | "HearthFires.esm" | "Dawnguard.esm" => {
            Origin::Native
        }
        _ if name.starts_with("cc") => Origin::CreationClub,
        _ => Origin::Mod,
    }
}

/// Whether a mod plugin can be loaded.
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Status {
    /// Shipped with the game or Creation Club; nothing to do.
    Included,
    /// Found in the game's `Data` folder.
    Installed,
    /// Not on disk, but the masterlist knows where to get it.
    Available(Vec<String>),
    /// Not on disk and not in the masterlist.
    Missing,
}

#[derive(Debug, Clone)]
pub struct PluginRow {
    pub name: String,
    pub origin: Origin,
    pub status: Status,
}

/// The status column's values, as the status filter offers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    BaseGame,
    CreationClub,
    Installed,
    NotInstalled,
    NotFound,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::BaseGame,
        Kind::CreationClub,
        Kind::Installed,
        Kind::NotInstalled,
        Kind::NotFound,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::BaseGame => "Base game",
            Kind::CreationClub => "Creation Club",
            Kind::Installed => "Installed",
            Kind::NotInstalled => "Not installed",
            Kind::NotFound => "Not found",
        }
    }
}

impl PluginRow {
    pub fn kind(&self) -> Kind {
        match self.status {
            Status::Included if self.origin == Origin::Native => Kind::BaseGame,
            Status::Included => Kind::CreationClub,
            Status::Installed => Kind::Installed,
            Status::Available(_) => Kind::NotInstalled,
            Status::Missing => Kind::NotFound,
        }
    }
}

/// A filter per column of the plugin list: text in the index and the
/// name (any part, any case), and which statuses are hidden.
#[derive(Debug, Default)]
pub struct PluginFilter {
    pub index: String,
    pub name: String,
    pub hidden: HashSet<Kind>,
}

impl PluginFilter {
    /// Whether the plugin at `index` in the load order passes.
    pub fn matches(&self, index: usize, row: &PluginRow) -> bool {
        let has =
            |text: &str, part: &str| text.to_lowercase().contains(&part.trim().to_lowercase());
        has(&format!("{index:02X}"), &self.index)
            && has(&row.name, &self.name)
            && !self.hidden.contains(&row.kind())
    }

    pub fn is_active(&self) -> bool {
        !self.index.trim().is_empty() || !self.name.trim().is_empty() || !self.hidden.is_empty()
    }
}

/// A parsed save and the state of each of its plugins.
#[derive(Clone)]
pub struct Analysis {
    pub path: String,
    pub save: SaveInfo,
    pub plugins: Vec<PluginRow>,
}

impl Analysis {
    pub fn new(
        path: String,
        save: SaveInfo,
        mod_map: &HashMap<String, Plugin>,
        installed: &HashSet<String>,
    ) -> Self {
        let plugins = save
            .plugin_info
            .plugins
            .iter()
            .map(|name| row(name, mod_map, installed))
            .collect();
        Self {
            path,
            save,
            plugins,
        }
    }

    /// The screenshot the game took when saving: RGBA in Special Edition
    /// saves, RGB in the original. `None` when the data doesn't match the size
    /// in the header. Alpha is ignored, since the game doesn't use it.
    pub fn screenshot(&self) -> Option<egui::ColorImage> {
        let h = &self.save.header;
        let (w, ht) = (h.screenshot_width as usize, h.screenshot_height as usize);
        let channels = if h.is_se { 4 } else { 3 };
        let data = &self.save.screenshot_data;
        if w == 0 || ht == 0 || data.len() != w * ht * channels {
            return None;
        }
        let pixels = data
            .chunks_exact(channels)
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        Some(egui::ColorImage::new([w, ht], pixels))
    }

    /// (installed, available, missing) among the mods.
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut counts = (0, 0, 0);
        for p in &self.plugins {
            match p.status {
                Status::Installed => counts.0 += 1,
                Status::Available(_) => counts.1 += 1,
                Status::Missing => counts.2 += 1,
                Status::Included => {}
            }
        }
        counts
    }

    /// A plain-text report of the save's plugins, for the clipboard.
    pub fn report(&self) -> String {
        let h = &self.save.header;
        let mut out = format!(
            "{} (level {}), save {}\n{}\n\n",
            h.player_name, h.player_level, h.save_number, self.path
        );
        for p in &self.plugins {
            let status = match &p.status {
                Status::Included if p.origin == Origin::Native => "base game".to_string(),
                Status::Included => "Creation Club".to_string(),
                Status::Installed => "installed".to_string(),
                Status::Available(urls) => format!("not installed: {}", urls.join(" ")),
                Status::Missing => "not installed, not in the masterlist".to_string(),
            };
            out.push_str(&format!("{}\t{}\n", p.name, status));
        }
        out
    }
}

fn row(name: &str, mod_map: &HashMap<String, Plugin>, installed: &HashSet<String>) -> PluginRow {
    let origin = origin_of(name);
    let status = if origin != Origin::Mod {
        Status::Included
    } else if installed.contains(&name.to_lowercase()) {
        Status::Installed
    } else if let Some(plugin) = mod_map.get(name).filter(|p| !p.urls.is_empty()) {
        Status::Available(plugin.urls.clone())
    } else {
        Status::Missing
    };
    PluginRow {
        name: name.to_string(),
        origin,
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_known_native_file_should_return_native_type() {
        assert_eq!(origin_of("Skyrim.esm"), Origin::Native);
    }

    #[test]
    fn given_known_creation_club_file_should_return_creation_club_type() {
        assert_eq!(origin_of("cceejsse001-hstead.esm"), Origin::CreationClub);
    }

    #[test]
    fn given_known_mod_file_should_return_mod_type() {
        assert_eq!(origin_of("TrueStormsSE.esp"), Origin::Mod);
    }

    #[test]
    fn mods_are_installed_available_or_missing() {
        let mut mod_map = HashMap::new();
        mod_map.insert(
            "Frostfall.esp".to_string(),
            Plugin {
                name: "Frostfall.esp".into(),
                urls: vec!["https://example.com/frostfall".into()],
            },
        );
        mod_map.insert(
            "NoLinks.esp".to_string(),
            Plugin {
                name: "NoLinks.esp".into(),
                urls: vec![],
            },
        );
        let installed: HashSet<String> = ["truestormsse.esp".to_string()].into();

        let status = |name: &str| row(name, &mod_map, &installed).status;
        assert_eq!(status("Skyrim.esm"), Status::Included);
        assert_eq!(status("ccbgssse001-fish.esm"), Status::Included);
        assert_eq!(status("TrueStormsSE.esp"), Status::Installed);
        assert_eq!(
            status("Frostfall.esp"),
            Status::Available(vec!["https://example.com/frostfall".into()])
        );
        assert_eq!(status("NoLinks.esp"), Status::Missing);
        assert_eq!(status("Unknown.esp"), Status::Missing);
    }

    #[test]
    fn each_column_filters() {
        let installed: HashSet<String> = ["truestormsse.esp".to_string()].into();
        let rows: Vec<_> = [
            "Skyrim.esm",
            "ccbgssse001-fish.esm",
            "TrueStormsSE.esp",
            "Unknown.esp",
        ]
        .iter()
        .map(|n| row(n, &HashMap::new(), &installed))
        .collect();
        let shown = |filter: &PluginFilter| -> Vec<&str> {
            rows.iter()
                .enumerate()
                .filter(|(i, r)| filter.matches(*i, r))
                .map(|(_, r)| r.name.as_str())
                .collect()
        };

        let mut filter = PluginFilter::default();
        assert!(!filter.is_active());
        assert_eq!(shown(&filter).len(), 4);

        filter.name = " storms ".into();
        assert_eq!(shown(&filter), ["TrueStormsSE.esp"]);

        filter = PluginFilter {
            index: "03".into(),
            ..Default::default()
        };
        assert_eq!(shown(&filter), ["Unknown.esp"]);

        filter = PluginFilter::default();
        filter.hidden.extend([Kind::BaseGame, Kind::CreationClub]);
        assert!(filter.is_active());
        assert_eq!(shown(&filter), ["TrueStormsSE.esp", "Unknown.esp"]);

        filter.name = "unknown".into();
        assert_eq!(shown(&filter), ["Unknown.esp"]);
    }
}
