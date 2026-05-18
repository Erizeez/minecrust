use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct CatalogEntry {
    pub hash: String,
    pub size: u64,
}

pub struct AssetLoader {
    catalog: HashMap<String, CatalogEntry>,
}

impl AssetLoader {
    pub fn new() -> Self {
        let catalog_path = "assets/processed/asset_catalog.json";
        let mut catalog = HashMap::new();

        if Path::new(catalog_path).exists() {
            if let Ok(mut file) = File::open(catalog_path) {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(parsed) = serde_json::from_str::<HashMap<String, CatalogEntry>>(&contents) {
                        catalog = parsed;
                        log::info!("AssetLoader catalog loaded with {} entries", catalog.len());
                    }
                }
            }
        } else {
            log::error!("Asset catalog file not found at: {}", catalog_path);
        }

        Self { catalog }
    }

    pub fn load_asset(&self, relative_path: &str) -> Result<Vec<u8>, anyhow::Error> {
        let local_path_str = format!("assets/raw/{}", relative_path);
        let local_path = Path::new(&local_path_str);

        // 1. Check if local file exists
        if local_path.exists() {
            let mut file = File::open(local_path)?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            return Ok(bytes);
        }

        // 2. If it does not exist, look up in catalog
        let entry = self.catalog.get(relative_path).ok_or_else(|| {
            anyhow::anyhow!("Asset path not found in catalog: {}", relative_path)
        })?;

        // 3. Download the asset from Mojang CDN
        let hash = &entry.hash;
        let prefix = &hash[0..2];
        let url = format!("https://resources.download.minecraft.net/{}/{}", prefix, hash);

        log::info!(
            "Downloading missing asset on-demand: {} (Size: {} bytes) from {}",
            relative_path,
            entry.size,
            url
        );

        let response = ureq::get(&url).call()?;
        if response.status() != 200 {
            return Err(anyhow::anyhow!(
                "Failed to download asset, status code: {}",
                response.status()
            ));
        }

        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;

        // Validate size
        if bytes.len() as u64 != entry.size {
            return Err(anyhow::anyhow!(
                "Size mismatch for downloaded asset {}: expected {}, got {}",
                relative_path,
                entry.size,
                bytes.len()
            ));
        }

        // 4. Save locally
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(local_path)?;
        file.write_all(&bytes)?;

        log::info!("Successfully downloaded and cached asset: {}", relative_path);

        Ok(bytes)
    }
}
