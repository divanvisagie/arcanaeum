use std::collections::HashSet;
use std::path::Path;

const PLUGIN_EXTENSIONS: [&str; 3] = ["esp", "esm", "esl"];

/// Scan the game's `Data` directory for `.esp`/`.esm`/`.esl` files actually
/// present on disk, regardless of how they got there (manually copied in,
/// or placed by a mod manager). Names are lowercased so lookups can be
/// done case-insensitively, since manually installed files often don't
/// match the casing recorded in a save.
pub fn scan_installed_plugins(data_dir: &Path) -> HashSet<String> {
    let mut installed = HashSet::new();

    let Ok(entries) = std::fs::read_dir(data_dir) else {
        tracing::error!("Could not read Data directory at {:?}", data_dir);
        return installed;
    };

    for entry in entries {
        let Ok(entry) = entry else { continue };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_lowercase();
        let ext = name.rsplit('.').next().unwrap_or("");
        if PLUGIN_EXTENSIONS.contains(&ext) {
            installed.insert(name);
        }
    }

    installed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_only_plugin_files() {
        let dir = std::env::temp_dir().join("arcanaeum_data_scanner_test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        fs::write(dir.join("TrueStormsSE.esp"), b"").unwrap();
        fs::write(dir.join("Skyrim.esm"), b"").unwrap();
        fs::write(dir.join("SomeMod.ESL"), b"").unwrap();
        fs::write(dir.join("readme.txt"), b"").unwrap();
        fs::create_dir_all(dir.join("textures")).unwrap();

        let installed = scan_installed_plugins(&dir);

        assert_eq!(installed.len(), 3);
        assert!(installed.contains("truestormsse.esp"));
        assert!(installed.contains("skyrim.esm"));
        assert!(installed.contains("somemod.esl"));

        fs::remove_dir_all(&dir).unwrap();
    }
}
