//! Settings kept between runs in `~/.config/arcanaeum/config.toml`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use skyplug_core::GamePaths;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The game folder picked with File > Set Game Folder; Steam's install
    /// when unset.
    pub game_folder: Option<PathBuf>,
    /// The save folder picked with File > Open Save Folder; the game's
    /// when unset.
    pub save_folder: Option<PathBuf>,
    pub skin: Option<String>,
    pub system_title_bar: bool,
}

pub fn dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("arcanaeum"))
}

impl Config {
    pub fn load() -> Self {
        dir()
            .and_then(|d| std::fs::read_to_string(d.join("config.toml")).ok())
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(dir) = dir() else { return };
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(text) = toml::to_string(self) {
            let _ = std::fs::write(dir.join("config.toml"), text);
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
        let config = Config {
            game_folder: Some(PathBuf::from("/games/Skyrim Special Edition")),
            save_folder: Some(PathBuf::from("/saves")),
            skin: Some("Dark Orange".into()),
            system_title_bar: true,
        };
        let text = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }

    #[test]
    fn missing_keys_take_defaults() {
        assert_eq!(toml::from_str::<Config>("").unwrap(), Config::default());
    }
}
