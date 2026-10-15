//! Plugins and their state: the game's load order from skyplug-core, a
//! save's plugins checked against it, changes not yet applied to
//! Plugins.txt, and the column filters and report text for both.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use skyplug_core::plugins::{PluginStatus, State};
use skyplug_core::saves::SaveFile;

/// Download links from the Vortex masterlist, by lowercased plugin name.
pub type Links = HashMap<String, Vec<String>>;

/// The base game and its DLC, for naming a save's plugins when the game
/// itself can't be found.
const NATIVE_MASTERS: [&str; 5] = [
    "Skyrim.esm",
    "Update.esm",
    "Dawnguard.esm",
    "HearthFires.esm",
    "Dragonborn.esm",
];

/// The game's plugins as they are on disk and in Plugins.txt.
#[derive(Debug, Clone, Default)]
pub struct Setup {
    pub statuses: Vec<PluginStatus>,
    /// The Special Edition masterlist's links, for plugins that aren't in Data.
    pub links: Links,
}

impl Setup {
    fn status(&self, name: &str) -> Option<&PluginStatus> {
        self.statuses
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// Whether Plugins.txt has `name` switched on now.
    fn is_enabled(&self, name: &str) -> bool {
        self.status(name).is_some_and(|s| s.state == State::Enabled)
    }
}

/// Plugins switched on or off but not yet written to Plugins.txt, by
/// lowercased name: the exact name and the state wanted.
#[derive(Debug, Clone, Default)]
pub struct Pending(BTreeMap<String, (String, bool)>);

impl Pending {
    pub fn get(&self, name: &str) -> Option<bool> {
        self.0.get(&name.to_lowercase()).map(|(_, on)| *on)
    }

    /// Asks for `name` on or off; asking for what Plugins.txt already says
    /// drops the change.
    pub fn set(&mut self, setup: &Setup, name: &str, on: bool) {
        let key = name.to_lowercase();
        if setup.is_enabled(name) == on {
            self.0.remove(&key);
        } else {
            self.0.insert(key, (name.to_string(), on));
        }
    }

    /// The changes as skyplug-core applies them: (exact name, on).
    pub fn changes(&self) -> Vec<(String, bool)> {
        self.0.values().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// The status column's values, as the status filter offers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The game and its DLC: always loaded.
    BaseGame,
    /// Listed in Skyrim.ccc: always loaded.
    CreationClub,
    /// In Data and switched on in Plugins.txt.
    Active,
    /// In Data but switched off.
    Disabled,
    /// Not in Data, and the masterlist knows where to get it.
    NotInstalled,
    /// Not in Data and nowhere to get it.
    NotFound,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::BaseGame,
        Kind::CreationClub,
        Kind::Active,
        Kind::Disabled,
        Kind::NotInstalled,
        Kind::NotFound,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Kind::BaseGame => "Base game",
            Kind::CreationClub => "Creation Club",
            Kind::Active => "Active",
            Kind::Disabled => "Disabled",
            Kind::NotInstalled => "Not installed",
            Kind::NotFound => "Not found",
        }
    }
}

/// One line of a plugin table.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The save's plugin index, or the plugin's place in the load order.
    pub index: Option<usize>,
    pub name: String,
    pub kind: Kind,
    pub links: Vec<String>,
    /// On or off, for plugins Plugins.txt decides; `None` for ones that
    /// always load or aren't there to switch.
    pub toggle: Option<bool>,
    /// Switched but not yet applied.
    pub pending: bool,
}

impl Row {
    /// The # column, in hex as the game numbers plugins.
    pub fn index_text(&self) -> String {
        self.index.map(|i| format!("{i:02X}")).unwrap_or_default()
    }

    fn new(
        name: &str,
        index: Option<usize>,
        setup: Option<&Setup>,
        links: &Links,
        pending: &Pending,
    ) -> Self {
        let mut row = Row {
            index,
            name: name.to_string(),
            kind: Kind::NotFound,
            links: Vec::new(),
            toggle: None,
            pending: false,
        };
        let status = setup.and_then(|s| s.status(name));
        row.kind = match status {
            Some(s) if s.is_cc => Kind::CreationClub,
            Some(s) if s.is_forced => Kind::BaseGame,
            Some(s) if s.state != State::Missing => {
                let on = pending.get(name).unwrap_or(s.state == State::Enabled);
                row.toggle = Some(on);
                row.pending = pending.get(name).is_some();
                if on {
                    Kind::Active
                } else {
                    Kind::Disabled
                }
            }
            // Without the game to look at, name what the name says.
            None if setup.is_none()
                && NATIVE_MASTERS.iter().any(|n| n.eq_ignore_ascii_case(name)) =>
            {
                Kind::BaseGame
            }
            None if setup.is_none() && name.to_lowercase().starts_with("cc") => Kind::CreationClub,
            _ => {
                row.links = links.get(&name.to_lowercase()).cloned().unwrap_or_default();
                if row.links.is_empty() {
                    Kind::NotFound
                } else {
                    Kind::NotInstalled
                }
            }
        };
        row
    }
}

