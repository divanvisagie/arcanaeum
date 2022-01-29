#![allow(unused_imports)]
#![allow(dead_code)]
use lz4_flex::decompress;
use std::fmt::{self, Formatter};

use eframe::egui::epaint::text::cursor;

use crate::parser::{
    header::read_header,
    plugin_info::read_plugin_info,
    utils::{read_bytes, read_charray, read_u32, read_u8},
};

use self::{header::Header, plugin_info::PluginInfo};

mod header;
mod plugin_info;
mod utils;

pub struct SaveInfo {
    pub magic_string: String,
    pub header_size: u32,
    pub header: Header,
    pub screenshot_data: Vec<u8>,
    pub uncompressed_length: u32,
    pub compressed_length: u32,
    pub form_version: u8,
    pub plugin_info_size: u32,
    pub plugin_info: PluginInfo,
}

impl fmt::Debug for SaveInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "magic_string: {:?} \n\
            header_size: {:?} \n\
            header: {:?} \n\
            uncompressed_length: {:?} \n\
            compressed_length: {:?} \n\
            form_version: {:?} \n\
            plugin_info_size: {:?} \n\
            plugin_info: {:?} \n\
            ",
            self.magic_string,
            self.header_size,
            self.header,
            self.uncompressed_length,
            self.compressed_length,
            self.form_version,
            self.plugin_info_size,
            self.plugin_info,
        )
    }
}

fn get_decompressed_buffer(
    buf: &[u8],
    compression_type: u16,
    cursor: usize,
    size: usize,
) -> Vec<u8> {
    let buf = match compression_type {
        0 => {
            println!("File is not compressed");
            buf[cursor..buf.len()].to_vec()
        }
        1 => panic!("TODO: Implement zlib decompression"),
        2 => {
            let slice = &buf[cursor..buf.len()];
            let decompressed = decompress(&slice, size)
                .expect("Could not decompress body")
                .clone();
            decompressed
        }
        _ => {
            //panic!("Unknown compression type: {:?}", compression_type),
            // Fail silently and return old buf
            tracing::info!("Unsupported compression type, returning buffer as is");
            buf[cursor..buf.len()].to_vec()
        }
    };
    buf
}

pub fn parse(buf: Vec<u8>) -> SaveInfo {
    let buf = buf.as_slice();
    let cursor = 0;
    let (magic_string, cursor) = read_charray(buf, cursor, 13);
    let (header_size, cursor) = read_u32(buf, cursor);
    let (header, cursor) = read_header(buf, cursor);

    let screenshot_data_size = (4 * header.screenshot_width * header.screenshot_height) as usize;
    let (screenshot_data, cursor) = read_bytes(buf, cursor, screenshot_data_size);
    let (uncompressed_length, cursor) = read_u32(buf, cursor);
    let (compressed_length, cursor) = read_u32(buf, cursor);

    tracing::info!("compressed: {compressed_length} uncompressed: {uncompressed_length}");

    /*
     * Need to check for compression before continuing
     */
    let buf = get_decompressed_buffer(
        buf,
        header.compression_type,
        cursor,
        uncompressed_length as usize,
    );
    let buf = buf.as_slice();

    let (form_version, cursor) = read_u8(buf, 0); //we need to start the cursor from 0 again
    let (plugin_info_size, cursor) = read_u32(buf, cursor);
    let (plugin_info, cursor) = read_plugin_info(buf, cursor);

    println!("Cursor position at {:?}", cursor);
    SaveInfo {
        magic_string,
        header_size,
        header,
        screenshot_data: screenshot_data.to_vec(),
        uncompressed_length,
        compressed_length,
        form_version,
        plugin_info_size,
        plugin_info,
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
        println!("{:?}", save_info);
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
