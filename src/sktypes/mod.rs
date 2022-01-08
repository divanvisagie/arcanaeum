use std::{io::{Read, Cursor}, mem::size_of};

use byteorder::{LittleEndian, ReadBytesExt};

pub mod info_item;
pub mod types;


pub fn read_string_of_size(br: &mut std::fs::File, size: u32) -> String {
    let mut str = String::new();
    br.take(size as u64)
        .read_to_string(&mut str)
        .map_err(|err| println!("{:?}", err))
        .ok();
    str
}

pub fn read_u32(br: &mut std::fs::File) -> u32 {
    let chunk_size = size_of::<u32>();

    let mut buffer = Vec::with_capacity(chunk_size);

    br.take(chunk_size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    let mut rdr = Cursor::new(buffer);
    rdr.read_u32::<LittleEndian>().unwrap()
}

pub fn read_f32(br: &mut std::fs::File) -> f32 {
    let chunk_size = size_of::<f32>();

    let mut buffer = Vec::with_capacity(chunk_size);

    br.take(chunk_size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    let mut rdr = Cursor::new(buffer);
    rdr.read_f32::<LittleEndian>().unwrap()
}

pub fn read_u16(br: &mut std::fs::File) -> u16 {
    let chunk_size = size_of::<u16>();

    let mut buffer = Vec::with_capacity(chunk_size);

    br.take(chunk_size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    let mut rdr = Cursor::new(buffer);
    rdr.read_u16::<LittleEndian>().unwrap()
}

pub fn read_u8(br: &mut std::fs::File) -> u8 {
    let chunk_size = size_of::<u8>();

    let mut buffer = Vec::with_capacity(chunk_size);

    br.take(chunk_size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    buffer[0]
}


pub fn read_wstring(br: &mut std::fs::File) -> String {
    let size = read_u16(br);
    let mut str = String::new();
    br.take(size as u64)
        .read_to_string(&mut str)
        .map_err(|err| println!("{:?}", err))
        .ok();
    str
}
