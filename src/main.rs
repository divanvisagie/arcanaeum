/**
 * Author: Divan Visagie
 * https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format#Header
 * https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions
*/
use std::io;
use std::io::Read;
use std::io::Seek;

use crate::sktypes::info_item::InfoItem;
use crate::sktypes::types::SkType;

mod sktypes;

fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    let res = rfd::FileDialog::new()
        .add_filter("Elder Scrolls Save", &["ess"])
        .set_directory("./input")
        .pick_file()
        .unwrap()
        .into_os_string();

    let mut file = std::fs::File::open(res)?;

    let file = file.by_ref();

    let items= [
        InfoItem::new(file,"magic", SkType::Char13),
        InfoItem::new(file, "header_size", SkType::UInt32),
        // Start Header Section
        InfoItem::new(file,"version", SkType::UInt32),
        InfoItem::new(file,"save_number", SkType::UInt32),
        InfoItem::new(file,"player_name", SkType::WString),
        InfoItem::new(file,"player_level", SkType::UInt32),
        InfoItem::new(file,"player_location", SkType::WString),
        InfoItem::new(file,"game_date", SkType::WString),
        InfoItem::new(file,"player_race_editor_id", SkType::WString),
        InfoItem::new(file,"player_sex", SkType::UInt16), // 0 = male, 1 = female
        InfoItem::new(file,"player_current_experience", SkType::Float32),
        InfoItem::new(file,"player_level_up_exp", SkType::Float32),
        InfoItem::new(file,"file_time", SkType::Unknown), // TODO: temp solution until FILETIME is implemented
        InfoItem::new(file,"shot_width", SkType::UInt32),
        InfoItem::new(file,"shot_height", SkType::UInt32),
        InfoItem::new(file,"compression", SkType::UInt16), //0 = None, 1 = zlib, 2 = lz4
        // End Header Section
        InfoItem::new(file,"screenshot_data", SkType::UInt8),
        InfoItem::new(file,"uncompressed_length", SkType::UInt32),
        InfoItem::new(file,"compressed_length", SkType::UInt32),
        InfoItem::new(file,"form_version", SkType::UInt8),
        InfoItem::new(file,"plugin_info_size", SkType::UInt32),
        // Start Plugin Info Section
        InfoItem::new(file,"pluginCount", SkType::UInt8),
        //    InfoItem::new("plugins", SkType::WString) //TODO: Implement wstring[plugincount]
    ];

    for i in items {
        i.print_value();
    }

    let sp = file.stream_position()?;
    println!("========================\nposition in file: {:?}", sp);

    // Pause execution
    let mut stdin = io::stdin();
    println!("Press any key to exit...");
    let _ = stdin.read(&mut [0u8]).unwrap();

    Ok(())
}
