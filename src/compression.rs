#[derive(Default)]
pub struct Decompressor<'a> {
    src: &'a [u8],

    a: u8,
    b: u8,
    y: usize,

    carry: bool,

    unknown7f: u8,

    // index before a backref is started
    old_index: usize,
    // bytes remaining in a backref
    backref_remaining: usize,

    dst: Vec<u8>,
}

enum NibblePos {
    Upper,
    Lower,
}

impl<'a> Decompressor<'a> {
    pub fn new(src: &'a [u8], y: usize) -> Self {
        Self {
            y,
            src,
            ..Default::default()
        }
    }

    fn read_raw(&mut self) {
        self.a = self.read_raw_value();
    }

    fn read_raw_value(&mut self) -> u8 {
        let value = self.src[self.y];
        self.y += 1;

        if self.y == 0 {
            unreachable!("y overflow (unknown subroutine at a149)");
        }

        value
    }

    fn read(&mut self) {
        self.read_raw();
        self.check_backref_end();
    }

    fn read_value(&mut self) -> u8 {
        let value = self.read_raw_value();
        self.check_backref_end();
        value
    }

    fn check_backref_end(&mut self) {
        if self.backref_remaining == 1 {
            self.y = self.old_index;
        }

        self.backref_remaining = self.backref_remaining.saturating_sub(1);
    }

    fn load_a_16bit(&self) -> u16 {
        (self.b as u16) << 8 | (self.a as u16)
    }

    fn store_a_16bit(&mut self, value: u16) {
        self.a = (value & 0xff) as u8;
        self.b = ((value >> 8) & 0xff) as u8;
    }

    fn asl_16bit(&mut self) {
        let mut val = self.load_a_16bit();
        self.carry = (val & 0x8000) != 0;
        val <<= 1;
        self.store_a_16bit(val);
    }

    fn xba(&mut self) {
        std::mem::swap(&mut self.a, &mut self.b);
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

    pub fn decompress(mut self) -> Vec<u8> {
        loop {
            let value = self.read_value();

            if value == 0xff {
                break;
            }

            let first = (value & 0x80) != 0;
            let second = (value & 0x40) != 0;

            match (first, second) {
                (false, false) => self.copy_simple(value),
                (false, true) => self.d_a283(value),
                (true, false) => self.op_copy_backread(value),
                (true, true) => self.d_a2ec(value),
            };
        }

        self.dst
    }

    fn copy_simple(&mut self, cnt: u8) {
        for _ in 0..=cnt {
            let value = self.read_value();
            self.dst.push(value);
        }
    }

    fn copy_nibble_fixed(&mut self, cnt: u8) {
        if self.a < 0x10 {
            // fixed upper nibble, read lower nibble
            let fixed = self.a & 0x0f;
            copy_inner(self, fixed, NibblePos::Upper, cnt as usize);
        } else {
            // fixed lower nibble, read upper nibble
            let fixed = self.a & 0x0f;
            copy_inner(self, fixed, NibblePos::Lower, cnt as usize);
        }

        fn copy_inner<'a>(s: &mut Decompressor<'a>, fixed: u8, fixed_pos: NibblePos, cnt: usize) {
            let mut current = None;

            for _ in 0..=cnt {
                let nibble = if let Some(current) = current.take() {
                    current & 0x0f
                } else {
                    let value = s.read_value();
                    current = Some(value);
                    value >> 4
                };

                let value = match fixed_pos {
                    NibblePos::Upper => (fixed << 4) | nibble,
                    NibblePos::Lower => (fixed & 0x0f) | (nibble << 4),
                };

                s.dst.push(value);
            }
        }
    }

    fn op_copy_backread(&mut self, value: u8) {
        let value = value & 0x7f;

        let cnt = value / 4 + 1;

        let upper = value & 0x03;
        let lower = self.read_raw_value();
        let back = u16::from_be_bytes([upper, lower]);

        self.copy_backread(cnt, back);
    }

    fn copy_backread(&mut self, cnt: u8, back: u16) {
        for _ in 0..=cnt {
            self.dst.push(self.dst[self.dst.len() - back as usize]);
        }

        self.check_backref_end();
    }

