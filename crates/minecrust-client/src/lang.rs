use crate::asset_loader::AssetLoader;
use std::collections::HashMap;

pub struct LangManager {
    translations: HashMap<String, String>,
}

impl LangManager {
    pub fn new() -> Self {
        Self {
            translations: HashMap::new(),
        }
    }

    pub fn load(&mut self, lang_code: &str, loader: &AssetLoader) {
        self.translations.clear();
        let relative_path = format!("minecraft/lang/{}.json", lang_code);

        match loader.load_asset(&relative_path) {
            Ok(bytes) => {
                match serde_json::from_slice::<HashMap<String, String>>(&bytes) {
                    Ok(parsed) => {
                        self.translations = parsed;
                        println!(
                            "Successfully loaded language: {} ({} entries)",
                            lang_code,
                            self.translations.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to parse language JSON: {:?}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to load language asset for {}: {:?}", lang_code, e);
            }
        }
    }

    pub fn get(&self, key: &str) -> String {
        self.translations
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }
}
