#[derive(Debug)]
pub struct GameBoy {
    ram: [u8; 32 * 1024],
    af: u16, // 0f bc
    bc: u16,
    de: u16,
    hl: u16,

    sp: u16,
    pc: u16,
}

impl GameBoy {
    pub fn new() -> Self {
        GameBoy {
            ram: [0; 32 * 1024],
            af: 0,
            bc: 0,
            de: 0,
            hl: 0,
            sp: 0,
            pc: 0,
        }
    }

    fn map_ram(&mut self, addr: u16) -> &mut u8 {
        match addr {
            0x0000..=0x3fff => {}
            0x4000..=0x7fff => {}
            0x8000..=0x9fff => {}
            0xa000..=0xbfff => {}
            0xc000..=0xcfff => {}
            0xd000..=0xdfff => {}
            0xe000..=0xfdff => {}
            0xfe00..=0xfe9f => {}
            0xfea0..=0xfeff => {}
            0xff00..=0xff7f => {}
            0xff80..=0xfffe => {}
            0xffff => {}
        }

        return &mut self.ram[0];
    }

    pub fn main(&mut self) {
        loop {
            let ins = self.fetch_value();

            match ins {
                0x0 => {
                    // no-op
                }
                0x01 => {
                    // Load the 2 bytes of immediate data into register pair BC.
                    //The first byte of immediate data is the lower byte
                    //(i.e., bits 0-7), and the second byte of immediate data
                    //is the higher byte (i.e., bits 8-15).

                    let low = self.fetch_value() as u16;
                    let high = self.fetch_value() as u16;

                    self.bc = low | (high << 8);
                }
                0x02 => {
                    // Store the contents of register A in the
                    // memory location specified by register pair BC.

                    let value = self.get_register_a();
                    let memory = self.map_ram(self.bc);
                    *memory = value;
                }
                0x03 => {
                    // Increment the contents of register pair BC by 1.

                    self.bc = self.bc.wrapping_add(1);
                }
                0x04 => {
                    let b = self.get_register_b();
                    let sum = b.wrapping_add(1);

                    self.set_zero_flag(sum == 0);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag((b & 0x0F) == 0x0F);

                    self.set_register_b(sum);
                }
                0x05 => {
                    let b = self.get_register_b();
                    let res = b.wrapping_sub(1);

                    self.set_zero_flag(res == 0);
                    self.set_subtraction_flag(true);
                    self.set_half_overflow_flag((b & 0x0F) == 0x00);

                    self.set_register_b(res);
                }
                0x06 => {
                    let value = self.fetch_value();
                    self.set_register_b(value);
                }
                0x07 => {
                    let mut val = self.get_register_a();
                    let last_bit = (val & 0x80) >> 7;

                    val <<= 1;
                    val |= last_bit;

                    self.set_zero_flag(false);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag(false);
                    self.set_overflow_flag(last_bit == 1);

                    self.set_register_a(val);
                }
                0x08 => {
                    let addr = self.fetch_value() as u16 | (self.fetch_value() as u16) << 8;

                    let stack_pointer = self.sp;

                    let first_half = self.map_ram(addr);
                    *first_half = (stack_pointer & 0x00FF) as u8;

                    let second_half = self.map_ram(addr + 1);
                    *second_half = (stack_pointer >> 8) as u8;
                }
                0x09 => {
                    self.hl = self.u16_wrapping_add(self.bc, self.hl);
                }
                0x0A => {
                    let value = *self.map_ram(self.bc);
                    self.set_register_a(value);
                }
                0x0B => {
                    self.bc = self.bc.wrapping_sub(1);
                }
                0x0C => {
                    let c = self.get_c();
                    let sum = c.wrapping_add(1);

                    self.set_zero_flag(sum == 0);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag((c & 0x0F) == 0x0F);

                    self.set_register_c(sum);
                }
                0x0D => {
                    let res = self.decrement_register(self.get_c());
                    self.set_register_c(res);
                }
                0x0E => {
                    let value = self.fetch_value();
                    self.set_register_c(value);
                }
                0x0F => {
                    let mut a = self.get_register_a();
                    let c = a & 1;

                    a >>= 1;

                    self.set_zero_flag(false);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag(false);
                    self.set_overflow_flag(c == 1);

                    self.set_register_a(a | (c << 7));
                }
                0x10 => {
                    // TODO: STOP
                }
                0x11 => {
                    let val = self.fetch_value() as u16 | (self.fetch_value() as u16) << 8;
                    self.de = val;
                }
                0x12 => {
                    let val = self.get_register_a();
                    let memory = self.map_ram(self.de);

                    *memory = val;
                }
                0x13 => {
                    self.de = self.de.wrapping_add(1);
                }
                0x14 => {
                    let d_val = self.get_register_d();
                    let sum = d_val.wrapping_add(1);

                    self.set_zero_flag(sum == 0);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag(d_val & 0x0F == 0x0F);

                    self.set_register_d(sum);
                }
                0x15 => {
                    let d_val = self.get_register_d();
                    let res = d_val.wrapping_sub(1);

                    self.set_zero_flag(res == 0);
                    self.set_subtraction_flag(true);
                    self.set_half_overflow_flag(d_val & 0x0F == 0);

                    self.set_register_d(res);
                }
                0x16 => {
                    let val = self.fetch_value();
                    self.set_register_d(val);
                }
                0x17 => {
                    let carry_val = self.get_overflow_flag();
                    let mut a = self.get_register_a();

                    self.set_zero_flag(false);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag(false);
                    self.set_overflow_flag((0x80 & a) != 0);

                    a <<= 1;
                    a |= carry_val as u8;

                    self.set_register_a(a);
                }
                0x18 => {
                    let val = self.fetch_value() as i8;
                    let new_adress = self.pc.wrapping_add(val as i16 as u16);
                    self.pc = new_adress as u16;
                }
                0x19 => {
                    self.hl = self.u16_wrapping_add(self.hl, self.de);
                }
                0x1A => {
                    let val = *self.map_ram(self.de);
                    self.set_register_a(val);
                }
                0x1B => {
                    self.de = self.de.wrapping_sub(1);
                }
                0x1C => {
                    let e = self.get_register_e();
                    let sum = e.wrapping_add(1);

                    self.set_zero_flag(sum == 0);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag((0x0F & e) == 0x0F);

                    self.set_register_e(sum);
                }
                0x1D => {
                    let val = self.decrement_register(self.get_register_e());
                    self.set_register_e(val);
                }
                0x1E => {
                    let val = self.fetch_value();
                    self.set_register_e(val);
                }
                0x1F => {
                    let mut a = self.get_register_a();
                    let carry_val = self.get_overflow_flag();

                    self.set_zero_flag(false);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag(false);
                    self.set_overflow_flag(a & 1 == 1);

                    a >>= 1;
                    a = a | (carry_val as u8) << 7;

                    self.set_register_a(a);
                }
                0x20 => {
                    let addr = self.fetch_value() as i8;
                    if !self.get_zero_flag() {
                        self.pc = self.pc.wrapping_add(addr as i16 as u16);
                    }
                }
                0xCB => {}
                _ => {}
            }
        }
    }

