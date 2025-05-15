use std::{borrow::Cow, collections::HashMap, fs, path::Path, sync::Arc};
use thiserror::Error;

use crate::{
    tile::PartialTileSet, Compressable, DecompressError, Decompressor, PaletteCollection, Sprite,
    TileMap, TileSet,
};

mod map;
pub use map::RomMap;
use map::RomMetadata;

#[derive(Debug, Clone)]
pub struct Rom<'rom> {
    data: Cow<'rom, [u8]>,
    crc: u32,
}

#[derive(Debug, Clone)]
pub struct MappedRom {
    pub metadata: RomMetadata,
    pub palettes: Vec<MappedPaletteCollection>,
    pub sprites: Vec<MappedSprite>,
}

#[derive(Debug, Clone)]
pub struct MappedPaletteCollection {
    pub name: String,
    pub palettes: Arc<PaletteCollection>,
}

#[derive(Debug, Clone)]
pub struct MappedSprite {
    pub name: String,
    pub category: Option<String>,
    pub sprite: Sprite,
}

#[derive(Error, Debug)]
pub enum RomError {
    #[error("Failed to read ROM file")]
    Read(#[from] std::io::Error),
    #[error("Failed to decompress data")]
    Decompress(#[from] DecompressError),
    #[error("Incompatible ROM map")]
    IncompatibleMap,

    #[error("Invalid palette definition for '{0}', first region cannot have a start offset")]
    InvalidPaletteDefinition(String),

    #[error("Sprite definition for '{0}' references unknown palette '{1}'")]
    UnknownPalette(String, String),
    #[error("Sprite definition for '{0}' references unknown tileset '{1}'")]
    UnknownTileset(String, String),
}

impl<'rom> Rom<'rom> {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, RomError> {
        let rom = fs::read(path.as_ref())?;
        let crc = crc32fast::hash(&rom);
        let data = Cow::Owned(rom);
        Ok(Self { data, crc })
    }

    pub fn new(data: &'rom [u8]) -> Self {
        let crc = crc32fast::hash(data);
        let data = Cow::Borrowed(data);
        Self { data, crc }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn crc(&self) -> u32 {
        self.crc
    }
}

impl MappedRom {
    pub fn new(rom: &Rom, map: &RomMap) -> Result<Self, RomError> {
        if let Some(metadata) = map.get_compatible_metadata(rom) {
            let metadata = metadata.clone();
            Self::new_inner(&rom.data, map, metadata)
        } else {
            Err(RomError::IncompatibleMap)
        }
    }

    pub fn new_forced(rom: &Rom, map: &RomMap) -> Result<Self, RomError> {
        let metadata = RomMetadata {
            name: "Unknown".to_string(),
            crc: rom.crc(),
        };

        Self::new_inner(&rom.data, map, metadata)
    }

    fn new_inner(rom: &[u8], map: &RomMap, metadata: RomMetadata) -> Result<Self, RomError> {
        log::debug!("decompressing rom palette data...");
        let mut palettes = HashMap::new();
        for definition in map.palettes.iter() {
            let first = definition
                .layout
                .first()
                .map(|layout| layout.region)
                .ok_or_else(|| RomError::InvalidPaletteDefinition(definition.name.clone()))?;

            let mut palette_collection = PaletteCollection::from_compressed(rom, first)?;
            for layout in definition.layout.iter().skip(1) {
                let result = Decompressor::new(rom, layout.region).decompress()?;

                palette_collection.add_palette_data(layout.start, &result.data);
            }

            palettes.insert(
                definition.name.clone(),
                MappedPaletteCollection {
                    name: definition.name.clone(),
                    palettes: Arc::new(palette_collection),
                },
            );
        }

        log::debug!("decompressing rom tileset data...");
        let mut tilesets = HashMap::new();
        for definition in map.tilesets.iter() {
            let mut tileset = TileSet::new();

            for layout in definition.layout.iter() {
                let partial_tile_set = PartialTileSet::from_compressed(rom, layout.region)?;
                tileset.add_tile_data(layout.offset, partial_tile_set);
            }

            tilesets.insert(definition.name.clone(), Arc::new(tileset));
        }

        log::debug!("decompressing rom tilemap data...");
        let mut layout_regions = HashMap::new();
        for definition in map.sprites.iter() {
            if !layout_regions.contains_key(&definition.layout_region) {
                let layout = TileMap::from_compressed(rom, definition.layout_region)?;
                layout_regions.insert(definition.layout_region, Arc::new(layout));
            }
        }

        log::debug!("building sprites...");
        let mut sprites = Vec::new();
        for sprite_def in map.sprites.iter() {
            let palette = palettes.get(&sprite_def.palette).ok_or_else(|| {
                RomError::UnknownPalette(sprite_def.name.clone(), sprite_def.palette.clone())
            })?;

            let tileset = tilesets.get(&sprite_def.tileset).ok_or_else(|| {
                RomError::UnknownTileset(sprite_def.name.clone(), sprite_def.tileset.clone())
            })?;

            let layout = layout_regions.get(&sprite_def.layout_region).unwrap();

            let sprite = Sprite::new(
                (sprite_def.size.0, sprite_def.size.1),
                tileset.clone(),
                layout.clone(),
                palette.palettes.clone(),
            );

            let mapped_sprite = MappedSprite {
                name: sprite_def.name.clone(),
                category: sprite_def.category.clone(),
                sprite,
            };
            sprites.push(mapped_sprite);
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
