use rayon::prelude::*;
use std::fs;
use thanatos::MappedRom;

fn main() {
    colog::init();

    let rom = MappedRom::open("panepon.sfc").expect("Failed to open ROM file");
    log::info!("Loaded ROM: '{}'", rom.metadata.name);

    let _ = fs::remove_dir_all("export");
    fs::create_dir("export").expect("Failed to create directory");

    rom.sprites.par_iter().for_each(|sprite| {
        let img = sprite.sprite.to_image();

        let img = image::imageops::resize(
            &img,
            sprite.sprite.size.0 * 8 * 10,
            sprite.sprite.size.1 * 8 * 10,
            image::imageops::FilterType::Nearest,
        );

        let path = if let Some(category) = &sprite.category {
            fs::create_dir_all(format!("export/{}", category)).expect("Failed to create directory");
            format!("export/{}/{}.png", category, sprite.name)
        } else {
            format!("export/{}.png", sprite.name)
        };

        img.save(&path).expect("Failed to save image");
        log::info!("Saved image: {}", path);
    });
}

/*
let _ = fs::remove_dir_all("test");
fs::create_dir("test").expect("Failed to create directory");

log::info!("Scanning for tiles...");
let rom_len = rom.data().len();
(0..rom_len).into_par_iter().progress().for_each(|offset| {
    let mut tiles = {
        let result = Decompressor::new(&rom.data(), offset).decompress();

        let data = match result {
            Ok(data) => data,
            Err(_) => {
                return;
            }
        };

        if data.len() == 0 || data.len() % 32 != 0 {
            return;
        }

        TileSet::from_slice(&data)
    };

    if tiles.tiles().len() < 64 {
        return;
    }

    let mut low_color_count = 0;
    for i in (0..tiles.tiles().len()).rev() {
        let mut used_colors = [false; 16];
        for j in 0..64 {
            let color = tiles.tiles()[i].data()[j];
            if !color.is_transparent() {
                used_colors[color.as_u8() as usize] = true;
            }
        }

        let color_count = used_colors.iter().filter(|&&c| c).count();
        if color_count < 2 {
            low_color_count += 1;
        }
    }

    if tiles.tiles().len() - low_color_count < 64 {
        return;
    }

    let folder_name = &format!("test/tiles_{:#08x}", offset);
    fs::create_dir(folder_name).expect("Failed to create directory");
    tiles.tiles().par_iter().enumerate().for_each(|(i, tile)| {
        let img = tile.with_palette(test_palette.get(10));

        const SCALE: u32 = 10;
        let img = image::imageops::resize(
            &img,
            8 * SCALE,
            8 * SCALE,
            image::imageops::FilterType::Nearest,
        );

        img.save(format!("{}/tile_{:04}.png", folder_name, i))
            .expect("Failed to save tile image");
    });
});
*/