/// A save's plugins, in the save's order, checked against the game's
/// setup (when it was found) and the changes not yet applied.
pub fn save_rows(
    save: &SaveFile,
    setup: Option<&Setup>,
    links: &Links,
    pending: &Pending,
) -> Vec<Row> {
    save.plugins
        .iter()
        .enumerate()
        .map(|(i, name)| Row::new(name, Some(i), setup, links, pending))
        .collect()
}

/// Every plugin the game knows of: those that load, in load order, then
/// the rest by name.
pub fn load_order_rows(setup: &Setup, pending: &Pending) -> Vec<Row> {
    let mut statuses: Vec<_> = setup.statuses.iter().collect();
    statuses.sort_by_key(|s| (s.load_order.is_none(), s.load_order, s.name.to_lowercase()));
    statuses
        .into_iter()
        .map(|s| {
            let index = s.load_order.map(|n| n - 1);
            Row::new(&s.name, index, Some(setup), &setup.links, pending)
        })
        .collect()
}

/// How many rows there are of each kind.
pub fn counts(rows: &[Row]) -> HashMap<Kind, usize> {
    let mut counts = HashMap::new();
    for row in rows {
        *counts.entry(row.kind).or_insert(0) += 1;
    }
    counts
}

/// A plain-text report of `rows` under `title`, for the clipboard.
pub fn report(title: &str, rows: &[Row]) -> String {
    let mut out = format!("{title}\n\n");
    for row in rows {
        let mut status = row.kind.label().to_string();
        if row.pending {
            status.push_str(" (not applied)");
        }
        if !row.links.is_empty() {
            status = format!("{status}: {}", row.links.join(" "));
        }
        out.push_str(&format!("{}\t{}\t{status}\n", row.index_text(), row.name));
    }
    out
}

/// A filter per column of a plugin table: text in the index and the name
/// (any part, any case), and which statuses are hidden.
#[derive(Debug, Default)]
pub struct PluginFilter {
    pub index: String,
    pub name: String,
    pub hidden: HashSet<Kind>,
}

impl PluginFilter {
    pub fn matches(&self, row: &Row) -> bool {
        let has =
            |text: &str, part: &str| text.to_lowercase().contains(&part.trim().to_lowercase());
        has(&row.index_text(), &self.index)
            && has(&row.name, &self.name)
            && !self.hidden.contains(&row.kind)
    }

    pub fn is_active(&self) -> bool {
        !self.index.trim().is_empty() || !self.name.trim().is_empty() || !self.hidden.is_empty()
    }
}

/// An opened save and the masterlist links for its game.
#[derive(Clone)]
pub struct Analysis {
    pub path: PathBuf,
    pub save: SaveFile,
    pub links: Links,
}

