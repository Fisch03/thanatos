use std::{collections::HashMap, fs, path::Path, sync::Arc};
use thiserror::Error;

use crate::{tile::Sprite, Compressable, DecompressError, Palette, PaletteCollection, TileSet};

mod map;
use map::{RomMap, RomMetadata};

#[derive(Debug, Clone)]
pub struct MappedRom {
    pub metadata: RomMetadata,
    pub palettes: Vec<Arc<MappedPalette>>,
    pub sprites: Vec<MappedSprite>,
}

#[derive(Debug, Clone)]
pub struct MappedPalette {
    pub name: String,
    pub palette: Palette,
}

#[derive(Debug, Clone)]
pub struct MappedSprite {
    pub name: String,
    pub category: Option<String>,
    pub sprite: Sprite,
}

#[derive(Error, Debug)]
pub enum RomLoadError {
    #[error("Failed to read ROM file")]
    Read(#[from] std::io::Error),
    #[error("Failed to decompress data")]
    Decompress(#[from] DecompressError),
    #[error("No compatible data map found for the ROM")]
    IncompatibleMap,
    #[error("Sprite definition for {0} produced sprite with invalid size")]
    InvalidSpriteSize(String),
    #[error("Sprite definition for {0} tried to access out of bounds tile")]
    OutOfBoundsTile(String),
    #[error("Sprite definition for {0} references unknown palette {1}")]
    UnknownPalette(String, String),
}

impl MappedRom {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, RomLoadError> {
        let rom = fs::read(path.as_ref())?;
        let map = RomMap::find_for_rom(&rom, &[]).ok_or(RomLoadError::IncompatibleMap)?;
        Self::new(&rom, map)
    }

    pub fn new(rom: &[u8], map: Arc<RomMap>) -> Result<Self, RomLoadError> {
        let crc = crc32fast::hash(rom);
        let metadata = map
            .supported_roms
            .iter()
            .find(|rom_type| rom_type.crc == crc)
            .ok_or(RomLoadError::IncompatibleMap)?
            .clone();

        let palette_regions =
            map.palettes
                .iter()
                .enumerate()
                .fold(HashMap::new(), |mut acc, (index, palette)| {
                    acc.entry(palette.region)
                        .or_insert_with(Vec::new)
                        .push(index);
                    acc
                });

        let mut palettes = HashMap::new();
        for (region, palette_indices) in palette_regions {
            let palette_collection = PaletteCollection::from_compressed(rom, region)?;

            for index in palette_indices {
                let palette_definition = &map.palettes[index];
                let palette = palette_collection.get(palette_definition.index);
                let mapped_palette = MappedPalette {
                    name: palette_definition.name.clone(),
                    palette: palette.clone(),
                };
                palettes.insert(palette_definition.name.clone(), Arc::new(mapped_palette));
            }
        }

        let sprite_regions =
            map.sprites
                .iter()
                .enumerate()
                .fold(HashMap::new(), |mut acc, (index, sprite)| {
                    acc.entry(sprite.region)
                        .or_insert_with(Vec::new)
                        .push(index);

                    acc
                });

        let mut sprites = Vec::new();
        for (region, sprite_indices) in sprite_regions {
            let tileset = TileSet::from_compressed(rom, region)?;

            for index in sprite_indices {
                let sprite_definition = &map.sprites[index];
                let mut tiles = Vec::new();

                let mut x = 0;
                let mut y = 0;
                for instruction in &sprite_definition.layout {
                    let mut index = instruction.start;
                    let count = instruction.count.unwrap_or(1);
                    let repeat = instruction.repeat.unwrap_or(1);
                    let gap = instruction.gap.unwrap_or(0);

                    for _ in 0..repeat {
                        for _ in 0..count {
                            let tile = tileset
                                .get(index)
                                .ok_or(RomLoadError::OutOfBoundsTile(
                                    sprite_definition.name.clone(),
                                ))?
                                .clone();

                            let palette_name = sprite_definition.palette.get((x, y));
                            let palette = palettes
                                .get(palette_name)
                                .ok_or(RomLoadError::UnknownPalette(
                                    sprite_definition.name.clone(),
                                    palette_name.to_string(),
                                ))?
                                .clone();

                            tiles.push((tile, palette.palette.clone()));

                            x += 1;
                            if x >= sprite_definition.size.0 {
                                x = 0;
                                y += 1;
                            }
                            index += 1;
                        }

                        index += gap;
                    }
                }

                if sprite_definition.size.0 * sprite_definition.size.1 != tiles.len() as u32 {
                    return Err(RomLoadError::InvalidSpriteSize(
                        sprite_definition.name.clone(),
                    ));
                }

                let sprite =
                    Sprite::new((sprite_definition.size.0, sprite_definition.size.1), tiles);

                let mapped_sprite = MappedSprite {
                    name: sprite_definition.name.clone(),
                    category: sprite_definition.category.clone(),
                    sprite,
                };
                sprites.push(mapped_sprite);
            }
        }

        let palettes = palettes
            .into_iter()
            .map(|(_, palette)| palette)
            .collect::<Vec<_>>();

        Ok(Self {
            metadata,
            palettes,
            sprites,
        })
    }
}
