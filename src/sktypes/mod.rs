use std::{io::{Read, Cursor, Seek}, mem::size_of};

use byteorder::{LittleEndian, ReadBytesExt};

pub mod types;

fn read_buffer(file: &mut std::fs::File, size: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(size);
    let sp = file.stream_position().ok();

    file.take(size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    tracing::info!("Read {:?} bytes from position: {:?}:  Buffer {:?}", size , sp.unwrap(), buffer);
    buffer
}

pub fn read_string_of_size(br: &mut std::fs::File, size: u32) -> String {
    let mut str = String::new();
    br.take(size as u64)
        .read_to_string(&mut str)
        .map_err(|err| println!("{:?}", err))
        .ok();
    str
}

pub fn read_u16(br: &mut std::fs::File) -> u16 {
    let chunk_size = size_of::<u16>();
    let buffer = read_buffer(br, chunk_size);
    let mut rdr = Cursor::new(buffer);
    rdr.read_u16::<LittleEndian>().unwrap()
}

