use crate::{PaletteCollection, TileMap, TileSet};
use image::{GenericImage, RgbaImage};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Sprite {
    pub size: (u32, u32),
    pub tiles: Arc<TileSet>,
    pub tile_map: Arc<TileMap>,
    pub palettes: Arc<PaletteCollection>,
}

impl Sprite {
    pub fn new(
        size: (u32, u32),
        tiles: Arc<TileSet>,
        tile_map: Arc<TileMap>,
        palettes: Arc<PaletteCollection>,
    ) -> Self {
        assert_eq!(
            size.0 * size.1,
            tile_map.len() as u32,
            "Sprite size does not match tile map size"
        );

        Sprite {
            size,
            tiles,
            tile_map,
            palettes,
        }
    }

    pub fn to_image(&self) -> RgbaImage {
        let mut image = RgbaImage::new(self.size.0 * 8, self.size.1 * 8);

        for i in 0..self.tile_map.len() {
            let entry = &self.tile_map[i];

            let x = (i as u32 % self.size.0) * 8;
            let y = (i as u32 / self.size.0) * 8;

            let tile = &self.tiles[entry.tile_index()];

            let palette = &self.palettes[entry.palette_index()];
            let tile_image = tile.with_palette(palette, entry.tile_settings());
            image.copy_from(&tile_image, x, y).unwrap();
        }

        image
    }
}
