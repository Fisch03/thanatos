use crate::{palette::ColorIndex, Compressable, DecompressError, PaletteIndex};

use super::Palette;
use image::{Rgba, RgbaImage};

//pdp uses 4bpp tiles
const PLANE_CNT: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct Tile([ColorIndex; 64]);

#[derive(Debug, Clone)]
pub struct TileSet(Box<[Tile; 1024]>);

#[derive(Debug, Clone)]
pub struct PartialTileSet(Vec<Tile>);

#[derive(Debug, Clone)]
pub struct TileMap(Vec<TileMapEntry>);

#[derive(Debug, Clone)]
pub struct TileMapEntry(u16);

#[derive(Debug, Clone, Copy, Default)]
pub struct TileSettings {
    pub x_flip: bool,
    pub y_flip: bool,
    pub priority: u8,
}

impl TileSet {
    pub fn new() -> Self {
        TileSet(Box::new([Tile([ColorIndex::new(0); 64]); 1024]))
    }

    pub fn add_tile_data(&mut self, offset: usize, tiles: PartialTileSet) {
        if offset + tiles.0.len() > self.0.len() {
            panic!("TileSet overflow");
        }

        for i in offset..offset + tiles.0.len() {
            self.0[i] = tiles.0[i - offset];
        }
    }

    pub fn tiles(&self) -> &[Tile] {
        self.0.as_ref()
    }
}

impl PartialTileSet {
    pub fn tiles(&self) -> &[Tile] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Compressable for PartialTileSet {
    fn try_from_slice(data: &[u8]) -> Result<Self, DecompressError> {
        if data.len() % 32 != 0 {
            return Err(DecompressError::InvalidLayout(
                "Tile data must be a multiple of 32 bytes".to_string(),
            ));
        }

        let tiles = data
            .chunks_exact(32)
            .map(Tile::from_slice)
            .collect::<Vec<_>>();

        Ok(PartialTileSet(tiles))
    }
}

impl std::ops::Index<usize> for TileSet {
    type Output = Tile;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl TileMap {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl std::ops::Index<usize> for TileMap {
    type Output = TileMapEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl Compressable for TileMap {
    fn try_from_slice(data: &[u8]) -> Result<Self, DecompressError> {
        if data.len() % 2 != 0 {
            return Err(DecompressError::InvalidLayout(
                "TileMap data must be a multiple of 2 bytes".to_string(),
            ));
        }

        let tile_map = data
            .chunks_exact(2)
            .map(|chunk| {
                let data = u16::from_le_bytes([chunk[0], chunk[1]]);
                TileMapEntry(data)
            })
            .collect::<Vec<_>>();

        Ok(TileMap(tile_map))
    }
}

impl TileMapEntry {
    pub fn tile_index(&self) -> usize {
        (self.0 & 0x3FF) as usize
    }

    pub fn palette_index(&self) -> PaletteIndex {
        PaletteIndex::new(((self.0 >> 10) & 0x7) as usize)
    }

    pub fn tile_settings(&self) -> TileSettings {
        let priority = ((self.0 >> 13) & 0x3) as u8;
        let y_flip = ((self.0 >> 15) & 0x1) != 0;
        let x_flip = ((self.0 >> 14) & 0x1) != 0;

        TileSettings {
            x_flip,
            y_flip,
            priority,
        }
    }

    pub fn as_u16(&self) -> u16 {
        self.0
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

                tile[row * 8 + col] = ColorIndex::new(color as usize);
            }
        }

        Tile(tile)
    }

    pub fn with_palette(&self, palette: &Palette, settings: TileSettings) -> RgbaImage {
        RgbaImage::from_fn(8, 8, |x, y| {
            let x = if settings.x_flip { 7 - x } else { x };
            let y = if settings.y_flip { 7 - y } else { y };

            let pixel_index = ((y * 8) + x) as usize;
            let color_index = self.0[pixel_index];

            if color_index.is_transparent() {
                Rgba([0, 0, 0, 0])
            } else {
                let color = palette[color_index];
                Rgba([color[0], color[1], color[2], 255])
            }
        })
    }
}
