use std::io::Read;

use crate::sktypes::{read_string_of_size};

use super::types::SkTypeReadable;


pub struct SkWstringArray {
    value: Vec<String>,
    name: String
}

impl SkWstringArray {
    pub fn from_file(file: &mut std::fs::File, name: &str, item_count: u32, total_size: u32) -> SkWstringArray {
        let end_of_sector = total_size - 2; // 2 is the size of u16 which was the first value read in the plugin info section already



        let str = read_string_of_size(file, 3);
        tracing::info!(">> {:?}", str);

        let mut buffer = Vec::with_capacity(end_of_sector.try_into().unwrap());
        file.take(end_of_sector.into())
            .read_to_end(&mut buffer)
            .map_err(|err| tracing::error!("{:?}", err))
            .ok();

        // TODO: Actually parse this section
        
        // let limit = 100;
        // let mut count = 0;
        // loop {
        //     if count == limit {
        //         break;
        //     }
        //     let mut buffer = Vec::with_capacity(0xff);
        //     file.take(1)
        //         .read_to_end(&mut buffer)
        //         .map_err(|err| tracing::error!("{:?}", err))
        //         .ok();
        //     count+=1;
        //     tracing::info!("buf: {:?}", buffer);
        // }

        let count = item_count.clone();
        SkWstringArray {
            name: name.to_string(),
            value : Vec::with_capacity(count.try_into().unwrap())
        }
    }
}
impl SkTypeReadable for SkWstringArray {
    fn get_value_string(&self) -> String {
        format!("{:?}", self.value)
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }

    fn get_type(&self) -> String {
        "wstring[]".to_string()
    }
}
