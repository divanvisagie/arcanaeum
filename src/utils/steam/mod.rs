use std::path::{Path, PathBuf};

/// A Steam library folder, with the set of appids Steam considers installed
/// under it (per `libraryfolders.vdf`'s `"apps"` block for that library).
///
/// Appid ownership matters because a stale/leftover install directory can
/// exist under a library Steam no longer considers an app installed in
/// (e.g. after moving the install to a different library without deleting
/// files Steam didn't track); relying on directory existence alone can
/// then pick the wrong install.
pub struct SteamLibrary {
    pub path: PathBuf,
    pub apps: Vec<String>,
}

fn extract_quoted(s: &str) -> Option<&str> {
    let s = s.trim();
    let s = s.strip_prefix('"')?;
    let end = s.find('"')?;
    Some(&s[..end])
}

/// Parse a `libraryfolders.vdf` file into each library's path and owned
/// appids. This is a depth-tracking scan rather than a full VDF parser —
/// enough for the file's fixed shape:
///
/// ```text
/// "libraryfolders"
/// {
///     "0"
///     {
///         "path"  "..."
///         "apps"
///         {
///             "<appid>"  "<size>"
///         }
///     }
/// }
/// ```
pub fn parse_libraries(vdf: &str) -> Vec<SteamLibrary> {
    let mut libraries = Vec::new();
    let mut depth: i32 = 0;
    let mut library_depth: Option<i32> = None;
    let mut apps_depth: Option<i32> = None;
    let mut current_path: Option<PathBuf> = None;
    let mut current_apps: Vec<String> = Vec::new();
    let mut pending_key: Option<String> = None;

    for raw_line in vdf.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line == "{" {
            depth += 1;
            if let Some(key) = pending_key.take() {
                if library_depth.is_none() && !key.is_empty() && key.chars().all(|c| c.is_ascii_digit()) {
                    library_depth = Some(depth);
                    current_path = None;
                    current_apps = Vec::new();
                } else if library_depth == Some(depth - 1) && key == "apps" {
                    apps_depth = Some(depth);
                }
            }
            continue;
        }

        if line == "}" {
            if apps_depth == Some(depth) {
                apps_depth = None;
            } else if library_depth == Some(depth) {
                if let Some(path) = current_path.take() {
                    libraries.push(SteamLibrary {
                        path,
                        apps: std::mem::take(&mut current_apps),
                    });
                }
                library_depth = None;
            }
            depth -= 1;
            continue;
        }

        let Some(rest) = line.strip_prefix('"') else { continue };
        let Some(key_end) = rest.find('"') else { continue };
        let key = &rest[..key_end];
        let after = rest[key_end + 1..].trim();

        if after.is_empty() {
            // Bare `"key"` line — its block opens on the next line.
            pending_key = Some(key.to_string());
            continue;
        }

        if apps_depth == Some(depth) {
            current_apps.push(key.to_string());
        } else if library_depth == Some(depth) && key == "path" {
            if let Some(value) = extract_quoted(after) {
                current_path = Some(PathBuf::from(value.replace("\\\\", "/")));
            }
        }
    }

    libraries
}

/// Read and parse the `libraryfolders.vdf` under a Steam install root
/// (e.g. `~/.local/share/Steam`).
pub fn read_steam_libraries(steam_root: &Path) -> std::io::Result<Vec<SteamLibrary>> {
    let vdf_path = steam_root.join("steamapps/libraryfolders.vdf");
    let contents = std::fs::read_to_string(vdf_path)?;
    Ok(parse_libraries(&contents))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_steam_libraries() {
        let home = dirs::home_dir().expect("Could not determine home directory");
        let steam_root = home.join(".local/share/Steam");
        let libraries = read_steam_libraries(&steam_root).unwrap();
        assert!(!libraries.is_empty());
    }

    #[test]
    fn test_parse_libraries() {
        let vdf = r#""libraryfolders"
{
	"0"
	{
		"path"		"/home/user/.local/share/Steam"
		"apps"
		{
			"70"		"667402149"
		}
	}
	"1"
	{
		"path"		"/tank/games"
		"apps"
		{
			"489830"		"0"
		}
	}
}
"#;
        let libraries = parse_libraries(vdf);
        assert_eq!(libraries.len(), 2);
        assert_eq!(libraries[0].path, PathBuf::from("/home/user/.local/share/Steam"));
        assert_eq!(libraries[0].apps, vec!["70".to_string()]);
        assert_eq!(libraries[1].path, PathBuf::from("/tank/games"));
        assert_eq!(libraries[1].apps, vec!["489830".to_string()]);
    }
}
