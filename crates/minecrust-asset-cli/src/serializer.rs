use anyhow::Result;
use minecrust_shared::AssetPack;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn save_asset_pack<P: AsRef<Path>>(pack: &AssetPack, path: P) -> Result<()> {
    let bytes = bincode::serialize(pack)?;
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    Ok(())
}
