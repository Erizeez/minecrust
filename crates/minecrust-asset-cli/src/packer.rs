use image::{RgbaImage, GenericImage};
use std::collections::HashMap;

/// Result of packing a single texture
#[derive(Debug, PartialEq)]
pub struct PackedUV {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

pub struct AtlasPacker {
    canvas: RgbaImage,
    next_x: u32,
    next_y: u32,
    tile_size: u32,
    canvas_size: u32,
    texture_map: HashMap<String, PackedUV>,
}

impl AtlasPacker {
    pub fn new(canvas_size: u32, tile_size: u32) -> Self {
        Self {
            canvas: RgbaImage::new(canvas_size, canvas_size),
            next_x: 0,
            next_y: 0,
            tile_size,
            canvas_size,
            texture_map: HashMap::new(),
        }
    }

    /// Add a 16x16 image to the atlas and return its UV mapping
    pub fn add_texture(&mut self, name: &str, img: &RgbaImage) -> anyhow::Result<&PackedUV> {
        if self.texture_map.contains_key(name) {
            return Ok(self.texture_map.get(name).unwrap());
        }

        if self.next_y >= self.canvas_size {
            anyhow::bail!("Atlas is full!");
        }

        // Copy pixels
        self.canvas.copy_from(img, self.next_x, self.next_y)?;

        // Calculate UVs (0.0 to 1.0)
        let u0 = self.next_x as f32 / self.canvas_size as f32;
        let v0 = self.next_y as f32 / self.canvas_size as f32;
        let u1 = (self.next_x + self.tile_size) as f32 / self.canvas_size as f32;
        let v1 = (self.next_y + self.tile_size) as f32 / self.canvas_size as f32;

        let packed_uv = PackedUV { u0, v0, u1, v1 };
        self.texture_map.insert(name.to_string(), packed_uv);

        // Advance grid
        self.next_x += self.tile_size;
        if self.next_x >= self.canvas_size {
            self.next_x = 0;
            self.next_y += self.tile_size;
        }

        Ok(self.texture_map.get(name).unwrap())
    }

    pub fn get_uv(&self, name: &str) -> Option<&PackedUV> {
        self.texture_map.get(name)
    }

    pub fn get_canvas_bytes(&self) -> Vec<u8> {
        self.canvas.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_grid_packer() {
        let mut packer = AtlasPacker::new(32, 16);
        
        let dummy_img1 = RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
        let dummy_img2 = RgbaImage::from_pixel(16, 16, image::Rgba([0, 255, 0, 255]));
        let dummy_img3 = RgbaImage::from_pixel(16, 16, image::Rgba([0, 0, 255, 255]));

        let uv1 = packer.add_texture("red", &dummy_img1).unwrap();
        assert_eq!(uv1.u0, 0.0);
        assert_eq!(uv1.v0, 0.0);
        assert_eq!(uv1.u1, 0.5);
        assert_eq!(uv1.v1, 0.5);

        let uv2 = packer.add_texture("green", &dummy_img2).unwrap();
        assert_eq!(uv2.u0, 0.5);
        assert_eq!(uv2.v0, 0.0);

        let uv3 = packer.add_texture("blue", &dummy_img3).unwrap();
        assert_eq!(uv3.u0, 0.0);
        assert_eq!(uv3.v0, 0.5);

        // Test deduplication
        let uv1_dup = packer.add_texture("red", &dummy_img1).unwrap();
        assert_eq!(uv1_dup.u0, 0.0);
        
        // Assert canvas dimensions and raw bytes length (32x32 * 4 channels = 4096 bytes)
        assert_eq!(packer.get_canvas_bytes().len(), 4096);
    }
}
