use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

pub fn read_steam_library_folders(steam_path: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut libraries = Vec::new();

    let file_path = steam_path.join("steamapps/libraryfolders.vdf");
    let file = File::open(&file_path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.contains("\"path\"") {
            let parts: Vec<&str> = line.split("\"").collect();
            if parts.len() > 3 {
                libraries.push(PathBuf::from(parts[3]));
            }
        }
    }

    Ok(libraries)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_steam_library_folders() {
        let steam_path = PathBuf::from("C:\\Program Files (x86)\\Steam");
        let libraries = read_steam_library_folders(&steam_path).unwrap();
        assert!(libraries.len() > 0);
    }
}
