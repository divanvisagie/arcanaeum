use super::utils::{read_u8, read_w_string};

#[derive(Debug)]
pub struct PluginInfo {
    pub plugin_count: u8,
    pub plugins: Vec<String>,
}

pub fn read_plugin_info(buf: &[u8], start: usize) -> (PluginInfo, usize) {
    let (plugin_count, start) = read_u8(buf, start);

    let mut plugins = Vec::new();
    let mut index_cursor = start;
    for _ in 0..plugin_count {
        let (str, cursor) = read_w_string(buf, index_cursor);
        plugins.push(str);
        index_cursor = cursor;
    }
    let plugin_info = PluginInfo {
        plugin_count,
        plugins,
    };
    (plugin_info, index_cursor)
}
