use image::Rgb;

pub struct Palette([Rgb<u8>; 16]);

impl Palette {
    /// Convert a slice of bytes into a SNES palette.
    pub fn from_slice(data: &[u8]) -> Vec<Self> {
        data.chunks_exact(16 * 2)
            .map(|chunk| {
                let mut palette = [Rgb([0, 0, 0]); 16];
                for (i, color) in chunk.chunks_exact(2).enumerate() {
                    let val_rgb15 = u16::from_le_bytes([color[0], color[1]]);
                    let r = ((val_rgb15 >> 0) & 0x1F) << 3;
                    let g = ((val_rgb15 >> 5) & 0x1F) << 3;
                    let b = ((val_rgb15 >> 10) & 0x1F) << 3;

                    palette[i] = Rgb([r as u8, g as u8, b as u8]);
                }
                Palette(palette)
            })
            .collect::<Vec<_>>()
    }
}