    fn repeat_value(&mut self, cnt: u16, value: u8) {
        for _ in 0..cnt {
            self.dst.push(value);
        }

        self.check_backref_end();
    }

    fn start_backref(&mut self, back: u16, cnt: u8) {
        self.backref_remaining = cnt as usize;
        self.old_index = self.y;
        self.y -= back as usize;
    }

    fn d_a2ec(&mut self, op: u8) {
        self.a = op;

        if op >= 0xfc {
            self.a &= 0x01;

            self.xba();

            self.read_raw();

            self.asl_16bit();
            self.asl_16bit();

            self.xba();

            let backref_cnt = self.a + 3;

            self.xba();
            self.a >>= 2;

            let mut a = self.load_a_16bit();
            a &= 0x003f;
            a += 2;
            self.store_a_16bit(a);

            self.start_backref(self.load_a_16bit(), backref_cnt);
        } else if op >= 0xf8 {
            self.a &= 0x03;
            self.xba();

            self.read_raw();

            self.asl_16bit();
            self.asl_16bit();
            self.asl_16bit();

            self.a >>= 3;

            self.xba();

            let backref_cnt = self.a + 3;

            self.read_raw();

            let mut a_16 = self.load_a_16bit();
            a_16 += 3;
            self.store_a_16bit(a_16);

            self.start_backref(self.load_a_16bit(), backref_cnt);
        } else if op >= 0xf0 {
            // repeat (up to 10 times)
            let cnt = (op & 0x07) + 3;
            let data = self.read_raw_value();

            self.repeat_value(cnt as u16, data);
        } else if op >= 0xe0 {
            // repeat (16+ times)
            let upper = op & 0x0f;
            let lower = self.read_value();
            let cnt = u16::from_be_bytes([upper, lower]) + 3;

            let data = self.read_raw_value();

            self.repeat_value(cnt, data);
        } else {
            let upper = op & 0x1f;
            let lower = self.read_value();

            let mut value = u16::from_be_bytes([upper, lower]);
            value <<= 1;
            let [cnt, mut back_upper] = value.to_be_bytes();

            back_upper >>= 1;
            let back_lower = self.read_raw_value();
            let back = u16::from_be_bytes([back_upper, back_lower]);

            self.copy_backread(cnt + 1, back);
        }
    }

    fn d_a283(&mut self, op: u8) {
        self.a = op;

        if op < 0x50 {
            self.a &= 0x0f;
            self.a += 1;
            let cnt = self.a;

            self.read();

            if self.a >= 0x80 {
                self.unknown7f = self.a;
                self.a <<= 1;

                if (self.a & 0x80) == 0 {
                    self.a &= 0x20;

                    if self.a == 0 {
                        self.a = self.unknown7f;
                        self.a &= 0x0f;

                        self.dst.push(self.a);

                        self.a = 0;
                    } else {
                        self.a = self.unknown7f;
                        self.a <<= 4;

                        self.dst.push(self.a);

                        self.a = 0x10;
                    }

                    self.copy_nibble_fixed(cnt);
                    return;
                }

                self.a &= 0x20;

                if self.a == 0 {
                    self.a = self.unknown7f;
                    self.a &= 0x0f;
                    self.a |= 0xf0;

                    self.dst.push(self.a);

                    self.a = 0x0f;

                    self.copy_nibble_fixed(cnt);
                    return;
                }

                self.a = self.unknown7f;
                self.a <<= 4;
                self.a |= 0x0f;

                self.dst.push(self.a);

                self.a = 0x1f;
            }

            self.copy_nibble_fixed(cnt);
        } else if op < 0x60 {
            let cnt = op & 0x0f;

            for _ in 0..=cnt {
                self.read();

                self.dst.push(self.a);
                self.dst.push(self.a);
            }
        } else {
            let fixed_value = self.read_value();

            let fixed_first = op < 0x70;
            let cnt = (op & 0x0f) + 1;

            for _ in 0..=cnt {
                if fixed_first {
                    self.dst.push(fixed_value);

                    let read = self.read_value();
                    self.dst.push(read);
                } else {
                    let read = self.read_value();
                    self.dst.push(read);

                    self.dst.push(fixed_value);
                }
            }
        }
    }
}
