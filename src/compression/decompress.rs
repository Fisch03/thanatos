use super::{NibblePos, Operation};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Decompressor<'a> {
    src: &'a [u8],
    dst: Vec<u8>,

    /// used for detecting out of bounds errors
    start_index: usize,
    /// used for detecting loops in decompression
    prev_index: usize,
    loop_count: usize,

    /// index to read from
    read_index: usize,
    /// index before a backref is started
    old_index: usize,
    /// bytes remaining in a backref
    backref_remaining: usize,
}

#[derive(Error, Debug)]
pub enum DecompressError {
    #[error("Data contained an invalid operation")]
    InvalidOperation,
    #[error("Loop detected in decompression")]
    LoopDetected,
    #[error("Maximum size exceeded")]
    MaxSizeExceeded,
    #[error("Invalid Data")]
    InvalidData,
}

impl Operation {
    pub fn decode(op: u8, decompressor: &mut Decompressor) -> Self {
        match op {
            n @ 0x00..0x40 => Self::CopySimple(n),

            n @ 0x40..0x50 => Self::decode_copy_nibble_fixed(n, decompressor),
            n @ 0x50..0x60 => Self::CopyDoubled(n & 0x0f),
            n @ 0x60..0x80 => Self::CopyInterleaved {
                count: (n & 0x0f) + 1,
                fixed_value: decompressor.read(),
                fixed_first: n < 0x70,
            },

            n @ 0x80..0xc0 => Self::decode_copy_backread_small(n, decompressor),
            n @ 0xc0..0xe0 => Self::decode_copy_backread_large(n, decompressor),

            n @ 0xe0..0xf0 => Self::decode_repeat_value_large(n, decompressor),
            n @ 0xf0..0xf8 => Self::decode_repeat_value_small(n, decompressor),

            n @ 0xf8..0xfc => Self::decode_backref_large(n, decompressor),
            n @ 0xfc..0xff => Self::decode_backref_small(n, decompressor),

            0xff => Self::Exit,
        }
    }

    fn decode_copy_nibble_fixed(op: u8, decompressor: &mut Decompressor) -> Self {
        let count = (op & 0x0f) + 1;

        let op_2 = decompressor.read();

        let fixed_pos = if (op_2 & 0x10) == 0 {
            NibblePos::Upper
        } else {
            NibblePos::Lower
        };

        let fixed;
        let initial: Option<u8>;
        if op_2 < 0x80 {
            initial = None;
            fixed = op_2 & 0x0f;
        } else {
            initial = Some(op_2 & 0x0f);
            fixed = if (op_2 & 0x40) == 0 { 0x00 } else { 0x0f };
        }

        Self::CopyNibbleFixed {
            count,
            fixed,
            fixed_pos,
            initial,
        }
    }

    fn decode_copy_backread_small(op: u8, decompressor: &mut Decompressor) -> Self {
        let count = (op & 0x7f) / 4 + 1;

        let upper = op & 0x03;
        let lower = decompressor.read_raw();
        let back = u16::from_be_bytes([upper, lower]);

        Self::CopyBackread { count, back }
    }
    fn decode_copy_backread_large(op: u8, decompressor: &mut Decompressor) -> Self {
        let upper = op & 0x1f;
        let lower = decompressor.read();

        let count = ((upper << 1) | (lower >> 7)) + 1;
        let back = u16::from_be_bytes([lower & 0x7f, decompressor.read_raw()]);

        Self::CopyBackread { count, back }
    }

    fn decode_repeat_value_small(op: u8, decompressor: &mut Decompressor) -> Self {
        let count = (op as u16 & 0x07) + 3;
        let value = decompressor.read_raw();

        Self::RepeatValue { count, value }
    }
    fn decode_repeat_value_large(op: u8, decompressor: &mut Decompressor) -> Self {
        let upper = op & 0x0f;
        let lower = decompressor.read();
        let count = u16::from_be_bytes([upper, lower]) + 3;

        let value = decompressor.read_raw();

        Self::RepeatValue { count, value }
    }

    fn decode_backref_small(op: u8, decompressor: &mut Decompressor) -> Self {
        let upper = op & 0x01;
        let lower = decompressor.read_raw();

        let back = (lower & 0x3f) + 2;
        let count = ((u16::from(upper) << 8) | u16::from(lower)) >> 6;

        Self::StartBackref {
            count: count + 3,
            back: back as u16,
        }
    }
    fn decode_backref_large(op: u8, decompressor: &mut Decompressor) -> Self {
        let upper = op & 0x03;
        let lower = decompressor.read_raw();

        let back_upper = lower & 0x1f;
        let back_lower = decompressor.read_raw();
        let back = u16::from_be_bytes([back_upper, back_lower]) + 3;

        let count = ((u16::from(upper) << 8) | u16::from(lower)) >> 5;

        Self::StartBackref {
            count: count + 3,
            back: back as u16,
        }
    }
}