impl Analysis {
    /// The screenshot the game took when saving: RGBA in Special Edition
    /// saves, RGB in the original. `None` when the data doesn't match the
    /// size in the header. Alpha is ignored, since the game doesn't use it.
    pub fn screenshot(&self) -> Option<egui::ColorImage> {
        let h = &self.save.header;
        let (w, ht) = (h.screenshot_width as usize, h.screenshot_height as usize);
        let channels = h.screenshot_bytes_per_pixel();
        let data = &self.save.screenshot;
        if w == 0 || ht == 0 || data.len() != w * ht * channels {
            return None;
        }
        let pixels = data
            .chunks_exact(channels)
            .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
            .collect();
        Some(egui::ColorImage::new([w, ht], pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(
        name: &str,
        state: State,
        is_cc: bool,
        is_forced: bool,
        load_order: Option<usize>,
    ) -> PluginStatus {
        PluginStatus {
            name: name.into(),
            state,
            is_cc,
            is_forced,
            is_master: false,
            load_order,
        }
    }

    fn setup() -> Setup {
        let mut links = Links::new();
        links.insert(
            "frostfall.esp".into(),
            vec!["https://example.com/frostfall".into()],
        );
        Setup {
            statuses: vec![
                status("Skyrim.esm", State::Enabled, false, true, Some(1)),
                status("ccBGSSSE001-Fish.esm", State::Enabled, true, true, Some(2)),
                status("TrueStormsSE.esp", State::Enabled, false, false, Some(3)),
                status("Off.esp", State::Disabled, false, false, None),
                status("Frostfall.esp", State::Missing, false, false, None),
            ],
            links,
        }
    }

    fn kind(name: &str, setup: Option<&Setup>, pending: &Pending) -> Kind {
        let links = setup.map(|s| s.links.clone()).unwrap_or_default();
        Row::new(name, None, setup, &links, pending).kind
    }

    #[test]
    fn rows_take_their_state_from_the_setup() {
        let setup = setup();
        let none = Pending::default();
        assert_eq!(kind("Skyrim.esm", Some(&setup), &none), Kind::BaseGame);
        assert_eq!(
            kind("ccbgssse001-fish.esm", Some(&setup), &none),
            Kind::CreationClub
        );
        assert_eq!(kind("truestormsse.esp", Some(&setup), &none), Kind::Active);
        assert_eq!(kind("Off.esp", Some(&setup), &none), Kind::Disabled);
        assert_eq!(
            kind("Frostfall.esp", Some(&setup), &none),
            Kind::NotInstalled
        );
        assert_eq!(kind("Unknown.esp", Some(&setup), &none), Kind::NotFound);
    }

    #[test]
    fn without_the_game_names_say_what_they_can() {
        let none = Pending::default();
        assert_eq!(kind("Dawnguard.esm", None, &none), Kind::BaseGame);
        assert_eq!(
            kind("ccQDRSSE001-SurvivalMode.esl", None, &none),
            Kind::CreationClub
        );
        assert_eq!(kind("TrueStormsSE.esp", None, &none), Kind::NotFound);
    }

    #[test]
    fn pending_changes_switch_rows_until_switched_back() {
        let setup = setup();
        let mut pending = Pending::default();
        pending.set(&setup, "OFF.ESP", true);
        pending.set(&setup, "TrueStormsSE.esp", false);
        assert_eq!(pending.len(), 2);
        let row = Row::new("Off.esp", None, Some(&setup), &setup.links, &pending);
        assert_eq!(
            (row.kind, row.toggle, row.pending),
            (Kind::Active, Some(true), true)
        );
        assert_eq!(
            kind("TrueStormsSE.esp", Some(&setup), &pending),
            Kind::Disabled
        );
        // Wanting what Plugins.txt already says is no change.
        pending.set(&setup, "truestormsse.esp", true);
        assert_eq!(pending.changes(), vec![("OFF.ESP".to_string(), true)]);
        // Plugins that always load, or aren't there, can't be switched.
        assert_eq!(
            Row::new("Skyrim.esm", None, Some(&setup), &setup.links, &pending).toggle,
            None
        );
        assert_eq!(
            Row::new("Frostfall.esp", None, Some(&setup), &setup.links, &pending).toggle,
            None
        );
    }

    #[test]
    fn the_load_order_lists_what_loads_first() {
        let rows = load_order_rows(&setup(), &Pending::default());
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Skyrim.esm",
                "ccBGSSSE001-Fish.esm",
                "TrueStormsSE.esp",
                "Frostfall.esp",
                "Off.esp"
            ]
        );
        assert_eq!(rows[2].index_text(), "02");
        assert_eq!(rows[3].index_text(), "");
    }

    #[test]
    fn each_column_filters() {
        let rows = load_order_rows(&setup(), &Pending::default());
        let shown = |filter: &PluginFilter| -> Vec<&str> {
            rows.iter()
                .filter(|r| filter.matches(r))
                .map(|r| r.name.as_str())
                .collect()
        };

        let mut filter = PluginFilter::default();
        assert!(!filter.is_active());
        assert_eq!(shown(&filter).len(), 5);

        filter.name = " storms ".into();
        assert_eq!(shown(&filter), ["TrueStormsSE.esp"]);

        filter = PluginFilter {
            index: "01".into(),
            ..Default::default()
        };
        assert_eq!(shown(&filter), ["ccBGSSSE001-Fish.esm"]);

        filter = PluginFilter::default();
        filter
            .hidden
            .extend([Kind::BaseGame, Kind::CreationClub, Kind::Active]);
        assert!(filter.is_active());
        assert_eq!(shown(&filter), ["Frostfall.esp", "Off.esp"]);
    }

    #[test]
    fn the_report_names_each_plugin_and_its_state() {
        let setup = setup();
        let mut pending = Pending::default();
        pending.set(&setup, "Off.esp", true);
        let text = report("Load order", &load_order_rows(&setup, &pending));
        assert!(text.starts_with("Load order\n\n00\tSkyrim.esm\tBase game\n"));
        assert!(text.contains("\tOff.esp\tActive (not applied)\n"));
        assert!(text.contains("\tFrostfall.esp\tNot installed: https://example.com/frostfall\n"));
    }
}
