use crate::Compressable;
use image::Rgb;

#[derive(Debug, Clone)]
pub struct Palette([Rgb<u8>; 16]);
pub struct PaletteCollection([Palette; 16]);

#[derive(Debug, Clone, Copy)]
pub struct ColorIndex(u8);

impl ColorIndex {
    pub const fn new(index: u8) -> Self {
        assert!(index < 16, "ColorIndex must be less than 16");

        ColorIndex(index)
    }

    pub const fn as_u8(&self) -> u8 {
        self.0
    }

    pub const fn is_transparent(&self) -> bool {
        self.0 == 0
    }
}

impl PaletteCollection {
    pub const fn get(&self, index: usize) -> &Palette {
        &self.0[index]
    }
}

impl Compressable for PaletteCollection {
    /// Convert a slice of bytes into a collection of SNES palettes.
    fn try_from_slice(data: &[u8]) -> Option<Self> {
        if data.len() != 512 {
            return None;
        }

        let palettes = data
            .chunks_exact(32)
            .map(|palette_data| Palette::from_slice(palette_data))
            .collect::<Vec<_>>();

        Some(PaletteCollection(palettes.try_into().unwrap()))
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

    pub const fn get(&self, index: ColorIndex) -> Rgb<u8> {
        self.0[index.0 as usize]
    }
}
