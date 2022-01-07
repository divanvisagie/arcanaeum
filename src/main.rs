use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;
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

// fn read_buffer(br: &mut std::fs::File) -> u32 {
//     let chunk_size = size_of::<u32>();

//     println!("Reading buffer for {:?} bytes", chunk_size as u64);

//     let mut buffer = Vec::with_capacity(chunk_size);

//     br.take(chunk_size as u64)
//         .read_to_end(&mut buffer)
//         .map_err(|err| println!("{:?}", err))
//         .ok();

//     println!("Read buffer {:?}", buffer);

//     let mut rdr = Cursor::new(buffer);
//     rdr.read_u32::<BigEndian>().unwrap()
// }


fn read_u32(br: &mut std::fs::File) -> u32 {
    let chunk_size = size_of::<u32>();

    println!("Reading buffer for {:?} bytes", chunk_size as u64);

    let mut buffer = Vec::with_capacity(chunk_size);

    br.take(chunk_size as u64)
        .read_to_end(&mut buffer)
        .map_err(|err| println!("{:?}", err))
        .ok();

    println!("Read buffer {:?}", buffer);

    let mut rdr = Cursor::new(buffer);
    rdr.read_u32::<BigEndian>().unwrap()
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

    let player_name = read_string_of_size(br, 12);
    println!("player name: {:?}", player_name);
    // let header = read_string(br, header_size);
    // println!("header size: {:?}", header);

    Ok(())
}
