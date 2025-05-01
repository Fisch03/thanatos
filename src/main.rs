use thanatos::Rom;

fn main() {
    colog::init();

    let rom = Rom::open("panepon.sfc").expect("Failed to open ROM file");

    dbg!(rom.version());

    /*
    let rom = fs::read("panepon.sfc").expect("Failed to read ROM file");

    const TILE_OFFSETS: &[(&str, usize)] = &[
        ("blocks+score", 0x60000),
        ("combo+coundown+pausemenu", 0x61c41),
        ("bg+player", 0x8a915),
    ];
    const PALETTE_OFFSETS: &[usize] = &[0x6295d];

    let _ = fs::remove_dir_all("tile_export");
    fs::create_dir("tile_export").expect("Failed to create directory");

    for (entry, offset) in TILE_OFFSETS {
        log::info!("exporting tiles for '{}'...", entry);

        log::info!("decompressing data...");
        let result = panepon_tile::decompress(&rom, *offset);

        log::info!("writing tiles...");
        write_tiles(entry, &result);

        log::info!("done!\n");
    }
    */
}

/*
fn write_tiles(dest: &str, data: &[u8]) {
    use image::{Rgb, Rgba};

    let dest = format!("tile_export/{}", dest);
    let dest = dest.as_str();

    let _ = fs::remove_dir_all(&dest);
    fs::create_dir(dest).expect("Failed to create directory");

    //pdp uses 4bpp tiles
    const PLANE_CNT: usize = 4;

    const PALETTE_DATA: &[u8] = include_bytes!("palette.bin");

    data.par_chunks(32)
        .progress()
        .enumerate()
        .for_each(|(i, tile)| {
            let mut data = [0; 64];

            for row in 0..8 {
                let mut row_planes = Vec::new();
                for plane in 0..PLANE_CNT / 2 {
                    let offset = 16 * plane;
                    row_planes.push(tile[offset + row * 2]);
                    row_planes.push(tile[offset + row * 2 + 1]);
                }

                for col in 0..8 {
                    let shift = 7 - col;
                    let mut color = 0;

                    for plane in 0..PLANE_CNT {
                        let bit = (row_planes[plane] >> shift) & 1;
                        color |= (bit << plane) as u8;
                    }

                    data[row * 8 + col] = color;
                }
            }

            let img = image::ImageBuffer::from_fn(8, 8, |x, y| {
                let pixel_index = ((y * 8) + x) as usize;
                let color_index = data[pixel_index] as usize;

                if color_index == 0 {
                    // Transparent color
                    Rgba([0, 0, 0, 0])
                } else {
                    let palette = palettes[10].0;
                    let color = palette[color_index];
                    Rgba([color[0], color[1], color[2], 255])
                }
            });

            const SCALE: u32 = 10;
            let img = image::imageops::resize(
                &img,
                8 * SCALE,
                8 * SCALE,
                image::imageops::FilterType::Nearest,
            );

            img.save(format!("{}/tile_{:04}.png", dest, i))
                .expect("Failed to save tile image");
        });
}
*/
