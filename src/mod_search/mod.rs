mod vortex_scanner;
use std::collections::HashMap;
use eframe::egui::TextBuffer;
use urlencoding::encode;
use futures::{Future, Stream, TryFutureExt, StreamExt};

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

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn get_search_result_url() {
        let url = get_url_for_search("cheese grater");
        assert_eq!(url, "https://search.nexusmods.com/mods?terms=cheese%20grater&game_id=1704&blocked_tags=1069&blocked_authors=&include_adult=1".to_string());
    }

    #[tokio::test]
    async fn search_nexus_mods_test() {
        let str = search_nexus_mods("Frostfall").await;
        assert_eq!(str, "totally wrong");
    }
}