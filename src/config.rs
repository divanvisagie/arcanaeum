//! Settings kept between runs in `~/.config/arcanaeum/config.toml`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use skyplug_core::GamePaths;

use crate::game;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The game installs, one per tab (ADR-0008). Never empty once loaded.
    pub installs: Vec<Install>,
    /// The tab shown.
    pub shown: usize,
    pub skin: Option<String>,
    pub system_title_bar: bool,
    /// Before tabs, one install's folders were kept at the top level;
    /// `load` moves them into the first install.
    #[serde(skip_serializing)]
    game_folder: Option<PathBuf>,
    #[serde(skip_serializing)]
    save_folder: Option<PathBuf>,
}

/// A copy of the game, shown in its own tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Install {
    /// The tab's title.
    pub name: String,
    /// The folder holding SkyrimSE.exe.
    pub game_folder: Option<PathBuf>,
    /// The Wine prefix the game runs in (the folder holding `drive_c`):
    /// Steam's Proton prefix, or GOG's under Wine or umu, say. Unused on
    /// Windows.
    pub wine_prefix: Option<PathBuf>,
    /// Where the saves are read from; the game's own when unset.
    pub save_folder: Option<PathBuf>,
}

impl Default for Install {
    fn default() -> Self {
        Self::named("Skyrim")
    }
}

/// No installs: `load` fills one in.
impl Default for Config {
    fn default() -> Self {
        Self {
            installs: Vec::new(),
            ..Self::with(Install::default())
        }
    }
}

pub fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("arcanaeum"))
}

impl Config {
    /// Just this install, shown, and defaults for the rest.
    pub fn with(install: Install) -> Self {
        Self {
            installs: vec![install],
            shown: 0,
            skin: None,
            system_title_bar: false,
            game_folder: None,
            save_folder: None,
        }
    }

    /// The saved config. With no installs yet (a first run, or a config
    /// from before tabs), the first is filled in with Steam's copy, if any.
    pub fn load() -> Self {
        let config = dir()
            .and_then(|d| std::fs::read_to_string(d.join("config.toml")).ok())
            .and_then(|t| toml::from_str::<Config>(&t).ok())
            .unwrap_or_default();
        let first_run = config.installs.is_empty();
        let mut config = config.normalized();
        if first_run && config.installs[0].game_folder.is_none() {
            if let Ok((game_folder, wine_prefix)) = game::steam_copy() {
                let first = &mut config.installs[0];
                first.name = "Steam".into();
                first.game_folder = Some(game_folder);
                first.wine_prefix = wine_prefix;
            }
        }
        config
    }

    /// Moves the folders of a config from before tabs into its first
    /// install, and keeps at least one install, with `shown` one of them.
    fn normalized(mut self) -> Self {
        if self.installs.is_empty() {
            self.installs.push(Install::default());
        }
        let first = &mut self.installs[0];
        first.game_folder = first.game_folder.take().or(self.game_folder.take());
        first.save_folder = first.save_folder.take().or(self.save_folder.take());
        self.shown = self.shown.min(self.installs.len() - 1);
        self
    }

    pub fn save(&self) {
        let Some(dir) = dir() else { return };
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(text) = toml::to_string(self) {
            let _ = std::fs::write(dir.join("config.toml"), text);
        }
    }

    /// The install in the tab shown.
    pub fn install(&self) -> &Install {
        &self.installs[self.shown]
    }

    pub fn install_mut(&mut self) -> &mut Install {
        &mut self.installs[self.shown]
    }
}

impl Install {
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_string(),
            game_folder: None,
            wine_prefix: None,
            save_folder: None,
        }
    }

    /// The chosen save folder, else the game's, else where Windows keeps it.
    pub fn save_folder(&self, game: Option<&GamePaths>) -> PathBuf {
        if let Some(folder) = &self.save_folder {
            return folder.clone();
        }
        match game {
            Some(paths) => paths.saves_dir.clone(),
            None => dirs::document_dir()
                .unwrap_or_default()
                .join("My Games/Skyrim Special Edition/Saves"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let mut config = Config::with(Install {
            name: "GOG".into(),
            game_folder: Some(PathBuf::from("/games/Skyrim Special Edition")),
            wine_prefix: Some(PathBuf::from("/games/prefix")),
            save_folder: Some(PathBuf::from("/saves")),
        });
        config.installs.push(Install::default());
        config.shown = 1;
        config.skin = Some("Dark Orange".into());
        config.system_title_bar = true;
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap().normalized(), config);
    }

    #[test]
    fn missing_keys_take_defaults() {
        let config = toml::from_str::<Config>("").unwrap();
        assert!(config.installs.is_empty(), "so load fills in Steam's copy");
        assert_eq!(config.normalized(), Config::with(Install::default()));
    }

    #[test]
    fn folders_from_before_tabs_move_into_the_first_install() {
        let old = "game_folder = \"/games/Skyrim\"\nsave_folder = \"/saves\"\nskin = \"Dark Orange\"\n";
        let config = toml::from_str::<Config>(old).unwrap().normalized();
        let install = Install {
            game_folder: Some(PathBuf::from("/games/Skyrim")),
            save_folder: Some(PathBuf::from("/saves")),
            ..Install::default()
        };
        assert_eq!(config.installs, vec![install]);
        assert_eq!(config.skin.as_deref(), Some("Dark Orange"));
        let saved: toml::Table = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert!(!saved.contains_key("game_folder") && !saved.contains_key("save_folder"));
    }

    #[test]
    fn shown_stays_on_an_install() {
        let config = toml::from_str::<Config>("shown = 3\n[[installs]]\nname = \"A\"\n").unwrap();
        assert_eq!(config.normalized().shown, 0);
    }
}
