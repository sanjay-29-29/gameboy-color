#[derive(Debug)]
pub struct GameBoy {
    ram: [u8; 32 * 1024],
    af: u16,
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
            let opcode = self.fetch_value_u8();
            let (x, y, z) = (opcode >> 6, (opcode >> 3) & 0x07, opcode & 0x07);

            match x {
                0 => {
                    if z == 0 {
                        if y == 0 {
                            // nop
                        }
                        if y == 2 {
                            // stop
                        }
                        if y == 3 {
                            // jr imm8
                            let new_add = self.fetch_value_u8() as i8;
                            self.pc = self.pc.wrapping_add(new_add as i16 as u16);
                        }
                        if y & 0b100 > 1 {
                            // jr cond, imm8
                            let offset = self.fetch_value_u8() as i8;

                            if self.check_condition(y) {
                                self.pc = self.pc.wrapping_add(offset as i16 as u16);
                            }
                        }
                    }
                    if z == 1 && (y & 0b001) == 0 {
                        // ld r16, imm16
                        let val = self.fetch_value_u16();
                        let register = self.get_r16(y);
                        *register = val;
                    }
                    if z == 2 && (y & 0b001) == 0 {
                        // ld [r16mem], a
                        let a = self.get_register_a();
                        let register = self.get_r16mem(y);

                        *register = a;

                        self.post_ins_r16mem(y);
                    }
                    if z == 2 && (y & 0b001) == 1 {
                        // ld a, [r16mem]
                        let val = *self.get_r16mem(y);
                        self.set_register_a(val);
                        self.post_ins_r16mem(y);
                    }
                    if z == 0 && y == 1 {
                        // ld [imm16], sp
                        let addr = self.fetch_value_u16();
                        let sp = self.sp;

                        let mem = self.map_ram(addr);
                        *mem = sp as u8;

                        let mem = self.map_ram(addr.wrapping_add(1));
                        *mem = (sp >> 8) as u8;
                    }
                    if z == 3 && (y & 0b001) == 0 {
                        // inc r16
                        let register = self.get_r16(y);
                        let sum = (*register).wrapping_add(1);
                        *register = sum;
                    }
                    if z == 3 && (y & 0b001) == 1 {
                        // dec r16
                        let register = self.get_r16(y);
                        let sum = (*register).wrapping_sub(1);
                        *register = sum;
                    }
                    if z == 1 && (y & 0b001) == 1 {
                        // add hl, r16
                        let register_val = *self.get_r16(y);
                        let (sum, did_carry) = self.hl.overflowing_add(register_val);

                        self.set_subtraction_flag(false);
                        self.set_half_overflow_flag(
                            (register_val & 0x0FFF) + (self.hl & 0x0FFF) > 0x0FFF,
                        );
                        self.set_overflow_flag(did_carry);

                        self.hl = sum;
                    }
                    if z == 4 {
                        // inc r8
                        let register = self.get_r8(y);
                        let sum = register.wrapping_add(1);

                        self.set_zero_flag(sum == 0);
                        self.set_subtraction_flag(false);
                        self.set_half_overflow_flag(register & 0x0F == 0x0F);

                        self.set_r8(y, sum);
                    }
                    if z == 5 {
                        // dec r8
                        let register = self.get_r8(y);
                        let diff = register.wrapping_sub(1);

                        self.set_zero_flag(diff == 0);
                        self.set_subtraction_flag(true);
                        self.set_half_overflow_flag(register & 0x0F == 0);

                        self.set_r8(y, diff);
                    }
                    if z == 6 {
                        // ld r8, imm8
                        let val = self.fetch_value_u8();
                        self.set_r8(y, val);
                    }
                    if z == 7 {
                        match y {
                            0 => {
                                // rlca
                                let a = self.get_register_a();
                                let last_bit = a >> 7;

                                self.set_zero_flag(false);
                                self.set_subtraction_flag(false);
                                self.set_half_overflow_flag(false);
                                self.set_overflow_flag(last_bit == 1);

                                self.set_register_a(a.rotate_left(1));
                            }
                            1 => {
                                // rrca
                                let a = self.get_register_a();
                                let first_bit = a & 1;

                                self.set_zero_flag(false);
                                self.set_subtraction_flag(false);
                                self.set_half_overflow_flag(false);
                                self.set_overflow_flag(first_bit == 1);

                                self.set_register_a(a.rotate_right(1));
                            }
                            2 => {
                                // rla
                                let mut a = self.get_register_a();
                                let carry = self.get_overflow_flag() as u8;

                                self.set_zero_flag(false);
                                self.set_subtraction_flag(false);
                                self.set_half_overflow_flag(false);
                                self.set_overflow_flag((a & 0x80) > 1);

                                let res = (a << 1) | carry;

                                self.set_register_a(res);
                            }
                            3 => {
                                // rra
                                let mut a = self.get_register_a();
                                let carry = self.get_overflow_flag() as u8;

                                self.set_zero_flag(false);
                                self.set_subtraction_flag(false);
                                self.set_half_overflow_flag(false);
                                self.set_overflow_flag((a & 1) == 1);

                                let res = (a >> 1) | (carry << 7);

                                self.set_register_a(res);
                            }
                            4 => {
                                // daa
                            }
                            5 => {
                                // cpl
                                self.set_register_a(!self.get_register_a());
                                self.set_subtraction_flag(true);
                                self.set_half_overflow_flag(true);
                            }
                            6 => {
                                // scf
                                self.set_subtraction_flag(false);
                                self.set_half_overflow_flag(false);
                                self.set_overflow_flag(true);
                            }
                            7 => {
                                // ccf
                                self.set_subtraction_flag(false);
                                self.set_half_overflow_flag(false);
                                self.set_overflow_flag(!self.get_overflow_flag());
                            }
                            _ => panic!("Invalid OP Code: {opcode}"),
                        }
                    }
                }
                1 => {
                    if y == 6 && z == 6 {
                        // HALT
                    } else {
                        let val = self.get_r8(z);
                        self.set_r8(y, val);
                    }
                }
                2 => {
                    self.handle_alu_op(y, z);
                }
                3 => {}
                _ => {}
            }
        }
    }

    fn check_condition(&self, y: u8) -> bool {
        let res = match y & !0b100 {
            0 => !self.get_zero_flag(),
            1 => self.get_zero_flag(),
            2 => !self.get_overflow_flag(),
            3 => self.get_overflow_flag(),
            _ => panic!("Not a valid condition {y}"),
        };

        res
    }

    fn handle_alu_op(&mut self, y: u8, z: u8) {
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

                let (sum1, carry1) = val.overflowing_add(a);
                let (sum, carry2) = sum1.overflowing_add(overflow);

                self.set_subtraction_flag(false);
                self.set_half_overflow_flag((0x0F & val) + (0x0F & a) + overflow > 0x0F);
                self.set_overflow_flag(carry1 || carry2);

                sum
            }
            2 => {
                let a = self.get_register_a();
                let (diff, did_carry) = a.overflowing_sub(val);

                self.set_subtraction_flag(true);
                self.set_half_overflow_flag((a & 0x0F) < (0x0F & val));
                self.set_overflow_flag(did_carry);

                diff
            }
            3 => {
                let overflow = self.get_overflow_flag() as u8;

                let (diff1, borrow1) = a.overflowing_sub(val);
                let (diff, borrow2) = diff1.overflowing_sub(overflow);

                self.set_subtraction_flag(true);
                self.set_half_overflow_flag((a & 0x0F) < (0x0F & val) + overflow);
                self.set_overflow_flag(borrow1 || borrow2);

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

                return; // CP X instruction does not update the register A
            }
            _ => panic!("Invalid operation {y}"),
        };

        self.set_register_a(res);
        self.set_zero_flag(res == 0);
    }

    fn get_r16(&mut self, y: u8) -> &mut u16 {
        let res = match y >> 1 {
            0x0 => &mut self.bc,
            0x1 => &mut self.de,
            0x2 => &mut self.hl,
            0x3 => &mut self.sp,
            _ => panic!("Not supported"),
        };

        res
    }

    fn get_r16mem(&mut self, y: u8) -> &mut u8 {
        let res = match y >> 1 {
            0x0 => self.map_ram(self.bc),
            0x1 => self.map_ram(self.de),
            0x2 => self.map_ram(self.hl),
            0x3 => self.map_ram(self.hl),
            _ => panic!("Not supported"),
        };

        res
    }

    fn post_ins_r16mem(&mut self, y: u8) {
        match y >> 1 {
            0x0 => {}
            0x1 => {}
            0x2 => self.hl = self.hl.wrapping_add(1),
            0x3 => self.hl = self.hl.wrapping_sub(1),
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

    fn fetch_value_u8(&mut self) -> u8 {
        let val = *self.map_ram(self.pc);
        self.pc = self.pc.wrapping_add(1);

        return val;
    }

    fn fetch_value_u16(&mut self) -> u16 {
        return self.fetch_value_u8() as u16 | (self.fetch_value_u8() as u16) << 8;
    }

    // fn decrement_register(&mut self, reg: u8) -> u8 {
    //     let res = reg.wrapping_sub(1);

    //     self.set_zero_flag(res == 0);
    //     self.set_subtraction_flag(true);
    //     self.set_half_overflow_flag((0x0F & reg) == 0);

    //     res
    // }

    // fn increment_register(&mut self, reg: u8) -> u8 {
    //     let res = reg.wrapping_add(1);

    //     self.set_zero_flag(res == 0);
    //     self.set_subtraction_flag(false);
    //     self.set_half_overflow_flag(reg & 0x0F == 0x0F);

    //     res
    // }

    // fn u16_wrapping_add(&mut self, val1: u16, val2: u16) -> u16 {
    //     let (sum, did_overflow) = val1.overflowing_add(val2);

    //     self.set_subtraction_flag(false);
    //     self.set_half_overflow_flag((val1 & 0x0FFF) + (val2 & 0x0FFF) > 0xFFF);
    //     self.set_overflow_flag(did_overflow);

    //     return sum;
    // }

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
