use serde::{
    de::{self, Visitor},
    Deserialize,
};
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
}

impl RomMap {
    pub fn parse(map: &str) -> Result<RomMap, toml::de::Error> {
        toml::de::from_str(map)
    }

    pub fn find_for_rom<'rom>(
        rom: &'rom [u8],
        additional_maps: &[Arc<RomMap>],
    ) -> Option<Arc<RomMap>> {
        let crc = crc32fast::hash(rom);

        INBUILT_MAPS
            .iter()
            .chain(additional_maps.iter())
            .find(|map| {
                map.supported_roms
                    .iter()
                    .any(|rom_type| rom_type.crc == crc)
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
    pub region: usize,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct PaletteLayout {
    pub default: String,
    pub regions: Vec<PaletteRegion>,
}

impl PaletteLayout {
    pub fn get(&self, pos: (u32, u32)) -> &str {
        for region in &self.regions {
            if pos.0 >= region.start.0
                && pos.0 < region.start.0 + region.size.0
                && pos.1 >= region.start.1
                && pos.1 < region.start.1 + region.size.1
            {
                return &region.palette;
            }
        }

        &self.default
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteRegion {
    pub palette: String,
    pub start: (u32, u32),
    pub size: (u32, u32),
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpriteDefinition {
    pub name: String,

    pub category: Option<String>,
    pub size: (u32, u32),

    pub region: usize,

    pub layout: Vec<SpriteLayoutInstruction>,

    pub palette: PaletteLayout,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpriteLayoutInstruction {
    pub start: usize,
    #[serde(default)]
    pub count: Option<usize>,

    #[serde(default)]
    pub repeat: Option<usize>,
    #[serde(default)]
    pub gap: Option<usize>,
}

impl<'de> Deserialize<'de> for PaletteLayout {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(PaletteLayoutVisitor)
    }
}

struct PaletteLayoutVisitor;

impl<'de> Visitor<'de> for PaletteLayoutVisitor {
    type Value = PaletteLayout;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string or a map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(PaletteLayout {
            default: value.to_string(),
            regions: Vec::new(),
        })
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut default = None;
        let mut regions = Vec::new();

        while let Some(elem) = seq.next_element::<toml::Value>()? {
            match elem {
                toml::Value::String(s) => {
                    if default.is_none() {
                        default = Some(s.to_string());
                    } else {
                        return Err(de::Error::custom("Multiple default palettes found"));
                    }
                }
                toml::Value::Table(table) => {
                    let region: PaletteRegion =
                        toml::de::from_str(&table.to_string()).map_err(de::Error::custom)?;
                    regions.push(region);
                }
                _ => {}
            }
        }

        if let Some(default) = default {
            Ok(PaletteLayout { default, regions })
        } else {
            Err(de::Error::custom("No default palette found"))
        }
    }
}
