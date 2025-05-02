use crate::palette::ColorIndex;

use super::Palette;
use image::{Rgba, RgbaImage};

//pdp uses 4bpp tiles
const PLANE_CNT: usize = 4;

pub struct Tile([ColorIndex; 64]);

pub struct TileSet(Vec<Tile>);

impl TileSet {
    pub fn from_slice(data: &[u8]) -> Self {
        assert!(
            data.len() % 32 == 0,
            "Tile data must be a multiple of 32 bytes"
        );

        let tiles = data
            .chunks_exact(32)
            .map(|tile_data| Tile::from_slice(tile_data))
            .collect::<Vec<_>>();

        TileSet(tiles)
    }

    pub fn remove(&mut self, index: usize) -> Tile {
        assert!(index < self.0.len(), "Index out of bounds");
        self.0.remove(index)
    }

    pub fn tiles(&self) -> &[Tile] {
        &self.0
    }
}

impl Tile {
    pub fn data(&self) -> &[ColorIndex] {
        &self.0
    }

    pub fn from_slice(data: &[u8]) -> Self {
        assert!(data.len() == 32, "Tile data must be 32 bytes long");

        let mut tile = [ColorIndex::new(0); 64];

        for row in 0..8 {
            let mut row_planes = Vec::new();
            for plane in 0..PLANE_CNT / 2 {
                let offset = 16 * plane;
                row_planes.push(data[offset + row * 2]);
                row_planes.push(data[offset + row * 2 + 1]);
            }

            for col in 0..8 {
                let shift = 7 - col;
                let mut color = 0;

                for plane in 0..PLANE_CNT {
                    let bit = (row_planes[plane] >> shift) & 1;
                    color |= (bit << plane) as u8;
                }

                tile[row * 8 + col] = ColorIndex::new(color);
            }
        }

        Tile(tile)
    }

    pub fn with_palette(&self, palette: &Palette) -> RgbaImage {
        RgbaImage::from_fn(8, 8, |x, y| {
            let pixel_index = ((y * 8) + x) as usize;
            let color_index = self.0[pixel_index];

            if color_index.is_transparent() {
                Rgba([0, 0, 0, 0])
            } else {
                let color = palette.get(color_index);
                Rgba([color[0], color[1], color[2], 255])
            }
        })
    }
}
