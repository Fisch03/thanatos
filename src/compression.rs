mod decompress;
pub use decompress::Decompressor;

#[derive(Debug)]
enum NibblePos {
    Upper,
    Lower,
}

#[derive(Debug)]
enum Operation {
    /// copy n bytes from src to dst
    CopySimple(u8),

    /// copy `count` nibbles(!) from src to dst, forming a full byte by replacing the other half with a fixed value
    CopyNibbleFixed {
        count: u8,
        fixed: u8,
        fixed_pos: NibblePos,
        initial: Option<u8>,
    },

    /// copy `count` bytes from src to dst, but copy each byte twice
    /// (e.g. 0x12 -> 0x12 0x12)
    CopyDoubled(u8),

    /// copy `count` bytes from src to dst, interleaving a fixed value
    /// (e.g. 0x12 -> 0x12 0x00 0x12 0x00)
    CopyInterleaved {
        count: u8,
        fixed_value: u8,

        /// whether the fixed value is first or second
        fixed_first: bool,
    },

    /// copy `count` bytes from (dst-back) to dst
    CopyBackread { count: u8, back: u16 },

    /// write a fixed byte `count` times to dst
    RepeatValue { count: u16, value: u8 },

    /// move the read index back `back` bytes for the next `count` bytes
    StartBackref { count: u16, back: u16 },

    /// decompression is done
    Exit,
}
