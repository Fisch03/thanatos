use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use std::{fs, path::PathBuf};
use thanatos::{Compressable, MappedRom, Rom, RomMap};

#[derive(Parser, Debug)]
struct Arguments {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Export sprite and palettes from a given ROM
    Export {
        rom: PathBuf,

        /// Supply a custom ROM map that provides the offsets of the palettes and sprites
        #[arg(short = 'm', long)]
        rom_map: Option<PathBuf>,

        #[command(flatten)]
        args: ExportArgs,
    },

    /// Scan a ROM for potential tiles. This will return a lot of garbage but can still be useful
    /// for finding tiles that are not in the inbuilt ROM map
    Scan {
        rom: PathBuf,

        #[command(flatten)]
        args: ScanArgs,
    },
}

pub struct LoadedRom<'rom> {
    pub rom: Rom<'rom>,
    pub mapped: Option<MappedRom>,
}

impl Commands {
    fn get_rom(&self) -> anyhow::Result<LoadedRom<'_>> {
        let rom_path = match self {
            Commands::Export { rom, .. } => rom,
            Commands::Scan { rom, .. } => rom,
        };

        let rom = Rom::open(rom_path)?;

        let mapped = match self {
            Commands::Export { rom_map, .. } => {
                if let Some(rom_map) = rom_map {
                    let map = RomMap::parse(&fs::read_to_string(rom_map)?)?;

                    if map.is_compatible_with(&rom) {
                        Some(MappedRom::new(&rom, &map)?)
                    } else {
                        log::warn!(
                            "ROM map is not compatible with the supplied ROM. Continuing anyway."
                        );

                        Some(MappedRom::new_forced(&rom, &map)?)
                    }
                } else {
                    let map = RomMap::find_inbuilt_for(&rom).with_context(|| {
                        "Failed to find compatible ROM map for the supplied ROM"
                    })?;

                    Some(MappedRom::new(&rom, &map)?)
                }
            }
            Commands::Scan { .. } => {
                RomMap::find_inbuilt_for(&rom).and_then(|map| MappedRom::new(&rom, &map).ok())
            }
        };

        Ok(LoadedRom { rom, mapped })
    }
}

fn main() -> anyhow::Result<()> {
    let args = Arguments::parse();
    colog::init();

    let rom = args.command.get_rom()?;
    if let Some(mapped) = &rom.mapped {
        log::info!(
            "Loaded ROM: '{}' with CRC: {:#08x}",
            mapped.metadata.name,
            mapped.metadata.crc
        );
    }

    match &args.command {
        Commands::Export { args, .. } => {
            export(rom, args.clone())?;
        }
        Commands::Scan { args, .. } => scan(rom, args.clone())?,
    }

    /*
    let rom = Rom::open("panepon.sfc").expect("Failed to open ROM file");
    let rom =
        MappedRom::new(&rom, RomMap::find_inbuilt_for(&rom).unwrap()).expect("Failed to map ROM");

    let _ = fs::remove_dir_all("export");
    fs::create_dir("export").expect("Failed to create directory");

    rom.sprites.iter().for_each(|sprite| {
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
        log::info!("Exported sprite: {}", path);
    });

    log::info!("Done!");
    */

    Ok(())
}

#[derive(Args, Debug, Clone)]
struct ExportArgs {
    /// The output directory to export the sprites and palettes to
    #[arg(short, long, default_value = "export")]
    out_dir: PathBuf,

    /// Overwrite the output directory if it already exists
    #[arg(long)]
    force: bool,

