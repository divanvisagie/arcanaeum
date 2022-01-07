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

fn main() -> io::Result<()> {
    let mut file = std::fs::File::open("test.ess")?;
    let br = file.by_ref();

    let magic = read_string_of_size(br,13);
    println!("magic: {:?}", magic);

    

    let header_size = read_u32(br);
    println!("header size: {:?}", header_size);

    let version = read_u32(br);
    println!("version {:?}", version);


    let save_number = read_u32(br);
    println!("saveNumber {:?}", save_number);

    let player_name = read_wstring(br);
    println!("player name: {:?}", player_name);

    let player_level = read_u32(br);
    println!("player level: {:?}", player_level);

    let player_location = read_wstring(br);
    println!("player location: {:?}", player_location);

    let sp = br.stream_position()?;
    println!("stream position: {:?}", sp);

    Ok(())
}
