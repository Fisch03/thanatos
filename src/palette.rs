use crate::{Compressable, DecompressError};
use image::Rgb;
use serde::Deserialize;

pub const BW_PALETTE: Palette = Palette([
    Rgb([0, 0, 0]),
    Rgb([255, 255, 255]),
    Rgb([170, 170, 170]),
    Rgb([85, 85, 85]),
    Rgb([0, 0, 0]),
    Rgb([255, 255, 255]),
    Rgb([170, 170, 170]),
    Rgb([85, 85, 85]),
    Rgb([0, 0, 0]),
    Rgb([255, 255, 255]),
    Rgb([170, 170, 170]),
    Rgb([85, 85, 85]),
    Rgb([0, 0, 0]),
    Rgb([255, 255, 255]),
    Rgb([170, 170, 170]),
    Rgb([85, 85, 85]),
]);

#[derive(Debug, Clone, Copy)]
pub struct Palette([Rgb<u8>; 16]);
#[derive(Debug, Clone)]
pub struct PaletteCollection([Palette; 16]);

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ColorIndex(usize);

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PaletteIndex(usize);

impl ColorIndex {
    pub const fn new(index: usize) -> Self {
        assert!(index < 16, "ColorIndex must be less than 16");

        ColorIndex(index)
    }

    pub const fn is_transparent(&self) -> bool {
        self.0 == 0
    }
}

impl PaletteIndex {
    pub const fn new(index: usize) -> Self {
        assert!(index < 16, "PaletteIndex must be less than 16");

        PaletteIndex(index)
    }
}

impl std::ops::Index<PaletteIndex> for PaletteCollection {
    type Output = Palette;

    fn index(&self, index: PaletteIndex) -> &Self::Output {
        &self.0[index.0]
    }
}

impl PaletteCollection {
    pub fn add_palette_data(&mut self, offset: usize, data: &[u8]) {
        if data.len() % 32 != 0 {
            panic!("Palette data must be a multiple of 32 bytes");
        }

        let palette_count = data.len() / 32;

        if offset + palette_count > self.0.len() {
            panic!("PaletteCollection overflow");
        }

        data.chunks_exact(32)
            .map(|palette_data| Palette::from_slice(palette_data))
            .enumerate()
            .for_each(|(i, palette)| {
                self.0[offset + i] = palette;
            });
    }
}

impl Compressable for PaletteCollection {
    /// Convert a slice of bytes into a collection of SNES palettes.
    fn try_from_slice(data: &[u8]) -> Result<Self, DecompressError> {
        if data.len() != 512 {
            return Err(DecompressError::InvalidLayout(format!(
                "Palette data must be 512 bytes long, was {}",
                data.len()
            )));
        }

        let mut collection = PaletteCollection([Palette([Rgb([0, 0, 0]); 16]); 16]);
        collection.add_palette_data(0, data);
        Ok(collection)
    }
}

impl Palette {
    /// Convert a slice of bytes into a SNES palette.
    pub fn from_slice(data: &[u8]) -> Self {
        assert!(data.len() == 32, "Palette data must be 32 bytes long");

        let mut palette = [Rgb([0, 0, 0]); 16];
        for (i, color) in data.chunks_exact(2).enumerate() {
            let val_rgb15 = u16::from_le_bytes([color[0], color[1]]);
            let r = ((val_rgb15 >> 0) & 0x1F) << 3;
            let g = ((val_rgb15 >> 5) & 0x1F) << 3;
            let b = ((val_rgb15 >> 10) & 0x1F) << 3;

            palette[i] = Rgb([r as u8, g as u8, b as u8]);
        }
        Palette(palette)
    }
}

impl std::ops::Index<ColorIndex> for Palette {
    type Output = Rgb<u8>;

    fn index(&self, index: ColorIndex) -> &Self::Output {
        &self.0[index.0]
    }
}