    fn fetch_value(&mut self) -> u8 {
        let val = *self.map_ram(self.pc);
        self.pc = self.pc.wrapping_add(1);

        return val;
    }

    fn decrement_register(&mut self, reg: u8) -> u8 {
        let res = reg.wrapping_sub(1);

        self.set_zero_flag(res == 0);
        self.set_subtraction_flag(true);
        self.set_half_overflow_flag((0x0F & reg) == 0);

        return res;
    }

    fn u16_wrapping_add(&mut self, val1: u16, val2: u16) -> u16 {
        let (sum, did_overflow) = val1.overflowing_add(val2);

        self.set_subtraction_flag(false);
        self.set_half_overflow_flag((val1 & 0x0FFF) + (val2 & 0x0FFF) > 0xFFF);
        self.set_overflow_flag(did_overflow);

        return sum;
    }

    fn set_register_a(&mut self, value: u8) {
        self.af = (self.af & 0x00FF) | (value as u16) << 8;
    }

    fn set_register_b(&mut self, value: u8) {
        self.bc = (self.bc & 0x00FF) | (value as u16) << 8;
    }

    fn set_register_c(&mut self, value: u8) {
        self.bc = (self.bc & 0xFF00) | value as u16;
    }

    fn set_register_d(&mut self, value: u8) {
        self.de = (self.de & 0x00FF) | (value as u16) << 8;
    }

    fn set_register_e(&mut self, value: u8) {
        self.de = (self.de & 0xFF00) | value as u16;
    }

    fn get_register_a(&self) -> u8 {
        (self.af >> 8) as u8
    }

    fn get_register_b(&self) -> u8 {
        (self.bc >> 8) as u8
    }

    fn get_register_c(&self) -> u8 {
        self.bc as u8
    }

    fn get_register_d(&self) -> u8 {
        (self.de >> 8) as u8
    }

    fn get_register_e(&self) -> u8 {
        self.de as u8
    }

    fn get_c(&self) -> u8 {
        self.bc as u8
    }

    fn set_zero_flag(&mut self, value: bool) {
        if value {
            self.af |= 0x0080;
        } else {
            self.af &= !0x0080;
        }
    }

    fn get_zero_flag(&self) -> bool {
        (0x0080 & self.af) > 0
    }

    fn set_subtraction_flag(&mut self, value: bool) {
        if value {
            self.af |= 0x0040;
        } else {
            self.af &= !0x0040;
        }
    }

    fn set_half_overflow_flag(&mut self, value: bool) {
        if value {
            self.af |= 0x0020;
        } else {
            self.af &= !0x0020;
        }
    }

    fn set_overflow_flag(&mut self, value: bool) {
        if value {
            self.af |= 0x0010;
        } else {
            self.af &= !0x0010;
        }
    }

    fn get_overflow_flag(&self) -> bool {
        (self.af & 0x0010) > 0
    }
}
