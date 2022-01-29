pub fn read_charray(buf: &[u8], start: usize, end: usize) -> (String, usize) {
    let chunk = &buf[start..end];
    let s = match std::str::from_utf8(chunk) {
        Ok(s) => s.to_string(),
        Err(_) => {
            tracing::error!("Failed to read string from {start} to {end}");
            "".to_string()
        }
    };
    (s, end)
}

pub fn read_f32(buf: &[u8], start: usize) -> (f32, usize) {
    let chunk = &buf[start..start + 4];
    let n = match <[u8; 4]>::try_from(chunk) {
        Ok(bytes) => {
            let n = f32::from_le_bytes(bytes);
            n
        }
        Err(_) => {
            tracing::error!("Could not parse u32 from chunk at {start}");
            0.0
        }
    };
    (n, start + 4)
}

pub fn read_u32(buf: &[u8], start: usize) -> (u32, usize) {
    let chunk = &buf[start..start + 4];
    let n = match <[u8; 4]>::try_from(chunk) {
        Ok(bytes) => {
            let n = u32::from_le_bytes(bytes);
            n
        }
        Err(_) => {
            tracing::error!("Could not parse u32 from chunk at {start}");
            0
        }
    };
    (n, start + 4)
}

pub fn read_u16(buf: &[u8], start: usize) -> (u16, usize) {
    let chunk = &buf[start..start + 2];
    let n = match <[u8; 2]>::try_from(chunk) {
        Ok(bytes) => {
            let n = u16::from_le_bytes(bytes);
            n
        }
        Err(_) => {
            tracing::error!("Could not parse u16 from chunk at {start}");
            0
        }
    };
    (n, start + 2)
}

pub fn read_u8(buf: &[u8], start: usize) -> (u8, usize) {
    let end = start + 1;
    let chunk = &buf[start..end];
    let n = match <[u8; 1]>::try_from(chunk) {
        Ok(bytes) => {
            let n = u8::from_le_bytes(bytes);
            n
        }
        Err(_) => {
            tracing::error!("Could not parse u8 from chunk at {start}");
            0
        }
    };
    (n, end)
}

pub fn read_w_string(buf: &[u8], start: usize) -> (String, usize) {
    let (length, start) = read_u16(buf, start);
    let end = start + length as usize;
    let chunk = &buf[start..end];
    let str = match std::str::from_utf8(chunk) {
        Ok(s) => s.to_string(),
        Err(e) => {
            println!("Error parsing string: {:?}", e);
            //  println!("Buffer from {:?} to {:?} value was {:?}", start, end);
            "".to_string()
        }
    };
    (str, end)
}

pub fn read_bytes(buf: &[u8], start: usize, bytes: usize) -> (&[u8], usize) {
    let end = start + bytes;
    let b = &buf[start..end];
    (b, end)
}
