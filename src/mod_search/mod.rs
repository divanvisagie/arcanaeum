mod vortex_scanner;

use eframe::egui::TextBuffer;
use urlencoding::encode;


#[allow(unused)]
pub fn get_url_for_search(term: &str) -> String {
    let encoded = encode(term.as_str());
    format!("https://search.nexusmods.com/mods?terms={:?}&game_id=1704&blocked_tags=1069&blocked_authors=&include_adult=1", encoded)
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn get_search_result_url() {
        let url = get_url_for_search("cheese grater");
        assert_eq!(url, "https://search.nexusmods.com/mods?terms=\"cheese%20grater\"&game_id=1704&blocked_tags=1069&blocked_authors=&include_adult=1".to_string());
    }
}