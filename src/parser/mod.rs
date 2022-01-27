#![allow(unused_imports)]
#![allow(dead_code)]

use crate::parser::{
    header::read_header,
    utils::{read_charray, read_u32},
};

use self::header::Header;

mod header;
mod utils;

#[derive(Debug)]
pub struct SaveInfo {
    pub magic_string: String,
    pub header_size: u32,
    pub header: Header,
}

pub fn parse(buf: Vec<u8>) -> SaveInfo {
    let buf = buf.as_slice();
    let cursor = 0;
    let (magic_string, cursor) = read_charray(buf, cursor, 13);
    let (header_size, cursor) = read_u32(buf, cursor);
    let (header, cursor) = read_header(buf, cursor);
    println!("Cursor position at {:?}", cursor);
    SaveInfo {
        magic_string,
        header_size,
        header,
    }
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

    #[test]
    fn test_parse_u32() {
        let buf = get_file_buffer();
        let save_info = parse(buf);
        assert_eq!(save_info.header_size, 97);
    }

    #[test]
    fn test_parse_header() {
        let buf = get_file_buffer();
        let save_info = parse(buf);
        assert_eq!(save_info.header.version, 12);
        assert_eq!(save_info.header.save_number, 3);
        assert_eq!(save_info.header.player_name, "Aluna Messana");
        assert_eq!(save_info.header.player_level, 1);
        assert_eq!(save_info.header.player_location, "Old Hroldan Inn");
        assert_eq!(save_info.header.game_date, "000.11.05");
        assert_eq!(save_info.header.player_race_editor_id, "RedguardRace");
    }
}
