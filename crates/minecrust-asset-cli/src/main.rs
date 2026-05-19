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
            let blocks_to_parse = vec![
                "stone", "dirt", "grass_block", "sand", "bedrock", "oak_log", "oak_leaves",
                "coal_ore", "iron_ore", "gold_ore", "diamond_ore", "glass",
                "block", "cube_all", "cube"
            ];
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
            let mut texture_animations = HashMap::new();

            println!("Resolving and packing textures...");
            let target_blocks = vec![
                "stone", "dirt", "grass_block", "sand", "water", "bedrock", 
                "oak_log", "oak_leaves", "coal_ore", "iron_ore", "gold_ore", "diamond_ore", "glass"
            ];
            for target in target_blocks {
                let mut uv_faces = [[0.0; 4]; 6];
                
                if target == "water" {
                    // Water has no standard block model in jar, hardcode texture
                    let tex_name = "minecraft:block/water_still";
                    let tex_path = "block/water_still";
                    
                    let img_path = format!("assets/minecraft/textures/{}.png", tex_path);
                    if let Ok(bytes) = extractor.read_file_as_bytes(&img_path) {
                        if let Ok(img) = image::load_from_memory(&bytes) {
                            let mut rgba = img.to_rgba8();
                            
                            let mut frametime = 1;
                            let meta_path = format!("{}.mcmeta", img_path);
                            if let Ok(meta_json) = extractor.read_file_as_string(&meta_path) {
                                if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_json) {
                                    if let Some(anim) = meta.get("animation") {
                                        if let Some(ft) = anim.get("frametime") {
                                            frametime = ft.as_u64().unwrap_or(1) as u32;
                                        }
                                    }
                                }
                            }
                            
                            let is_animated = rgba.height() > rgba.width() && rgba.height() % rgba.width() == 0;
                            let frame_size = rgba.width();
                            let mut packed_rgba = rgba.clone();
                            
                            if is_animated {
                                packed_rgba = image::imageops::crop(&mut packed_rgba, 0, 0, frame_size, frame_size).to_image();
                            }
                            
                            if let Ok(packed_uv) = packer.add_texture(tex_name, &packed_rgba, None, None) {
                                let uv = [packed_uv.u0, packed_uv.v0, packed_uv.u1, packed_uv.v1];
                                uv_faces = [uv, uv, uv, uv, uv, uv]; // All faces use water_still
                                
                                if is_animated {
                                    let frame_count = rgba.height() / frame_size;
                                    let mut frames_rgba = Vec::new();
                                    for i in 0..frame_count {
                                        let frame = image::imageops::crop(&mut rgba, 0, i * frame_size, frame_size, frame_size).to_image();
                                        frames_rgba.push(frame.into_raw());
                                    }
                                    
                                    texture_animations.insert(tex_name.to_string(), minecrust_shared::TextureAnimation {
                                        frametime,
                                        frame_count,
                                        frame_size,
                                        atlas_x: (packed_uv.u0 * 1024.0) as u32,
                                        atlas_y: (packed_uv.v0 * 1024.0) as u32,
                                        frames_rgba,
                                    });
                                }
                            }
                        }
                    }
                } else {
                    let resolved = resolver.resolve_textures(&format!("minecraft:block/{}", target));
                    
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
                            let tex_path = tex_name.replace("minecraft:", "");
                            let img_path = format!("assets/minecraft/textures/{}.png", tex_path);
                            
                            if let Ok(bytes) = extractor.read_file_as_bytes(&img_path) {
                                if let Ok(img) = image::load_from_memory(&bytes) {
                                    let mut rgba = img.to_rgba8();
                                    
                                    let n_img_path = format!("assets/minecraft/textures/{}_n.png", tex_path);
                                    let s_img_path = format!("assets/minecraft/textures/{}_s.png", tex_path);
                                    let n_img = extractor.read_file_as_bytes(&n_img_path).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|i| i.to_rgba8());
                                    let s_img = extractor.read_file_as_bytes(&s_img_path).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|i| i.to_rgba8());
                                    
                                    let mut frametime = 1;
                                    let meta_path = format!("{}.mcmeta", img_path);
                                    if let Ok(meta_json) = extractor.read_file_as_string(&meta_path) {
                                        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_json) {
                                            if let Some(anim) = meta.get("animation") {
                                                if let Some(ft) = anim.get("frametime") {
                                                    frametime = ft.as_u64().unwrap_or(1) as u32;
                                                }
                                            }
                                        }
                                    }
                                    
                                    let is_animated = rgba.height() > rgba.width() && rgba.height() % rgba.width() == 0;
                                    let frame_size = rgba.width();
                                    let mut packed_rgba = rgba.clone();
                                    
                                    if is_animated {
                                        packed_rgba = image::imageops::crop(&mut packed_rgba, 0, 0, frame_size, frame_size).to_image();
                                    }
                                    
                                    if let Ok(packed_uv) = packer.add_texture(tex_name, &packed_rgba, n_img.as_ref(), s_img.as_ref()) {
                                        uv_faces[i] = [packed_uv.u0, packed_uv.v0, packed_uv.u1, packed_uv.v1];
                                        
                                        if is_animated && !texture_animations.contains_key(tex_name) {
                                            let frame_count = rgba.height() / frame_size;
                                            let mut frames_rgba = Vec::new();
                                            for i in 0..frame_count {
                                                let frame = image::imageops::crop(&mut rgba, 0, i * frame_size, frame_size, frame_size).to_image();
                                                frames_rgba.push(frame.into_raw());
                                            }
                                            
                                            texture_animations.insert(tex_name.to_string(), minecrust_shared::TextureAnimation {
                                                frametime,
                                                frame_count,
                                                frame_size,
                                                atlas_x: (packed_uv.u0 * 1024.0) as u32,
                                                atlas_y: (packed_uv.v0 * 1024.0) as u32,
                                                frames_rgba,
                                            });
                                        }
                                    }
                                }
                            }
                        }
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
                texture_animations,
            };

            serializer::save_asset_pack(&pack, out_file)?;
            println!("Successfully packed {} blocks and {} textures!", pack.block_dict.len(), pack.texture_dict.len());
        }
    }

    Ok(())
}
