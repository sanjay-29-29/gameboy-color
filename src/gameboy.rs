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
            let ins = self.map_ram(self.pc);

            match ins {
                0x0 => {
                    // no-op
                    self.pc += 1;
                }
                0x01 => {
                    // Load the 2 bytes of immediate data into register pair BC.
                    //The first byte of immediate data is the lower byte
                    //(i.e., bits 0-7), and the second byte of immediate data
                    //is the higher byte (i.e., bits 8-15).

                    let low = *self.map_ram(self.pc + 1) as u16;
                    let high = *self.map_ram(self.pc + 2) as u16;

                    self.bc = low | (high << 8);

                    self.pc += 3;
                }
                0x02 => {
                    // Store the contents of register A in the
                    // memory location specified by register pair BC.

                    let value = self.get_register_a();
                    let memory = self.map_ram(self.bc);
                    *memory = value;

                    self.pc += 1;
                }
                0x03 => {
                    // Increment the contents of register pair BC by 1.

                    self.bc += 1;

                    self.pc += 1;
                }
                0x04 => {
                    let b = self.get_register_b();
                    let sum = b.wrapping_add(1);

                    self.set_zero_flag(sum == 0);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag((b & 0x0F) == 0x0F);

                    self.set_register_b(sum);

                    self.pc += 1;
                }
                0x05 => {
                    let b = self.get_register_b();
                    let res = b.wrapping_sub(1);

                    self.set_zero_flag(res == 0);
                    self.set_subtraction_flag(true);
                    self.set_half_overflow_flag((b & 0x0F) == 0x00);

                    self.set_register_b(res);

                    self.pc += 1;
                }
                0x06 => {
                    let value = *self.map_ram(self.pc + 1);
                    self.set_register_b(value);

                    self.pc += 2;
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

                    self.pc += 1;
                }
                0x08 => {
                    let addr = *self.map_ram(self.pc + 1) as u16
                        | (*self.map_ram(self.pc + 2) as u16) << 8;

                    let stack_pointer = self.sp;

                    let first_half = self.map_ram(addr);
                    *first_half = (stack_pointer & 0x00FF) as u8;

                    let second_half = self.map_ram(addr + 1);
                    *second_half = (stack_pointer >> 8) as u8;

                    self.pc += 3;
                }
                0x09 => {
                    let (sum, did_overflow) = self.bc.overflowing_add(self.hl);

                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag((self.bc & 0x0FFF) + (self.hl & 0x0FFF) > 0x0FFF);
                    self.set_overflow_flag(did_overflow);

                    self.hl = sum;

                    self.pc += 1;
                }
                0x0A => {
                    let value = *self.map_ram(self.bc);
                    self.set_register_a(value);

                    self.pc += 1;
                }
                0x0B => {
                    self.bc = self.bc.wrapping_sub(1);

                    self.pc += 1;
                }
                0x0C => {
                    let c = self.get_c();
                    let sum = c.wrapping_add(1);

                    self.set_zero_flag(sum == 0);
                    self.set_subtraction_flag(false);
                    self.set_half_overflow_flag((c & 0x0F) == 0x0F);

                    self.set_register_c(sum);

                    self.pc += 1;
                }
                0x0D => {
                    let c = self.get_c();
                    let res = c.wrapping_sub(1);

                    self.set_zero_flag(res == 0);
                    self.set_subtraction_flag(true);
                    self.set_half_overflow_flag((c & 0x0F) == 0x00);

                    self.set_register_c(res);

                    self.pc += 1;
                }
                0x0E => {
                    let value = *self.map_ram(self.pc + 1);
                    self.set_register_c(value);

                    self.pc += 2;
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

                    self.pc += 1;
                }
                0xcb => {}
                _ => {}
            }
        }
    }

    fn set_register_a(&mut self, value: u8) {
        self.af = (self.af & 0x00FF) | (value as u16) << 8;
    }

    fn set_register_b(&mut self, value: u8) {
        self.bc = (self.bc & 0x00FF) | (value as u16) << 8;
    }

    fn set_register_c(&mut self, value: u8) {
        self.bc = (self.bc & 0xFF00) | (value as u16);
    }

    fn get_register_a(&self) -> u8 {
        (self.af >> 8) as u8
    }

    fn get_register_b(&self) -> u8 {
        (self.bc >> 8) as u8
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
}
