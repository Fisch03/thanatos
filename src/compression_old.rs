#[derive(Default)]
struct State<'a> {
    src: &'a [u8],

    a: u8,
    b: u8,
    y: usize,

    carry: bool,

    unknown7f: u8,
    unknown80: u8,
    unknown81: u8,
    unknown82: usize,
    cnt85: usize,

    dst: Vec<u8>,
}

impl<'a> State<'a> {
    fn new(src: &'a [u8], y: usize) -> Self {
        Self {
            y,
            src,
            ..Default::default()
        }
    }

    fn read(&mut self) {
        self.a = self.src[self.y];
        self.y += 1;

        if self.y == 0 {
            unreachable!("y overflow (unknown subroutine at a149)");
        }
    }

    fn load_unknown7f_16bit(&self) -> u16 {
        let mut value = 0;
        value |= self.unknown80 as u16;
        value <<= 8;
        value |= self.unknown7f as u16;
        value
    }

    fn store_unknown7f_16bit(&mut self) {
        self.unknown7f = self.a;
        self.unknown80 = self.b;
    }

    fn neg(&mut self) -> bool {
        (self.a & 0x80) != 0
    }

    fn asl(&mut self) {
        self.carry = (self.a & 0x80) != 0;
        self.a <<= 1;
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

    fn lsr(&mut self) {
        self.carry = (self.a & 0x01) != 0;
        self.a >>= 1;
    }

    fn ror(&mut self) {
        let input = if self.carry { 0x80 } else { 0 };
        self.carry = (self.a & 0x01) != 0;
        self.a >>= 1;
        self.a |= input;
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
}

pub fn decompress(src: &[u8], offset: usize) -> Vec<u8> {
    stacker::grow(1024 * 1024 * 10, || decompress_inner(src, offset))
}

pub fn decompress_inner(src: &[u8], offset: usize) -> Vec<u8> {
    let mut state = State::new(src, offset);

    fn d_a12b(s: &mut State) {
        s.read();

        let stack1 = s.a;
        if s.cnt85 != 0 {
            d_a13d(s);
            s.a = stack1;
        }

        return;
    }

    fn d_a13d(s: &mut State) {
        s.cnt85 -= 1;

        if s.cnt85 == 0 {
            // load $84 and store in $7e (why?)
            s.y = s.unknown82;
        }

        return;
    }

    fn d_a14f(s: &mut State) {
        s.unknown7f = s.a;
        s.asl();

        if !s.neg() {
            return d_a176(s);
        }

        s.a &= 0x20;

        if s.a == 0 {
            return d_a168(s);
        }

        s.a = s.unknown7f;
        s.asl();
        s.asl();
        s.asl();
        s.asl();
        s.a |= 0x0f;

        s.dst.push(s.a);

        s.a = 0x1f;
        return d_a1a0(s);
    }

    fn d_a168(s: &mut State) {
        s.a = s.unknown7f;
        s.a &= 0x0f;
        s.a |= 0xf0;

        s.dst.push(s.a);

        s.a = 0x0f;

        return d_a1a0(s);
    }

    fn d_a176(s: &mut State) {
        s.a &= 0x20;

        if s.a == 0 {
            return d_a188(s);
        }

        s.a = s.unknown7f;
        s.asl();
        s.asl();
        s.asl();
        s.asl();

        s.dst.push(s.a);

        s.a = 0x10;

        return d_a1a0(s);
    }

    fn d_a188(s: &mut State) {
        s.a = s.unknown7f;
        s.a &= 0x0f;

        s.dst.push(s.a);

        s.a = 0;

        return d_a1a0(s);
    }

    fn d_a194(s: &mut State) {
        s.a &= 0x0f;
        s.a += 1;
        s.unknown81 = s.a;

        d_a12b(s);

        if s.a >= 0x80 {
            return d_a14f(s);
        }

        return d_a1a0(s);
    }

    fn d_a1a0(s: &mut State) {
        if s.a < 0x10 {
            return d_a1cc(s);
        }

        s.a &= 0x0f;
        s.unknown7f = s.a;

        return d_a1a8(s);
    }

    fn d_a1a8(s: &mut State) {
        d_a12b(s);

        s.unknown80 = s.a;
        s.a &= 0xf0;
        s.a |= s.unknown7f;

        s.dst.push(s.a);

        let branch = s.unknown81 == 0;
        s.unknown81 = s.unknown81.wrapping_sub(1);
        if branch {
            return d_a1f3(s);
        }

        s.a = s.unknown80;
        s.asl();
        s.asl();
        s.asl();
        s.asl();

        s.a |= s.unknown7f;

        s.dst.push(s.a);

        let branch = s.unknown81 >= 1;
        s.unknown81 = s.unknown81.wrapping_sub(1);
        if branch {
            return d_a1a8(s);
        }

        return d_a2a1(s);
    }

    fn d_a1cc(s: &mut State) {
        s.asl();
        s.asl();
        s.asl();
        s.asl();

        s.unknown7f = s.a;

        return d_a1d2(s);
    }

    fn d_a1d2(s: &mut State) {
        d_a12b(s);
        s.unknown80 = s.a;

        s.lsr();
        s.lsr();
        s.lsr();
        s.lsr();

        s.a |= s.unknown7f;

        s.dst.push(s.a);

        let branch = s.unknown81 == 0;
        s.unknown81 = s.unknown81.wrapping_sub(1);
        if branch {
            return d_a1f3(s);
        }

        s.a = s.unknown80;
        s.a &= 0x0f;
        s.a |= s.unknown7f;

        s.dst.push(s.a);

        let branch = s.unknown81 >= 1;
        s.unknown81 = s.unknown81.wrapping_sub(1);
        if branch {
            return d_a1d2(s);
        }

        return d_a2a1(s);
    }

    fn d_a1f3(s: &mut State) {
        return d_a2a1(s);
    }

    fn d_a1f6(s: &mut State) {
        if s.a < 0x50 {
            return d_a194(s);
        }

        s.a &= 0x0f;
        s.unknown81 = s.a;

        return d_a1fe(s);
    }

    fn d_a1fe(s: &mut State) {
        s.read();
        let stack1 = s.a;
        if s.cnt85 != 0 {
            d_a13d(s);
            s.a = stack1;
        }

        s.dst.push(s.a);
        s.dst.push(s.a);

        let branch = s.unknown81 > 0;
        s.unknown81 = s.unknown81.wrapping_sub(1);
        if branch {
            return d_a1fe(s);
        }

        return d_a2a1(s);
    }

    fn d_a21e(s: &mut State) {
        s.lsr();

        //cmp $60
        //bcc $a2a1

        if s.a < 0x60 {
            return d_a1f6(s);
        }

        s.xba();

        s.read();

        let stack1 = s.a;
        if s.cnt85 != 0 {
            d_a13d(s);
            s.a = stack1;
        }

        s.unknown7f = s.a;

        s.xba();

        let branch = s.a >= 0x70;
        s.a &= 0x0f;
        s.a += 1;

        s.unknown81 = s.a;

        if branch {
            return d_a262(s);
        }

        loop {
            s.a = s.unknown7f;
            s.dst.push(s.a);

            s.read();

            let stack1 = s.a;
            if s.cnt85 != 0 {
                d_a13d(s);
                s.a = stack1;
            }

            s.dst.push(s.a);

            if s.unknown81 == 0 {
                break;
            }
            s.unknown81 -= 1;
        }

        return d_a2a1(s);
    }

    fn d_a262(s: &mut State) {
        loop {
            s.read();

            let stack1 = s.a;
            if s.cnt85 != 0 {
                d_a13d(s);
                s.a = stack1;
            }
            s.dst.push(s.a);
            s.dst.push(s.unknown7f);

            if s.unknown81 == 0 {
                break;
            }
            s.unknown81 -= 1;
        }

        return d_a2a1(s);
    }

    fn d_a283(s: &mut State, neg: bool) {
        if neg {
            return d_a21e(s);
        }

        s.lsr();
        s.unknown81 = s.a;

        return d_a288(s);
    }

    fn d_a288(s: &mut State) {
        s.read();

        let stack1 = s.a;
        if s.cnt85 != 0 {
            d_a13d(s);
            s.a = stack1;
        }

        s.dst.push(s.a);

        let overflow = s.unknown81 == 0;
        s.unknown81 = s.unknown81.wrapping_sub(1);
        if !overflow {
            return d_a288(s);
        }

        return d_a2a1(s);
    }

    // entry point
    fn d_a2a1(s: &mut State) {
        s.read();

        let stack1 = s.a;
        if s.cnt85 != 0 {
            d_a13d(s);
            s.a = stack1;
        }

        s.asl();
        let neg = s.neg();

        if !s.carry {
            return d_a283(s, neg);
        }

        if neg {
            return d_a2ec(s);
        }

        return d_a2b7(s);
    }

    fn d_a2b7(s: &mut State) {
        s.lsr();

        let mut tmp = s.a;
        tmp >>= 2;
        tmp += 1;
        s.unknown81 = tmp;

        s.a &= 0x03;

        s.xba();

        return d_a2c2(s);
    }

    fn d_a2c2(s: &mut State) {
        s.read();

        s.store_unknown7f_16bit();

        s.carry = true;

        let back = s.load_unknown7f_16bit();

        loop {
            s.dst.push(s.dst[s.dst.len() - back as usize]);
            if s.unknown81 == 0 {
                break;
            }
            s.unknown81 -= 1;
        }

        return d_a2e3(s);
    }

    fn d_a2e3(s: &mut State) {
        if s.cnt85 != 0 {
            d_a13d(s);
        }

        return d_a2a1(s);
    }

    fn d_a2ec(s: &mut State) {
        s.ror();

        if s.a >= 0xe0 {
            return d_a311(s);
        }

        s.a &= 0x1f;
        s.xba();

        s.read();

        let stack1 = s.a;
        if s.cnt85 != 0 {
            d_a13d(s);
            s.a = stack1;
        }

        s.asl_16bit();
        s.lsr();
        s.xba();
        s.a += 1;
        s.unknown81 = s.a;

        return d_a2c2(s);
    }

    fn d_a311(s: &mut State) {
        if s.a >= 0xf0 {
            return d_a355(s);
        }

        s.a &= 0x0f;
        s.unknown80 = s.a;

        s.read();

        let stack1 = s.a;
        if s.cnt85 != 0 {
            d_a13d(s);
            s.a = stack1;
        }

        s.unknown7f = s.a;

        s.read();

        let mut a_16 = s.load_unknown7f_16bit();
        a_16 += 3;
        let carry = (a_16 & 1) != 0;
        a_16 >>= 1;

        loop {
            s.dst.push(s.a);
            s.dst.push(s.a);
            a_16 -= 1;
            if a_16 == 0 {
                break;
            }
        }

        if carry {
            s.dst.push(s.a);
        }

        return d_a2e3(s);
    }

    fn d_a355(s: &mut State) {
        if s.a >= 0xf8 {
            return d_a372(s);
        }

        s.a &= 0x07;
        s.a += 2;
        s.unknown81 = s.a;

        s.read();

        loop {
            s.dst.push(s.a);
            if s.unknown81 == 0 {
                break;
            }
            s.unknown81 -= 1;
        }

        return d_a2e3(s);
    }

    fn d_a372(s: &mut State) {
        if s.a >= 0xfc {
            return d_a3bf(s);
        }

        s.a &= 0x03;
        s.xba();

        s.read();

        s.asl_16bit();
        s.asl_16bit();
        s.asl_16bit();

        s.lsr();
        s.lsr();
        s.lsr();

        s.xba();

        let stack1 = s.a;

        s.read();

        let mut a_16 = s.load_a_16bit();
        a_16 += 3;
        s.store_a_16bit(a_16);

        return d_a39b(s, stack1);
    }

    fn d_a39b(s: &mut State, stack1: u8) {
        s.unknown82 = s.y;
        s.store_unknown7f_16bit();

        // load $7e and store in $84 (why?)

        let mut a_16 = s.y;
        a_16 -= s.load_unknown7f_16bit() as usize;
        // omitted code that i dont know how to handle at 0xa3ad
        s.y = a_16;

        s.a = stack1;
        s.a += 3;

        s.cnt85 = s.a as usize;

        return d_a2a1(s);
    }

    fn d_a3bf(s: &mut State) {
        if s.a >= 0xfe {
            return d_a3e2(s);
        }

        s.a &= 0x01;

        s.xba();

        s.read();
        s.asl_16bit();
        s.asl_16bit();

        s.xba();

        let stack1 = s.a;

        s.xba();
        s.lsr();
        s.lsr();

        let mut a = s.load_a_16bit();
        a &= 0x003f;
        a += 2;
        s.store_a_16bit(a);

        return d_a39b(s, stack1);
    }

    fn d_a3e2(_: &mut State) {
        return;
    }

    d_a2a1(&mut state);

    state.dst
}
