/**
 * Author: Divan Visagie
 * https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format#Header
 * https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions
*/
use std::io;
use std::io::Read;
use std::io::Seek;

use crate::sktypes::{read_f32, read_string_of_size, read_u16, read_u32, read_u8, read_wstring};

mod sktypes;

#[derive(PartialEq, Eq)]
enum SkType {
    UInt8,
    UInt32,
    UInt16,
    Float32,
    WString,
    Chars,
}

struct InfoItem {
    name: String,
    size: usize,
    sk_type: SkType,
}

impl InfoItem {
    fn new(name: &str, sk_type: SkType) -> InfoItem {
        InfoItem {
            name: name.to_string(),
            size: 0,
            sk_type,
        }
    }

    fn new_with_size(name: &str, size: usize, sk_type: SkType) -> InfoItem {
        InfoItem {
            name: name.to_string(),
            size,
            sk_type,
        }
    }

    fn print_value(&self, br: &mut std::fs::File) {
        match self.sk_type {
            SkType::Chars => {
                let str = read_string_of_size(br, self.size as u32);
                println!("{}:    {:?}", self.name, str);
            }
            SkType::UInt8 => {
                let u = read_u8(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::UInt16 => {
                let u = read_u16(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::UInt32 => {
                let u = read_u32(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::Float32 => {
                let u = read_f32(br);
                println!("{}:    {:?}", self.name, u);
            }
            SkType::WString => {
                let str = read_wstring(br);
                println!("{}:    {:?}", self.name, str);
            }
        }
    }
}

fn main() -> io::Result<()> {
    let res = rfd::FileDialog::new()
        .add_filter("Elder Scrolls Save", &["ess"])
        .set_directory("./input")
        .pick_file()
        .unwrap()
        .into_os_string();

    let mut file = std::fs::File::open(res)?;

    let br = file.by_ref();

    let items = [
        InfoItem::new_with_size("magic", 13, SkType::Chars),
        InfoItem::new("header_size", SkType::UInt32),
        // Start Header Section
        InfoItem::new("version", SkType::UInt32),
        InfoItem::new("save_number", SkType::UInt32),
        InfoItem::new("player_name", SkType::WString),
        InfoItem::new("player_level", SkType::UInt32),
        InfoItem::new("player_location", SkType::WString),
        InfoItem::new("game_date", SkType::WString),
        InfoItem::new("player_race_editor_id", SkType::WString),
        InfoItem::new("player_sex", SkType::UInt16), // 0 = male, 1 = female
        InfoItem::new("player_current_experience", SkType::Float32),
        InfoItem::new("player_level_up_exp", SkType::Float32),
        InfoItem::new_with_size("file_time", 8, SkType::Chars), // TODO: temp solution until FILETIME is implemented
        InfoItem::new("shot_width", SkType::UInt32),
        InfoItem::new("shot_height", SkType::UInt32),
        InfoItem::new("compression", SkType::UInt16), //0 = None, 1 = zlib, 2 = lz4
        // End Header Section
        InfoItem::new("screenshot_data", SkType::UInt8),
        InfoItem::new("uncompressed_length", SkType::UInt32),
        InfoItem::new("compressed_length", SkType::UInt32),
        InfoItem::new("form_version", SkType::UInt8),
        InfoItem::new("plugin_info_size", SkType::UInt32),
        // Start Plugin Info Section
        InfoItem::new("pluginCount", SkType::UInt8),
        //    InfoItem::new("plugins", SkType::WString) //TODO: Implement wstring[plugincount]
    ];

    for i in items {
        i.print_value(br);
    }

    let sp = br.stream_position()?;
    println!("========================\nposition in file: {:?}", sp);

    // Pause execution
    let mut stdin = io::stdin();
    println!("Press any key to exit...");
    let _ = stdin.read(&mut [0u8]).unwrap();

    Ok(())
}
