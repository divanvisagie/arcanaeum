mod vortex_scanner;
use std::{collections::{HashMap}, fs};
use eframe::egui::TextBuffer;
use serde::{Deserialize, Serialize};
use urlencoding::encode;

#[derive(Debug, Serialize, Deserialize)]
struct Plugin {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MasterList {
    plugins: Vec<Plugin>
}

fn get_url_for_search(term: &str) -> String {
    let encoded = encode(term.as_str()).to_string();
    format!("https://search.nexusmods.com/mods?terms={}&game_id=1704&blocked_tags=1069&blocked_authors=&include_adult=1", encoded)
}


#[allow(unused)]
pub async fn search_nexus_mods(term: &str) -> String {
    let url = get_url_for_search(term);
    let resp =  reqwest::get(url).await;
    let json = resp.unwrap().json::<HashMap<String, String>>().await.unwrap();

    "test".to_string()
}

#[allow(unused)]
pub fn get_masterlist_data() {
    let path =   "C:\\Users\\visag\\AppData\\Roaming\\Vortex\\skyrimse\\masterlist\\masterlist.yaml";
    let mut file_contents = fs::read_to_string(path).expect("Could not read file");
    let parsed: MasterList = serde_yaml::from_str(file_contents.as_str()).expect("It borked!");
    println!("{:?}",parsed.plugins);
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn get_search_result_url() {
        let url = get_url_for_search("cheese grater");
        assert_eq!(url, "https://search.nexusmods.com/mods?terms=cheese%20grater&game_id=1704&blocked_tags=1069&blocked_authors=&include_adult=1".to_string());
    }

    #[test]
    fn get_masterlist_data_test() {
        get_masterlist_data()
    }

    #[tokio::test]
    async fn search_nexus_mods_test() {
        let str = search_nexus_mods("Frostfall").await;
        assert_eq!(str, "totally wrong");
    }
}