#[allow(dead_code)]
use std::{io::{Read, Cursor, Seek}, mem::size_of};

use byteorder::{LittleEndian, ReadBytesExt};

pub mod types;
pub mod skui_value;
