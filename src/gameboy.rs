use std::mem;

fn error(ins: u8) {
    panic!("The program panicked at {ins}");
}

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
            let opcode = self.fetch_value();
            let (x, y, z) = (opcode >> 6, (opcode >> 3) & 0x07, opcode & 0x07);

            match x {
                0x00 => {
                    if y == 0 && z == 0 {
                        // NO-OP
                    }
                    if y == 0 && z == 7 {
                        // RLCA

                        let mut a = self.get_register_a();
                        let last_bit = a >> 7;

                        self.set_zero_flag(false);
                        self.set_subtraction_flag(false);
                        self.set_half_overflow_flag(false);
                        self.set_overflow_flag(last_bit == 1);

                        a <<= 1;
                        self.set_register_a(a | last_bit);
                    }
                    if y == 2 && z == 8 {
                        // RLA
                    }
                }
                0x01 => {
                    if y == 6 && z == 6 {
                        // halt instruction
                    } else {
                        let val = self.get_r8(z);
                        self.set_r8(y, val);
                    }
                }
                0x10 => {
                    let val = self.get_r8(z);
                    let a = self.get_register_a();

                    let res: u8 = match y {
                        0 => {
                            let (sum, did_overflow) = val.overflowing_add(a);

                            self.set_subtraction_flag(false);
                            self.set_half_overflow_flag((0x0F & val) + (0x0F & a) > 0x0F);
                            self.set_overflow_flag(did_overflow);

                            sum
                        }
                        1 => {
                            let overflow = self.get_overflow_flag() as u8;
                            let (sum, did_overflow) = val.overflowing_add(overflow + a);

                            self.set_subtraction_flag(false);
                            self.set_half_overflow_flag(
                                (0x0F & val) + (0x0F & a) + overflow > 0x0F,
                            );
                            self.set_overflow_flag(did_overflow);

                            sum
                        }
                        2 => {
                            let a = self.get_register_a();
                            let (diff, did_carry) = val.overflowing_sub(a);

                            self.set_subtraction_flag(true);
                            self.set_half_overflow_flag((a & 0x0F) > (0x0F & val));
                            self.set_overflow_flag(did_carry);

                            diff
                        }
                        3 => {
                            let a = self.get_register_a();
                            let (diff, did_carry) =
                                a.overflowing_sub(val + self.get_overflow_flag() as u8);

                            self.set_subtraction_flag(true);
                            self.set_half_overflow_flag((a & 0x0F) > (0x0F & val) + val);
                            self.set_overflow_flag(did_carry);

                            diff
                        }
                        4 => {
                            let and = val & a;

                            self.set_subtraction_flag(false);
                            self.set_half_overflow_flag(true);
                            self.set_overflow_flag(false);

                            and
                        }
                        5 => {
                            let xor = val ^ a;

                            self.set_subtraction_flag(false);
                            self.set_half_overflow_flag(false);
                            self.set_overflow_flag(false);

                            xor
                        }
                        6 => {
                            let or = val | a;

                            self.set_subtraction_flag(false);
                            self.set_half_overflow_flag(false);
                            self.set_overflow_flag(false);

                            or
                        }
                        7 => {
                            let (diff, did_carry) = a.overflowing_sub(val);

                            self.set_zero_flag(diff == 0);
                            self.set_subtraction_flag(true);
                            self.set_half_overflow_flag((0x0F & a) < (0x0F & val));
                            self.set_overflow_flag(did_carry);

                            continue; // CP X instruction does not update the register A
                        }
                        _ => panic!("Invalid operation {y}"),
                    };

                    self.set_register_a(res);
                    self.set_zero_flag(res == 0);
                }
                0x11 => {}
                _ => {}
            }
        }
    }

    fn get_r16(&mut self, instruction: u8) -> &mut u16 {
        let res = match instruction >> 4 {
            0x0 => &mut self.bc,
            0x1 => &mut self.de,
            0x2 => &mut self.hl,
            0x3 => &mut self.sp,
            _ => panic!("Not supported"),
        };

        res
    }

    fn get_r16mem(&mut self, instruction: u8) -> &mut u16 {
        let res = match instruction >> 4 {
            0x0 => &mut self.bc,
            0x1 => &mut self.de,
            0x2 => &mut self.hl,
            0x3 => &mut self.hl,
            _ => panic!("Not supported"),
        };

        res
    }

    fn post_ins_r16mem(&mut self, instruction: u8) {
        match instruction >> 4 {
            0x0 => {}
            0x1 => {}
            0x2 => self.hl += 1,
            0x3 => self.hl += 1,
            _ => panic!("Not supported"),
        };
    }

    fn get_r8(&mut self, register: u8) -> u8 {
        let res = match register {
            0 => self.get_register_b(),
            1 => self.get_register_c(),
            2 => self.get_register_d(),
            3 => self.get_register_e(),
            4 => self.get_register_h(),
            5 => self.get_register_l(),
            6 => *self.map_ram(self.hl),
            7 => self.get_register_a(),
            _ => panic!("Trying to get r8 with {register}"),
        };

        res
    }

    fn set_r8(&mut self, register: u8, val: u8) {
        match register {
            0 => self.set_register_b(val),
            1 => self.set_register_c(val),
            2 => self.set_register_d(val),
            3 => self.set_register_e(val),
            4 => self.set_register_h(val),
            5 => self.set_register_l(val),
            6 => {
                let mem = self.map_ram(self.hl);
                *mem = val;
            }
            7 => self.set_register_a(val),
            _ => panic!("Trying to get r8 with {register}"),
        };
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

        res
    }

    fn increment_register(&mut self, reg: u8) -> u8 {
        let res = reg.wrapping_add(1);

        self.set_zero_flag(res == 0);
        self.set_subtraction_flag(false);
        self.set_half_overflow_flag(reg & 0x0F == 0x0F);

        res
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

    fn set_register_h(&mut self, value: u8) {
        self.hl = (self.hl & 0x00FF) | (value as u16) << 8;
    }

    fn set_register_l(&mut self, value: u8) {
        self.hl = (self.hl & 0xFF00) | value as u16;
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

    fn get_register_l(&self) -> u8 {
        self.hl as u8
    }

    fn get_register_h(&self) -> u8 {
        (self.hl >> 8) as u8
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

    fn get_zero_flag(&self) -> bool {
        (0x0080 & self.af) > 0
    }

    fn get_overflow_flag(&self) -> bool {
        (self.af & 0x0010) > 0
    }
}
