use eframe::egui::epaint::text::cursor;

use super::utils::{read_f32, read_u16, read_u32, read_w_string};

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
pub struct FileTime {
    pub dw_low: u32,
    pub dw_high: u32,
}

pub fn read_filetime(buf: &[u8], start: usize) -> (FileTime, usize) {
    let (dw_low, cursor) = read_u32(buf, start);
    let (dw_high, cursor) = read_u32(buf, cursor);
    let ft = FileTime { dw_low, dw_high };
    (ft, cursor)
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
    pub player_current_xp: f32,
    pub player_level_up_xp: f32,
    pub filetime: FileTime,
    pub screenshot_width: u32,
    pub screenshot_height: u32,

    /**
     * 	(SE only)
     *  0 = None
     *  1 = zLib (appears to not be used, see this from MO2. It seems however to be used for Change Forms)
     *  2 = LZ4 (Block format)
     *  If compression is present, everything after the compression lengths is compressed.
     */
    pub compression_type: u16,
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
    let (player_current_xp, cursor) = read_f32(buf, cursor);
    let (player_level_up_xp, cursor) = read_f32(buf, cursor);
    let (filetime, cursor) = read_filetime(buf, cursor);
    let (screenshot_width, cursor) = read_u32(buf, cursor);
    let (screenshot_height, cursor) = read_u32(buf, cursor);
    let (compression_type, cursor) = read_u16(buf, cursor);

    let header = Header {
        version,
        save_number,
        player_name,
        player_level,
        player_location,
        game_date,
        player_race_editor_id,
        player_sex: Sex::from(player_sex_bit),
        player_current_xp,
        player_level_up_xp,
        filetime,
        screenshot_width,
        screenshot_height,
        compression_type,
    };
    (header, cursor)
}
