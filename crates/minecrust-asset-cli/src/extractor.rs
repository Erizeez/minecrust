use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

pub struct AssetExtractor {
    archive: ZipArchive<File>,
}

impl AssetExtractor {
    pub fn new<P: AsRef<Path>>(jar_path: P) -> Result<Self> {
        let file = File::open(jar_path.as_ref())
            .with_context(|| format!("Failed to open JAR file at {:?}", jar_path.as_ref()))?;
        let archive = ZipArchive::new(file).context("Failed to parse JAR as ZIP archive")?;
        Ok(Self { archive })
    }

    pub fn read_file_as_string(&mut self, internal_path: &str) -> Result<String> {
        let mut file = self.archive.by_name(internal_path)
            .with_context(|| format!("File {} not found in JAR", internal_path))?;
        let mut content = String::new();
        std::io::Read::read_to_string(&mut file, &mut content)?;
        Ok(content)
    }

    pub fn read_file_as_bytes(&mut self, internal_path: &str) -> Result<Vec<u8>> {
        let mut file = self.archive.by_name(internal_path)
            .with_context(|| format!("File {} not found in JAR", internal_path))?;
        let mut content = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut content)?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jar_is_valid_zip() {
        // Assume the 1.21.1.jar is downloaded in assets/raw
        // Since tests run from the crate root, we need to path correctly.
        let jar_path = "../../assets/raw/1.21.1.jar";
        if Path::new(jar_path).exists() {
            let mut extractor = AssetExtractor::new(jar_path).expect("Should be a valid ZIP");
            let stone_json = extractor.read_file_as_string("assets/minecraft/blockstates/stone.json")
                .expect("Should contain stone blockstate");
            assert!(stone_json.contains("minecraft:block/stone"));
        } else {
            // Skip test nicely if not downloaded
            println!("Skipping test_jar_is_valid_zip because jar not found at {}", jar_path);
        }
    }
}
