use std::{fs, path::Path};
use thiserror::Error;

/// Represents a valid Panepon ROM
#[derive(Debug, Clone)]
pub struct Rom {
    data: Vec<u8>,
    version: RomVersion,
}

#[derive(Error, Debug)]
pub enum RomError {
    #[error("Rom CRC doesnt match any of the known Panepon ROM CRCs")]
    InvalidRomType,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RomVersion {
    PaneponJP,
    // PaneponSwitchOnlineRev1,
    // TetrisAttackUS,
    // TetrisAttackEU,
}

impl Rom {
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn version(&self) -> RomVersion {
        self.version
    }

    /// Convenience function to open a ROM file from a path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, RomError> {
        let data = fs::read(path)?;
        Self::from_slice(&data)
    }

    /// Validates the ROM data and returns a new `Rom` instance.
    pub fn from_slice(data: &[u8]) -> Result<Self, RomError> {
        match RomVersion::from_rom(&data) {
            Some(kind) => Ok(Self {
                data: data.to_vec(),
                version: kind,
            }),
            None => Err(RomError::InvalidRomType),
        }
    }
}

impl RomVersion {
    pub fn from_rom(data: &[u8]) -> Option<Self> {
        let crc = crc32fast::hash(data);

        log::debug!("Rom CRC: {:#x}", crc);

        match crc {
            0x14D70786 => Some(RomVersion::PaneponJP),
            0xE3510CB3 => unimplemented!("PaneponSwitchOnlineRev1"),
            0x6C128210 => unimplemented!("TetrisAttackUS"),
            0x7417E83B => unimplemented!("TetrisAttackEU"),
            _ => None,
        }
    }
}
