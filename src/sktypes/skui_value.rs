use super::types::SkTypeReadable;

fn plugin_type_from_name(name: String) -> PluginType {
    match name.as_str() {
        "Skyrim.esm" | "Update.esm" | "Dragonborn.esm" | "HearthFires.esm" | "Dawnguard.esm" => {
            tracing::info!("{:?} is Native", name);
            PluginType::Native
        }
        _ => {
            if name.starts_with("cc") {
                tracing::info!("{:?} is Creation Club", name);
                PluginType::CreationClub
            } else {
                tracing::info!("{:?} is Mod", name);
                PluginType::Mod
            }
        }
    }
}

#[derive(PartialEq, Debug)]
#[allow(dead_code)]
pub enum PluginType {
    Native,
    CreationClub,
    Mod,
    NotAPlugin,
}

pub enum UIValueType {
    Plugin,
    Header,
    Value,
}

pub struct SkUIValue {
    name: String,
    value: String,
    pub value_type: UIValueType,
    pub plugin_type: PluginType,
}
impl SkUIValue {
    pub fn new(name: &str, value: String, value_type: UIValueType) -> SkUIValue {
        let plugin_type = plugin_type_from_name(value.clone());
        SkUIValue {
            name: name.to_string(),
            value,
            value_type,
            plugin_type,
        }
    }
}
impl SkTypeReadable for SkUIValue {
    fn get_value_string(&self) -> String {
        self.value.to_string()
    }

    fn get_name(&self) -> String {
        self.name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_known_native_file_should_return_native_type() {
        let input = "Skyrim.esm".to_string();
        let t = plugin_type_from_name(input);
        assert_eq!(t, PluginType::Native);
    }

    #[test]
    fn given_known_creation_club_file_should_return_creation_club_type() {
        let input = "cceejsse001-hstead.esm".to_string();
        let t = plugin_type_from_name(input);
        assert_eq!(t, PluginType::CreationClub);
    }

    #[test]
    fn given_known_mod_file_should_return_native_type() {
        let input = "TrueStormsSE.esp".to_string();
        let t = plugin_type_from_name(input);
        assert_eq!(t, PluginType::Mod);
    }
}
