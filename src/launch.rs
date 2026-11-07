//! Starting the game from the app (ADR-0013). Linux only: the game runs
//! through whatever launch command the install names, which owns Proton, the
//! prefix and anything else about how the game is run; this adds the choice
//! of what to start and the environment flags (an FPS counter, say).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::Install;

/// What to start.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// SKSE's loader, for a modded game.
    Skse,
    /// `SkyrimSE.exe` itself.
    Game,
    /// Bethesda's launcher, for resolution and quality.
    Launcher,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Skse, Mode::Game, Mode::Launcher];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Skse => "Skyrim with SKSE",
            Mode::Game => "Skyrim without SKSE",
            Mode::Launcher => "Bethesda Launcher",
        }
    }

    /// The argument the launch command takes for this (none for SKSE, its default).
    fn argument(self) -> Option<&'static str> {
        match self {
            Mode::Skse => None,
            Mode::Game => Some("--vanilla"),
            Mode::Launcher => Some("--launcher"),
        }
    }
}

/// `DXVK_HUD` value that draws the frame rate.
const FPS_HUD: &str = "fps";

/// The `KEY=VALUE` words of `text`, split on whitespace. A word with no `=` or
/// an empty key is returned as an error.
pub fn parse_env(text: &str) -> Result<Vec<(String, String)>, String> {
    text.split_whitespace()
        .map(|word| match word.split_once('=') {
            Some((key, value)) if !key.is_empty() => Ok((key.to_string(), value.to_string())),
            _ => Err(format!("\"{word}\" isn't KEY=VALUE.")),
        })
        .collect()
}

/// The launch command's name, as the GOG setup's `install.sh` links it into `PATH`.
const SCRIPT: &str = "skyrim-gog";

/// The launch command to use: the one set in Settings, else a `skyrim-gog` in
/// the `bin` folder beside the game's folder, else one on `PATH`.
pub fn launcher(install: &Install) -> Option<PathBuf> {
    if let Some(set) = &install.launcher {
        return Some(set.clone());
    }
    let beside = install
        .game_folder
        .as_deref()
        .and_then(Path::parent)
        .map(|root| root.join("bin").join(SCRIPT));
    let on_path = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join(SCRIPT));
    beside.into_iter().chain(on_path).find(|p| p.is_file())
}

/// The command that starts `mode` for this install, or why it can't.
pub fn command(install: &Install, mode: Mode) -> Result<Command, String> {
    let launcher = launcher(install)
        .ok_or("No launch command is set for this tab. Set one in Settings.")?;
    let launcher = launcher.as_path();
    if !launcher.is_file() {
        return Err(format!("{} isn't a file.", launcher.display()));
    }
    let env = parse_env(&install.launch_env)?;
    let mut command = Command::new(launcher);
    command.args(mode.argument());
    if install.fps_counter && !env.iter().any(|(key, _)| key == "DXVK_HUD") {
        command.env("DXVK_HUD", FPS_HUD);
    }
    command.envs(env);
    if let Some(folder) = install.game_folder.as_deref().filter(|f| f.is_dir()) {
        command.current_dir(folder);
    }
    Ok(command)
}

/// Starts `mode`, leaving it running in a group of its own so closing the app
/// doesn't take the game with it. The launch command logs its own output.
pub fn start(install: &Install, mode: Mode) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let mut command = command(install, mode)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let program = Path::new(command.get_program()).display().to_string();
    let mut child = command.spawn().map_err(|e| format!("Couldn't run {program}: {e}"))?;
    // Reap it when it ends, so it doesn't stay a zombie while the app runs.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn install(env: &str, fps: bool) -> Install {
        Install {
            // Any file will do: the command isn't run.
            launcher: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")),
            launch_env: env.into(),
            fps_counter: fps,
            ..Install::default()
        }
    }

    fn envs(command: &Command) -> Vec<(String, String)> {
        command
            .get_envs()
            .filter_map(|(k, v)| Some((k.to_str()?.into(), v?.to_str()?.into())))
            .collect()
    }

    #[test]
    fn modes_pass_the_arguments_of_the_launch_command() {
        let args = |mode| {
            let c = command(&install("", false), mode).unwrap();
            c.get_args().map(OsStr::to_owned).collect::<Vec<_>>()
        };
        assert!(args(Mode::Skse).is_empty());
        assert_eq!(args(Mode::Game), ["--vanilla"]);
        assert_eq!(args(Mode::Launcher), ["--launcher"]);
    }

    #[test]
    fn the_fps_counter_sets_dxvk_hud() {
        let c = command(&install("", true), Mode::Skse).unwrap();
        assert_eq!(envs(&c), [("DXVK_HUD".into(), "fps".into())]);
        assert!(envs(&command(&install("", false), Mode::Skse).unwrap()).is_empty());
    }

    #[test]
    fn extra_flags_are_added_and_win_over_the_checkbox() {
        let c = command(&install("MANGOHUD=1 DXVK_HUD=full", true), Mode::Skse).unwrap();
        let envs = envs(&c);
        assert!(envs.contains(&("MANGOHUD".into(), "1".into())));
        assert!(envs.contains(&("DXVK_HUD".into(), "full".into())));
        assert_eq!(envs.len(), 2);
    }

    #[test]
    fn bad_flags_and_a_missing_command_say_so() {
        assert!(parse_env("FOO").is_err() && parse_env("=1").is_err());
        assert_eq!(parse_env("A=1 B=").unwrap().len(), 2);
        assert!(command(&install("oops", false), Mode::Skse).is_err());
    }

    #[test]
    fn a_script_beside_the_game_folder_is_found() {
        let root = std::env::temp_dir().join("arcanaeum_launcher_detect");
        let game = root.join("Skyrim Special Edition");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(root.join("bin").join(SCRIPT), "").unwrap();
        let install = Install { game_folder: Some(game), ..Install::default() };
        assert_eq!(launcher(&install), Some(root.join("bin").join(SCRIPT)));
        let set = Install { launcher: Some(PathBuf::from("/mine")), ..install };
        assert_eq!(launcher(&set), Some(PathBuf::from("/mine")), "Settings wins");
    }
}
