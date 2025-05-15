use crate::Rom;
use serde::Deserialize;
use std::sync::{Arc, LazyLock};

static INBUILT_MAPS: LazyLock<Vec<Arc<RomMap>>> = LazyLock::new(|| {
    const INBUILT_MAP_SRC: &[&str] = &[include_str!("panepon_map.toml")];
    INBUILT_MAP_SRC
        .iter()
        .map(|&map| Arc::new(RomMap::parse(map).expect("Failed to parse inbuilt map")))
        .collect()
});

#[derive(Debug, Clone, Deserialize)]
pub struct RomMap {
    pub supported_roms: Vec<RomMetadata>,

    #[serde(rename = "palette")]
    pub palettes: Vec<PaletteDefinition>,
    #[serde(rename = "sprite")]
    pub sprites: Vec<SpriteDefinition>,
    #[serde(rename = "tileset")]
    pub tilesets: Vec<TileSetDefinition>,
}

impl RomMap {
    pub fn parse(map: &str) -> Result<RomMap, toml::de::Error> {
        toml::de::from_str(map)
    }

    pub fn is_compatible_with(&self, rom: &Rom) -> bool {
        self.get_compatible_metadata(rom).is_some()
    }

    pub fn get_compatible_metadata(&self, rom: &Rom) -> Option<RomMetadata> {
        self.supported_roms
            .iter()
            .find(|rom_type| rom_type.crc == rom.crc())
            .cloned()
    }

    pub fn find_inbuilt_for<'rom>(rom: &Rom) -> Option<Arc<RomMap>> {
        INBUILT_MAPS
            .iter()
            .find(|map| {
                map.supported_roms
                    .iter()
                    .any(|rom_type| rom_type.crc == rom.crc())
            })
            .cloned()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RomMetadata {
    pub name: String,
    pub crc: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteDefinition {
    pub name: String,
    pub layout: Vec<PaletteLayout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteLayout {
    pub region: usize,

    #[serde(default)]
    pub start: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpriteDefinition {
    pub name: String,
    pub category: Option<String>,

    pub size: (u32, u32),

    pub tileset: String,
    pub palette: String,

    pub layout_region: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileSetDefinition {
    pub name: String,
    pub layout: Vec<TileSetLayout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileSetLayout {
    pub region: usize,
    pub offset: usize,
}
