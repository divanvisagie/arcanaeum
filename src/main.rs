/**
 * https://en.uesp.net/wiki/Skyrim_Mod:Save_File_Format#Header
 * https://en.uesp.net/wiki/Skyrim_Mod:File_Format_Conventions
*/


use byteorder::{ReadBytesExt, LittleEndian};
use std::io::{Cursor, Seek};
use std::{
    io::{self, Read},
    mem::size_of,
};


fn read_string_of_size(br: &mut std::fs::File, size: u32) -> String {
    let mut str = String::new();
    br.take(size as u64)
        .read_to_string(&mut str)
        .map_err(|err| println!("{:?}", err))
        .ok();
    str
}

fn read_u32(br: &mut std::fs::File) -> u32 {
    let chunk_size = size_of::<u32>();

    let mut buffer = Vec::with_capacity(chunk_size);

    br.take(chunk_size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    let mut rdr = Cursor::new(buffer);
    rdr.read_u32::<LittleEndian>().unwrap()
}

fn read_u16(br: &mut std::fs::File) -> u16 {
    let chunk_size = size_of::<u16>();

    let mut buffer = Vec::with_capacity(chunk_size);

    br.take(chunk_size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    let mut rdr = Cursor::new(buffer);
    rdr.read_u16::<LittleEndian>().unwrap()
}

fn read_wstring(br: &mut std::fs::File) -> String {
    let size = read_u16(br);
    let mut str = String::new();
    br.take(size as u64)
        .read_to_string(&mut str)
        .map_err(|err| println!("{:?}", err))
        .ok();
    str
}

#[derive(PartialEq, Eq)]
enum SkType {
    SkU32,
    SKU16,
    SkWstring,
    SkChars
}

struct InfoItem {
    name: String,
    size: usize,
    sk_type: SkType
}

impl InfoItem {
    fn new(name: &str, sk_type: SkType) -> InfoItem {
        InfoItem {
            name: name.to_string(),
            size: 0,
            sk_type
        }
    }

    fn new_with_size(name: &str, size: usize, sk_type: SkType) -> InfoItem {
        InfoItem {
            name: name.to_string(),
            size,
            sk_type
        }
    }

    fn print_value(&self,br: &mut std::fs::File){
        if self.sk_type == SkType::SkChars {
            let str = read_string_of_size(br,self.size as u32);
            println!("{}:    {:?}", self.name ,str);
        }
        if self.sk_type == SkType::SkU32 {
            let u  = read_u32(br);
            println!("{}:    {:?}", self.name, u);
        }

        if self.sk_type == SkType::SKU16 {
            let u  = read_u16(br);
            println!("{}:    {:?}", self.name, u);
        }

        if self.sk_type == SkType::SkWstring {
            let str = read_wstring(br);
            println!("{}:    {:?}", self.name, str);
        }
    }

}

fn main() -> io::Result<()> {
    let mut file = std::fs::File::open("test.ess")?;
    let br = file.by_ref();


    let items = [
       InfoItem::new_with_size("magic",13, SkType::SkChars),
       InfoItem::new("header_size", SkType::SkU32),
       InfoItem::new("version", SkType::SkU32),
       InfoItem::new("save_number", SkType::SkU32),
       InfoItem::new("player_name", SkType::SkWstring),
       InfoItem::new("player_level", SkType::SkU32),
       InfoItem::new("player_location", SkType::SkWstring),
       InfoItem::new("game_date", SkType::SkWstring),
       InfoItem::new("player_race_editor_id", SkType::SkWstring),
       InfoItem::new("player_sex", SkType::SKU16)
    ];

    for i in items {
        i.print_value(br);
    }

    let sp = br.stream_position()?;
    println!("========================\nposition in file: {:?}", sp);

    Ok(())
}
