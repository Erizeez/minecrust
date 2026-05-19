mod extractor;
mod models;
mod packer;
mod resolver;
mod serializer;

use clap::{Parser, Subcommand};
use extractor::AssetExtractor;
use minecrust_shared::{AssetPack, BlockRenderData};
use packer::AtlasPacker;
use resolver::ModelResolver;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "minecrust-asset-cli")]
#[command(about = "Minecrust offline asset cooker", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract and pack assets into a binary .mca file
    Pack {
        /// Path to the 1.21.1.jar file
        #[arg(short, long)]
        jar_path: PathBuf,

        /// Output .mca file path
        #[arg(short, long, default_value = "assets/processed/assets.mca")]
        out_file: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Pack { jar_path, out_file } => {
            println!("Initializing extractor with JAR: {:?}", jar_path);
            let mut extractor = AssetExtractor::new(jar_path)?;

            // 1. Load basic models into resolver
            let mut resolver = ModelResolver::new();
            println!("Loading models...");
            
            // In a real scenario, we'd iterate over all files in the zip starting with `assets/minecraft/models/block/`
            // For MVP, we hardcode the dependencies for `stone` and `dirt`, and `grass_block`
            let blocks_to_parse = vec!["stone", "dirt", "grass_block", "block", "cube_all", "cube"];
            for block in &blocks_to_parse {
                let path = format!("assets/minecraft/models/block/{}.json", block);
                if let Ok(json) = extractor.read_file_as_string(&path) {
                    let model: models::Model = serde_json::from_str(&json)?;
                    resolver.insert_model(format!("minecraft:block/{}", block), model);
                } else if let Ok(json) = extractor.read_file_as_string(&format!("assets/minecraft/models/{}.json", block)) {
                     // For base models like `block/cube` which might just be in `models/block/cube.json` but named without prefix in some contexts
                    let model: models::Model = serde_json::from_str(&json)?;
                    resolver.insert_model(format!("minecraft:block/{}", block), model);
                }
            }

            // 2. Resolve textures and pack them into the atlas
            let mut packer = AtlasPacker::new(1024, 16);
            let mut block_dict = HashMap::new();
            let mut texture_dict = HashMap::new();

            println!("Resolving and packing textures...");
            let target_blocks = vec!["stone", "dirt", "grass_block"];
            for target in target_blocks {
                let resolved = resolver.resolve_textures(&format!("minecraft:block/{}", target));
                
                let mut uv_faces = [[0.0; 4]; 6];
                
                // MC faces: up, down, north, south, east, west
                let faces = vec!["north", "south", "east", "west", "up", "down"];
                for (i, face) in faces.iter().enumerate() {
                    let tex_name = resolved.get(*face)
                        .or_else(|| resolved.get("all"))
                        .or_else(|| match *face {
                            "up" => resolved.get("top"),
                            "down" => resolved.get("bottom"),
                            _ => resolved.get("side"),
                        });
                        
                    if let Some(tex_name) = tex_name {
                        // Strip 'minecraft:block/' prefix if present
                        let tex_path = tex_name.replace("minecraft:", "");
                        let img_path = format!("assets/minecraft/textures/{}.png", tex_path);
                        let n_img_path = format!("assets/minecraft/textures/{}_n.png", tex_path);
                        let s_img_path = format!("assets/minecraft/textures/{}_s.png", tex_path);
                        
                        let bytes = extractor.read_file_as_bytes(&img_path)?;
                        let img = image::load_from_memory(&bytes)?.to_rgba8();
                        
                        let n_img = extractor.read_file_as_bytes(&n_img_path)
                            .ok()
                            .and_then(|b| image::load_from_memory(&b).ok())
                            .map(|i| i.to_rgba8());
                            
                        let s_img = extractor.read_file_as_bytes(&s_img_path)
                            .ok()
                            .and_then(|b| image::load_from_memory(&b).ok())
                            .map(|i| i.to_rgba8());
                        
                        let packed_uv = packer.add_texture(tex_name, &img, n_img.as_ref(), s_img.as_ref())?;
                        uv_faces[i] = [packed_uv.u0, packed_uv.v0, packed_uv.u1, packed_uv.v1];
                    }
                }
                
                block_dict.insert(format!("minecraft:{}", target), BlockRenderData { uv_faces });
            }

            // 2b. Extract special entity textures (Steve and Alex)
            let special_textures = vec![
                "assets/minecraft/textures/entity/player/wide/steve.png",
                "assets/minecraft/textures/entity/player/slim/alex.png",
            ];
            
            for tex_path in special_textures {
                if let Ok(bytes) = extractor.read_file_as_bytes(tex_path) {
                    if let Ok(img) = image::load_from_memory(&bytes) {
                        let rgba = img.to_rgba8();
                        // Use the file name as the identifier, e.g., "steve" or "alex"
                        let name = tex_path.split('/').last().unwrap().replace(".png", "");
                        if let Ok(packed_uv) = packer.add_texture(&name, &rgba, None, None) {
                            texture_dict.insert(name, [packed_uv.u0, packed_uv.v0, packed_uv.u1, packed_uv.v1]);
                        }
                    }
                }
            }

            // 3. Serialize AssetPack
            println!("Serializing to {:?}...", out_file);
            if let Some(parent) = out_file.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let pack = AssetPack {
                version: "1.21.1-mvp".to_string(),
                atlas_png: packer.get_albedo_bytes(),
                atlas_normal_png: packer.get_normal_bytes(),
                atlas_specular_png: packer.get_specular_bytes(),
                block_dict,
                texture_dict,
            };

            serializer::save_asset_pack(&pack, out_file)?;
            println!("Successfully packed {} blocks and {} textures!", pack.block_dict.len(), pack.texture_dict.len());
        }
    }

    Ok(())
}
