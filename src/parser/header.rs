use super::utils::{read_u16, read_u32, read_w_string};

#[derive(Debug)]
pub enum Sex {
    Male,
    Female,
    Undefined,
}

impl From<u16> for Sex {
    fn from(sex_bit: u16) -> Sex {
        match sex_bit {
            0 => Sex::Male,
            1 => Sex::Female,
            _ => Sex::Undefined,
        }
    }
}

#[derive(Debug)]
pub struct Header {
    pub version: u32,
    pub save_number: u32,
    pub player_name: String,
    pub player_level: u32,
    pub player_location: String,
    pub game_date: String,
    pub player_race_editor_id: String,
    pub player_sex: Sex,
}

pub fn read_header(buf: &[u8], start: usize) -> (Header, usize) {
    let (version, cursor) = read_u32(buf, start);
    let (save_number, cursor) = read_u32(buf, cursor);
    let (player_name, cursor) = read_w_string(buf, cursor);
    let (player_level, cursor) = read_u32(buf, cursor);
    let (player_location, cursor) = read_w_string(buf, cursor);
    let (game_date, cursor) = read_w_string(buf, cursor);
    let (player_race_editor_id, cursor) = read_w_string(buf, cursor);
    let (player_sex_bit, cursor) = read_u16(buf, cursor);
    let header = Header {
        version,
        save_number,
        player_name,
        player_level,
        player_location,
        game_date,
        player_race_editor_id,
        player_sex: Sex::from(player_sex_bit),
    };
    (header, cursor)
}
