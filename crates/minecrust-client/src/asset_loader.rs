use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct CatalogEntry {
    pub hash: String,
    pub size: u64,
}

pub struct AssetLoader {
    catalog: HashMap<String, CatalogEntry>,
    assets_root: PathBuf,
}

impl AssetLoader {
    pub fn new() -> Self {
        let assets_root = Self::find_assets_root().unwrap_or_else(|err| {
            panic!(
                "\n========================================================================\n\
                [Minecrust 资源加载错误]: 无法定位 'assets' 资源目录！\n\
                诊断明细:\n\
                {}\n\n\
                建议修复方案:\n\
                1. 确保您的 'assets' 资源夹位于项目工作区根目录下。\n\
                2. 通过环境变量手动指定路径启动游戏:\n\
                   MINECRUST_ASSETS_DIR=/path/to/assets cargo run\n\
                ========================================================================\n",
                err
            );
        });

        let catalog_path = assets_root.join("processed/asset_catalog.json");
        let mut catalog = HashMap::new();

        if catalog_path.exists() {
            if let Ok(mut file) = File::open(&catalog_path) {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(parsed) = serde_json::from_str::<HashMap<String, CatalogEntry>>(&contents) {
                        catalog = parsed;
                        log::info!("AssetLoader catalog loaded with {} entries", catalog.len());
                    }
                }
            }
        } else {
            log::error!("Asset catalog file not found at: {:?}", catalog_path);
        }

        Self { catalog, assets_root }
    }

    pub fn assets_root(&self) -> &Path {
        &self.assets_root
    }

    fn find_assets_root() -> Result<PathBuf, String> {
        let mut checked_paths = Vec::new();

        // 1. 检查环境变量
        if let Ok(env_val) = std::env::var("MINECRUST_ASSETS_DIR") {
            let p = PathBuf::from(env_val);
            if p.exists() && p.is_dir() {
                return Ok(p);
            }
            checked_paths.push(format!("环境变量 MINECRUST_ASSETS_DIR 指向的路径不存在或非目录: {:?}", p));
        }

        // 2. 从可执行文件当前目录向上逐级回溯（最多5级）
        if let Ok(exe_path) = std::env::current_exe() {
            let mut current = exe_path.parent();
            let mut depth = 0;
            while let Some(dir) = current {
                if depth > 5 { break; }
                let candidate = dir.join("assets");
                if candidate.exists() && candidate.is_dir() {
                    return Ok(candidate);
                }
                checked_paths.push(format!("可执行文件回溯路径不存在: {:?}", candidate));
                current = dir.parent();
                depth += 1;
            }
        }

        // 3. 检查当前工作目录
        if let Ok(cwd) = std::env::current_dir() {
            let candidate = cwd.join("assets");
            if candidate.exists() && candidate.is_dir() {
                return Ok(candidate);
            }
            checked_paths.push(format!("当前工作目录不存在: {:?}", candidate));
        }

        Err(checked_paths.join("\n"))
    }

    pub fn load_asset(&self, relative_path: &str) -> Result<Vec<u8>, anyhow::Error> {
        let local_path = self.assets_root.join("raw").join(relative_path);

        // 1. Check if local file exists
        if local_path.exists() {
            let mut file = File::open(&local_path)?;
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
