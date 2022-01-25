#![allow(unused_imports)]
#![allow(dead_code)]
use skyrim_savegame::SaveFile;

#[derive(Debug)]
pub struct SaveInfo {
    pub magic_string: String,
}

fn read_string(buffer: Vec<u8>, start: usize, end: usize) -> (String, usize) {
    let chunk = &buffer[start..end];
    let s = match std::str::from_utf8(chunk) {
        Ok(s) => s.to_string(),
        Err(_) => {
            tracing::error!("Failed to read string from {start} to {end}");
            "".to_string()
        }
    };
    (s, end)
}

pub fn parse(buf: Vec<u8>) -> SaveInfo {
    let cursor = 0;
    let (magic_string, cursor) = read_string(buf, cursor, 13);
    println!("Cursor position at {:?}", cursor);
    SaveInfo { magic_string }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::{io::Read, path::PathBuf};

    fn get_file_buffer() -> Vec<u8> {
        let path_buf = PathBuf::from("./input/test_skse.ess");
        let mut file = std::fs::File::open(path_buf).unwrap();

        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_parse_magic_string() {
        let buf = get_file_buffer();
        let save_info = parse(buf);
        assert_eq!(save_info.magic_string, "TESV_SAVEGAME");
    }
}