    #[arg(short, long, default_value = "tilemap")]
    format: ExportFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum ExportFormat {
    /// Export as Tilemap files for use with tilemap editors
    Tilemap,

    /// Export as PNG images
    Png,
}

fn export(rom: LoadedRom, args: ExportArgs) -> anyhow::Result<()> {
    log::info!("Exporting sprites and palettes...");

    let rom = rom.mapped.with_context(|| {
        "Failed to load ROM map. Please provide a valid ROM map or use the scan command."
    })?;

    if args.force && args.out_dir.exists() {
        fs::remove_dir_all(&args.out_dir)
            .with_context(|| "Failed to clean up old export directory")?;
    }
    fs::create_dir(&args.out_dir).with_context(|| "Failed to create output directory")?;

    rom.sprites.iter().for_each(|sprite| {
        let extension = match args.format {
            ExportFormat::Tilemap => "tilemap",
            ExportFormat::Png => "png",
        };

        let path = if let Some(category) = &sprite.category {
            fs::create_dir_all(format!("export/{}", category)).expect("Failed to create directory");
            format!("export/{}/{}.{}", category, sprite.name, extension)
        } else {
            format!("export/{}.{}", sprite.name, extension)
        };

        match args.format {
            ExportFormat::Tilemap => {
                todo!("Exporting tilemap is not implemented yet");
            }
            ExportFormat::Png => {
                let img = sprite.sprite.to_image();

                let img = image::imageops::resize(
                    &img,
                    sprite.sprite.size.0 * 8 * 5,
                    sprite.sprite.size.1 * 8 * 5,
                    image::imageops::FilterType::Nearest,
                );

                img.save(&path).expect("Failed to save image");
            }
        }

        log::info!("Exported sprite: {}", path);
    });

    Ok(())
}

#[derive(Args, Debug, Clone)]
struct ScanArgs {
    /// The output directory to export found sprites to
    #[arg(short, long)]
    out_dir: Option<PathBuf>,

    /// Overwrite the output directory if it already exists
    #[arg(short, long)]
    force: bool,

    /// The minimum number of tiles to consider a valid tile set
    /// The default of 128 might miss a few tile sets but it doesnt return a lot of garbage either,
    /// using 64 is recommended if you want to find all tile sets
    #[arg(short, long, default_value = "128")]
    threshold: usize,
}

fn scan(rom: LoadedRom, args: ScanArgs) -> anyhow::Result<()> {
    use indicatif::ProgressBar;
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use thanatos::{Decompressor, PartialTileSet};

    log::info!("Scanning entire ROM for tiles...");

    let out_dir = args
        .out_dir
        .unwrap_or_else(|| format!("scan_0x{}", rom.rom.crc()).into());

    if args.force && out_dir.exists() {
        fs::remove_dir_all(&out_dir).with_context(|| "Failed to clean up old scan directory")?;
    }
    fs::create_dir(&out_dir).with_context(|| "Failed to create output directory")?;

    let rom = rom.rom;

    let progress = ProgressBar::new(rom.data().len() as u64);

    let found = AtomicUsize::new(0);
    let rom_len = rom.data().len();
    (0..rom_len).into_par_iter().for_each(|offset| {
        progress.inc(1);

        let (tiles, end_position) = {
            let result = Decompressor::new(&rom.data(), offset).decompress();

            let result = match result {
                Ok(result) => result,
                Err(_) => return,
            };

            if result.data.len() == 0 || result.data.len() % 32 != 0 {
                return;
            }

            let tiles = match PartialTileSet::try_from_slice(&result.data) {
                Ok(tiles) => tiles,
                Err(_) => return,
            };

            (tiles, offset + result.bytes_read)
        };

        let tile_amt = tiles.tiles().len();
        if tile_amt < args.threshold || tile_amt >= 1024 || !tile_amt.is_power_of_two() {
            return;
        }

        progress.println(format!(
            "Found potential tile set with {} tiles at {:#07x}-{:#07x}",
            tiles.tiles().len(),
            offset,
            end_position
        ));

        found.fetch_add(1, Ordering::SeqCst);

        let folder_name = &out_dir.join(format!("tiles_{:#07x}-{:#07x}", offset, end_position));
        fs::create_dir(folder_name).expect("Failed to create directory");
        tiles.tiles().par_iter().enumerate().for_each(|(i, tile)| {
            let img = tile.with_palette(&thanatos::BW_PALETTE, Default::default());

            const SCALE: u32 = 10;
            let img = image::imageops::resize(
                &img,
                8 * SCALE,
                8 * SCALE,
                image::imageops::FilterType::Nearest,
            );

            img.save(folder_name.join(format!("tile_{:04}.png", i)))
                .expect("Failed to save tile image");
        });
    });

    progress.finish_and_clear();
    log::info!(
        "Done! Found {} potential tile sets",
        found.load(Ordering::SeqCst)
    );

    Ok(())
}