impl<'a> Decompressor<'a> {
    pub fn new(src: &'a [u8], y: usize) -> Self {
        Self {
            src,
            dst: Vec::new(),

            read_index: y,
            start_index: y,

            backref_remaining: 0,
            old_index: 0,
            prev_index: 0,
            loop_count: 0,
        }
    }

    pub fn decompress(mut self) -> Result<Vec<u8>, DecompressError> {
        loop {
            let value = self.read();

            let operation = Operation::decode(value, &mut self);
            //dbg!(&operation);
            log::trace!("operation: {:?}", operation);

            match operation {
                Operation::CopySimple(count) => self.copy_simple(count),
                Operation::CopyNibbleFixed {
                    fixed,
                    fixed_pos,
                    count,
                    initial,
                } => self.copy_nibble_fixed(count, fixed, fixed_pos, initial),
                Operation::CopyDoubled(count) => self.copy_doubled(count),
                Operation::CopyInterleaved {
                    count,
                    fixed_value,
                    fixed_first,
                } => self.copy_interleaved(count, fixed_value, fixed_first),

                Operation::CopyBackread { count, back } => {
                    self.copy_backread(count, back)?;
                }

                Operation::RepeatValue { count, value } => {
                    self.repeat_value(count, value);
                }

                Operation::StartBackref { count, back } => {
                    self.start_backref(count, back)?;
                }

                Operation::Exit => break,
            }

            if self.dst.len() > 0x10000 {
                return Err(DecompressError::MaxSizeExceeded);
            }

            if self.prev_index == self.read_index {
                self.loop_count += 1;
                if self.loop_count > 0x100 {
                    return Err(DecompressError::LoopDetected);
                }
            } else {
                self.loop_count = 0;
            }
            self.prev_index = self.read_index;
        }

        Ok(self.dst)
    }

    fn read_raw(&mut self) -> u8 {
        let value = self.src[self.read_index];
        self.read_index += 1;

        if self.read_index == 0 {
            unreachable!("y overflow (unknown subroutine at a149)");
        }

        value
    }

    fn read(&mut self) -> u8 {
        let value = self.read_raw();
        self.check_backref_end();
        value
    }

    fn check_backref_end(&mut self) {
        if self.backref_remaining == 1 {
            self.read_index = self.old_index;
        }

        self.backref_remaining = self.backref_remaining.saturating_sub(1);
    }

    #[allow(dead_code)]
    fn print_dst(&self) {
        for chunk in self.dst.chunks(16) {
            for v in chunk {
                print!("{:02x}", v);
            }
            println!();
        }
    }

    fn copy_simple(&mut self, count: u8) {
        for _ in 0..=count {
            let value = self.read();
            self.dst.push(value);
        }
    }

    fn copy_doubled(&mut self, count: u8) {
        for _ in 0..=count {
            let value = self.read();

            self.dst.push(value);
            self.dst.push(value);
        }
    }

    fn copy_nibble_fixed(
        &mut self,
        mut count: u8,
        fixed: u8,
        fixed_pos: NibblePos,
        initial: Option<u8>,
    ) {
        if initial.is_some() {
            count += 1;
        }

        let mut current = initial;

        for _ in 0..=count {
            let nibble = if let Some(current) = current.take() {
                current & 0x0f
            } else {
                let value = self.read();
                current = Some(value);
                value >> 4
            };

            let value = match fixed_pos {
                NibblePos::Upper => (fixed << 4) | nibble,
                NibblePos::Lower => (fixed & 0x0f) | (nibble << 4),
            };

            self.dst.push(value);
        }
    }

    fn copy_interleaved(&mut self, count: u8, fixed_value: u8, fixed_first: bool) {
        for _ in 0..=count {
            if fixed_first {
                self.dst.push(fixed_value);

                let read = self.read();
                self.dst.push(read);
            } else {
                let read = self.read();
                self.dst.push(read);

                self.dst.push(fixed_value);
            }
        }
    }

    fn copy_backread(&mut self, count: u8, back: u16) -> Result<(), DecompressError> {
        if self.dst.len() < back as usize || back == 0 {
            return Err(DecompressError::InvalidOperation);
        }

        for _ in 0..=count {
            self.dst.push(self.dst[self.dst.len() - back as usize]);
        }

        self.check_backref_end();

        Ok(())
    }

    fn repeat_value(&mut self, count: u16, value: u8) {
        for _ in 0..count {
            self.dst.push(value);
        }

        self.check_backref_end();
    }

    fn start_backref(&mut self, count: u16, back: u16) -> Result<(), DecompressError> {
        if self.read_index < back as usize || (self.read_index - back as usize) < self.start_index {
            return Err(DecompressError::InvalidOperation);
        }

        self.backref_remaining = count as usize;
        self.old_index = self.read_index;
        self.read_index -= back as usize;

        Ok(())
    }
}
