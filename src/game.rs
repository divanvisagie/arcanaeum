//! Finding the game and the user's files through skyplug-core: Steam Play
//! (Proton) on Linux, the user's own folders on Windows (ADR-0005).

use std::path::{Path, PathBuf};

use skyplug_core::{steam, GameInstall, GamePaths, DEFAULT_APPID};

/// Skyrim Special Edition's Steam app id.
pub const APPID: &str = DEFAULT_APPID;

/// The install in `game_folder` when one was picked, else the one Steam
/// says owns the app id.
pub fn find(game_folder: Option<&Path>) -> Result<GameInstall, String> {
    match game_folder {
        Some(dir) => GameInstall::from_game_dir(dir).map_err(|e| format!("{e:#}")),
        None => steam::find_game_install(APPID).map_err(|e| format!("{e:#}")),
    }
}

pub fn paths(install: &GameInstall) -> GamePaths {
    GamePaths::new(install, APPID)
}

/// `My Games/Skyrim Special Edition`, holding the saves and the game's ini
/// files, for Steam's install.
pub fn documents_dir() -> Option<PathBuf> {
    find(None).ok().map(|install| install.documents_dir(APPID))
}

/// Whether Skyrim is running. The game rewrites Plugins.txt when it quits,
/// so changes applied while it runs would be lost.
pub fn is_running() -> bool {
    #[cfg(target_os = "linux")]
    {
        // Proton runs the game as a process named after its exe.
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return false;
        };
        entries.flatten().any(|entry| {
            std::fs::read_to_string(entry.path().join("comm"))
                .is_ok_and(|comm| comm.trim().eq_ignore_ascii_case("SkyrimSE.exe"))
        })
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq SkyrimSE.exe", "/NH"])
            .output()
            .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains("SkyrimSE.exe"))
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        false
    }
}
