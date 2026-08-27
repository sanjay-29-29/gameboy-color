use std::{
    io, thread,
    time::{Duration, Instant},
};

use crate::constants::{DIV_REGISTER, INTERRUPT_FLAG, VBK_ADDR, WRAM_BANK_SELECT};

#[derive(Debug)]
pub struct GameBoy {
    catrigde_rom: [u8; 512 * 1024],
    catridge_ram: [u8; 32 * 1024],

    catridge_selected_rom: u8,
    catridge_selected_ram: u8,

    w_ram: [u8; 32 * 1024],  // Work RAM
    v_ram: [u8; 16 * 1024],  // Video RAM
    h_ram: [u8; 128],        // High RAM
    oam: [u8; 160],          // Object Attribute Memory
    io_registers: [u8; 128], // IO Registers

    // GP registers
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,

    sp: u16, // Stack Pointer
    pc: u16, // Program Counter

    interrupt_master_enable: bool, // Interrupt Master Enable Flag
    interrupt_enable: u8,          // Interrupt Enable

    external_ram_enabled: bool,
    cpu_timer: u32,
    cpu_halted: bool,
}

impl GameBoy {
    pub fn new(rom: Vec<u8>) -> Self {
        let mut gb = GameBoy {
            af: 0x01B0,
            bc: 0x0013,
            de: 0x00D8,
            hl: 0x014D,
            sp: 0xFFFE,
            pc: 0x3FF0,

            w_ram: [0; 32 * 1024],
            v_ram: [0; 16 * 1024],
            h_ram: [0; 128],
            oam: [0; 160],
            io_registers: [0; 128],

            catrigde_rom: [0; 512 * 1024],
            catridge_ram: [0; 32 * 1024],
            external_ram_enabled: false,
            catridge_selected_rom: 1,
            catridge_selected_ram: 0,

            interrupt_master_enable: false, // disabled when game starts running
            interrupt_enable: 0,
            cpu_timer: 0,
            cpu_halted: false,
        };

        gb.load_rom(rom);

        gb
    }

    fn load_rom(&mut self, rom: Vec<u8>) {
        for i in 0..rom.len() {
            self.catrigde_rom[i] = rom[i];
        }
    }

    fn write_ram(&mut self, addr: u16, val: u8) {
        self.increment_cpu_timer(1);

        let addr_usize = addr as usize;

        match addr {
            0x0000..=0x1fff => {
                // external RAM enabled by writing $A
                if val & 0x0F == 0xA {
                    self.external_ram_enabled = true;
                }
            }
            0x2000..=0x3fff => {
                // ROM Bank
                let mut selected_bank = val & 0x1F;

                if selected_bank == 0 {
                    selected_bank = 1;
                }

                self.catridge_selected_rom = selected_bank;
            }
            0x4000..=0x5fff => {
                // RAM Bank
                self.catridge_selected_ram = 0x03 & val;
            }
            0x6000..=0x7fff => {
                // ROM
            }
            0x8000..=0x9fff => {
                // VRAM
                if self.read_ram(VBK_ADDR) & 1 == 0 {
                    self.v_ram[0x2000 + (addr_usize - 0x8000)] = val;
                }
                self.v_ram[addr_usize - 0x8000] = val;
            }
            0xa000..=0xbfff => {
                // 8 KiB External RAM
                self.catridge_ram[addr_usize - 0xa000] = val;
            }
            0xc000..=0xcfff => {
                // 4 KiB Work RAM (WRAM)
                // Bank 0
                self.w_ram[addr_usize - 0xc000] = val;
            }
            0xd000..=0xdfff => {
                // switchable bank 1–7
                let mut selected_bank =
                    (self.io_registers[WRAM_BANK_SELECT - 0xff00] >> 5) as usize;

                if selected_bank == 0 {
                    selected_bank = 1; // 0 maps to Bank 1
                }

                self.w_ram[(0x1000 * selected_bank) + (addr_usize - 0xd000)] = val;
            }
            0xe000..=0xfdff => {
                // Echo RAM (mirror of C000–DDFF)
                self.write_ram(addr - 0x2000, val);
            }
            0xfe00..=0xfe9f => {
                // Object attribute memory (OAM)
                self.oam[addr_usize - 0xfe00] = val;
            }
            0xfea0..=0xfeff => {
                // return 0xFF;
            }
            0xff00..=0xff7f => {
                if addr == 0xFF02 {
                    // If bit 7 (0x80) is set, a transfer is starting
                    if val == 0x81 {
                        let char_to_print = self.read_ram(0xFF01) as char;
                        print!("{}", char_to_print);
                    }
                }
                
                if addr_usize == DIV_REGISTER {
                    self.io_registers[addr_usize - 0xff00] = 0;
                }

                // I/O Registers
                self.io_registers[addr_usize - 0xff00] = val;
            }
            0xff80..=0xfffe => {
                // High RAM (HRAM)
                self.h_ram[addr_usize - 0xff80] = val;
            }
            0xffff => {
                self.interrupt_enable = val;
            }
        }
    }

    fn read_ram(&mut self, addr: u16) -> u8 {
        self.increment_cpu_timer(1);

        let addr_usize = addr as usize;

        match addr {
            0x0000..=0x3fff => {
                // 16 KiB ROM bank 00
                return self.catrigde_rom[addr_usize];
            }
            0x4000..=0x7fff => {
                // 16 KiB ROM Bank 01–NN
                return self.catrigde_rom
                    [(0x4000 * self.catridge_selected_rom as usize) + (addr_usize - 0x4000)];
            }
            0x8000..=0x9fff => {
                // VRAM
                let mut base_addr: usize = 0;

                if self.read_ram(VBK_ADDR) & 1 == 1 {
                    base_addr = 0x2000;
                }

                return self.v_ram[base_addr + (addr_usize - 0x8000)];
            }
            0xa000..=0xbfff => {
                // 8 KiB External RAM
                return self.catridge_ram
                    [(0x2000 * self.catridge_selected_ram as usize) + (addr_usize - 0xa000)];
            }
            0xc000..=0xcfff => {
                // 4 KiB Work RAM (WRAM)
                // Bank 0
                return self.w_ram[addr_usize - 0xc000];
            }
            0xd000..=0xdfff => {
                // switchable bank 1–7
                // 4 KiB Work RAM (WRAM)
                let mut selected_bank = (self.read_ram(WRAM_BANK_SELECT as u16) & 0x07) as usize;

                if selected_bank == 0 {
                    selected_bank = 1; // 0 maps to Bank 1
                }

                return self.w_ram[(0x1000 * selected_bank) + (addr_usize - 0xd000)];
            }
            0xe000..=0xfdff => {
                // Echo RAM (mirror of C000–DDFF)
                return self.read_ram(addr - 0x2000);
            }
            0xfe00..=0xfe9f => {
                // Object attribute memory (OAM)
                return self.oam[addr_usize - 0xfe00];
            }
            0xfea0..=0xfeff => {
                return 0xFF;
            }
            0xff00..=0xff7f => {
                // I/O Registers
                return self.io_registers[addr_usize - 0xff00];
            }
            0xff80..=0xfffe => {
                // High RAM (HRAM)
                return self.h_ram[addr_usize - 0xff80];
            }
            0xffff => {
                return self.interrupt_enable;
            }
        }
    }

    pub fn main(&mut self) {
        loop {
            if self.cpu_halted {
                if self.read_ram(INTERRUPT_FLAG) & self.interrupt_enable & 0x0F > 1 {
                    self.cpu_halted = false;
                }
            }

            if !self.cpu_halted {
                self.fde();
                self.update_timers();
            }
        }
    }

    // pub fn handle_interrupt(&mut self) {
    //     if !self.interrupt_master_enable {
    //         return;
    //     }
    //
    //     for i in 0..=4 {
    //         if (self.interrupt_enable >> i) & (self.read_ram(INTERRUPT_FLAG) >> i) & 1 != 1 {
    //             continue;
    //         }

    //         self.clear_interrupt(i);
    //         self.interrupt_master_enable = false;
    //         self.push_to_stack(self.pc);

    //         match i {
    //             0 => 0x0040, // VBlank
    //             1 => 0x0048, // LCD STAT
    //             2 => 0x0050, // Timer
    //             3 => 0x0058, // Serial
    //             4 => 0x0060, // Joypad
    //             _ => panic!("Invalid Interrupt {i}"),
    //         };

    //         break;
    //     }
    // }

    fn update_timers(&mut self) {
        self.io_registers[DIV_REGISTER - 0xff00] = (self.cpu_timer / 128) as u8; 
    }

    fn fde(&mut self) {
        let opcode = self.fetch_value_u8();
        let (x, y, z) = (opcode >> 6, (opcode >> 3) & 0x07, opcode & 0x07);

        // println!("{opcode} {:x}", self.pc);
        // thread::sleep(Duration::from_millis(1));

        match x {
            0 => {
                if z == 0 {
                    if y == 0 {
                        // nop
                    }
                    if y == 2 {
                        // TODO: stop
                    }
                    if y == 3 {
                        // jr imm8
                        let new_add = self.fetch_value_u8() as i8;
                        self.pc = self.pc.wrapping_add(new_add as i16 as u16);

                        self.increment_cpu_timer(1);
                    }
                    if y & 0b100 > 1 {
                        // jr cond, imm8
                        let offset = self.fetch_value_u8() as i8;

                        if self.check_condition(y) {
                            self.increment_cpu_timer(1);
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

                    self.set_r16mem(y, a);
                    self.post_ins_r16mem(y);
                }
                if z == 2 && (y & 0b001) == 1 {
                    // ld a, [r16mem]
                    let val = self.get_r16mem(y);

                    self.set_register_a(val);
                    self.post_ins_r16mem(y);
                }
                if z == 0 && y == 1 {
                    // ld [imm16], sp
                    let addr = self.fetch_value_u16();
                    let sp = self.sp;

                    self.write_ram(addr, sp as u8);
                    self.write_ram(addr.wrapping_add(1), (sp >> 8) as u8);
                }
                if z == 3 && (y & 0b001) == 0 {
                    // inc r16
                    let register = self.get_r16(y);
                    let sum = (*register).wrapping_add(1);

                    *register = sum;

                    self.increment_cpu_timer(1);
                }
                if z == 3 && (y & 0b001) == 1 {
                    // dec r16
                    let register = self.get_r16(y);
                    let sum = (*register).wrapping_sub(1);

                    *register = sum;

                    self.increment_cpu_timer(1);
                }
                if z == 1 && (y & 0b001) == 1 {
                    // add hl, r16
                    let register_val = *self.get_r16(y);
                    let (sum, did_carry) = self.hl.overflowing_add(register_val);

                    self.set_subtraction_flag(false);
                    self.set_half_carry_flag((register_val & 0x0FFF) + (self.hl & 0x0FFF) > 0x0FFF);
                    self.set_carry_flag(did_carry);

                    self.hl = sum;

                    self.increment_cpu_timer(1);
                }
                if z == 4 {
                    // inc r8
                    let register = self.get_r8(y);
                    let sum = register.wrapping_add(1);

                    self.set_zero_flag(sum == 0);
                    self.set_subtraction_flag(false);
                    self.set_half_carry_flag(register & 0x0F == 0x0F);

                    self.set_r8(y, sum);
                }
                if z == 5 {
                    // dec r8
                    let register = self.get_r8(y);
                    let diff = register.wrapping_sub(1);

                    self.set_zero_flag(diff == 0);
                    self.set_subtraction_flag(true);
                    self.set_half_carry_flag(register & 0x0F == 0);

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
                            self.set_half_carry_flag(false);
                            self.set_carry_flag(last_bit == 1);

                            self.set_register_a(a.rotate_left(1));
                        }
                        1 => {
                            // rrca
                            let a = self.get_register_a();
                            let first_bit = a & 1;

                            self.set_zero_flag(false);
                            self.set_subtraction_flag(false);
                            self.set_half_carry_flag(false);
                            self.set_carry_flag(first_bit == 1);

                            self.set_register_a(a.rotate_right(1));
                        }
                        2 => {
                            // rla
                            let a = self.get_register_a();
                            let carry = self.get_carry_flag() as u8;

                            self.set_zero_flag(false);
                            self.set_subtraction_flag(false);
                            self.set_half_carry_flag(false);
                            self.set_carry_flag((a & 0x80) > 1);

                            let res = (a << 1) | carry;

                            self.set_register_a(res);
                        }
                        3 => {
                            // rra
                            let a = self.get_register_a();
                            let carry = self.get_carry_flag() as u8;

                            self.set_zero_flag(false);
                            self.set_subtraction_flag(false);
                            self.set_half_carry_flag(false);
                            self.set_carry_flag((a & 1) == 1);

                            let res = (a >> 1) | (carry << 7);

                            self.set_register_a(res);
                        }
                        4 => {
                            // daa
                            let mut a = self.get_register_a();
                            let mut adjust = 0u8;
                            let mut carry = self.get_carry_flag();

                            if self.get_subtraction_flag() {
                                if self.get_half_carry_flag() {
                                    adjust |= 0x06;
                                }
                                if carry {
                                    adjust |= 0x60;
                                }
                                a = a.wrapping_sub(adjust);
                            } else {
                                if self.get_half_carry_flag() || (a & 0x0F) > 0x09 {
                                    adjust |= 0x06;
                                }
                                if carry || a > 0x99 {
                                    adjust |= 0x60;
                                    carry = true;
                                }
                                a = a.wrapping_add(adjust);
                            }

                            self.set_zero_flag(a == 0);
                            self.set_half_carry_flag(false);
                            self.set_carry_flag(carry);
                            self.set_register_a(a);
                        }
                        5 => {
                            // cpl
                            self.set_register_a(!self.get_register_a());
                            self.set_subtraction_flag(true);
                            self.set_half_carry_flag(true);
                        }
                        6 => {
                            // scf
                            self.set_subtraction_flag(false);
                            self.set_half_carry_flag(false);
                            self.set_carry_flag(true);
                        }
                        7 => {
                            // ccf
                            self.set_subtraction_flag(false);
                            self.set_half_carry_flag(false);
                            self.set_carry_flag(!self.get_carry_flag());
                        }
                        _ => panic!("Invalid OP Code: {opcode}"),
                    }
                }
            }
            1 => {
                if y == 6 && z == 6 {
                    // TODO: halt
                    self.cpu_halted = true;
                    println!("Halted");
                } else {
                    let val = self.get_r8(z);
                    self.set_r8(y, val);
                }
            }
            2 => {
                let val = self.get_r8(z);
                self.handle_alu_op(y, val);
            }
            3 => {
                if z == 6 {
                    // alu ops
                    let val = self.fetch_value_u8();
                    self.handle_alu_op(y, val);
                }
                if z == 0 && (y & 0b100) == 0 {
                    // ret cond
                    self.increment_cpu_timer(1);

                    if self.check_condition(y) {
                        self.pc = self.pop_from_stack();
                        self.increment_cpu_timer(1);
                    }
                }
                if z == 1 && y == 1 {
                    // ret
                    self.pc = self.pop_from_stack();
                    self.increment_cpu_timer(1);
                }
                if z == 1 && y == 3 {
                    // reti
                    self.interrupt_master_enable = true;
                    self.pc = self.pop_from_stack();
                    self.increment_cpu_timer(1);
                }
                if z == 2 && (y & 0b100) == 0 {
                    // jp cond, imm16
                    let addr = self.fetch_value_u16();

                    if self.check_condition(y) {
                        self.pc = addr;
                        self.increment_cpu_timer(1);
                    }
                }
                if z == 3 && y == 0 {
                    // jp imm16
                    self.pc = self.fetch_value_u16();
                    self.increment_cpu_timer(1);
                }
                if z == 1 && y == 5 {
                    // jp hl
                    self.pc = self.hl;
                }
                if z == 4 && (y & 0b100) == 0 {
                    // call cond, imm16
                    let val = self.fetch_value_u16();

                    if self.check_condition(y) {
                        self.push_to_stack(self.pc);
                        self.pc = val;
                        self.increment_cpu_timer(1);
                    }
                }
                if z == 5 && y == 1 {
                    // call imm16
                    let val = self.fetch_value_u16();
                    self.push_to_stack(self.pc);
                    self.pc = val;
                    self.increment_cpu_timer(1);
                }
                if z == 7 {
                    // rst tgt3
                    self.push_to_stack(self.pc);
                    self.pc = 0x0000 + (8 * y as u16); // JMP to offset + (y * 8)
                    self.increment_cpu_timer(1);
                }
                if z == 1 && (y & 0b001) == 0 {
                    // pop r16stk
                    let val = self.pop_from_stack();

                    if y == 6 {
                        // force the bottom 4 bits to 0
                        self.af = val & 0xFFF0;
                    } else {
                        let register = self.get_r16stk(y);
                        *register = val;
                    }
                }
                if z == 5 && (y & 0b001) == 0 {
                    // push r16stk
                    let mut register = *self.get_r16stk(y);

                    if y == 6 {
                        register &= 0xFFF0;
                    }

                    self.push_to_stack(register);
                    self.increment_cpu_timer(1);
                }
                if z == 3 && y == 1 {
                    // $CB prefix instructions
                    let ins = self.fetch_value_u8();
                    self.handle_prefix_cb_instruction(ins);
                }
                if z == 2 && y == 4 {
                    // ldh [c], a
                    let c = self.get_register_c();
                    let a = self.get_register_a();

                    self.write_ram((0xFF00_u16).wrapping_add(c as u16), a);
                }
                if z == 0 && y == 4 {
                    // ldh [imm8], a
                    let val = self.fetch_value_u8() as u16;
                    let a = self.get_register_a();

                    self.write_ram((0xFF00_u16).wrapping_add(val), a);
                }
                if z == 2 && y == 5 {
                    // ld [imm16], a
                    let a = self.get_register_a();
                    let addr = self.fetch_value_u16();

                    self.write_ram(addr, a);
                }
                if z == 2 && y == 6 {
                    // ldh a, [c]
                    let c = self.get_register_c();
                    let val = self.read_ram((0xFF00_u16).wrapping_add(c as u16));

                    self.set_register_a(val);
                }
                if z == 0 && y == 6 {
                    // ldh a, [imm8]
                    let addr = self.fetch_value_u8() as u16;
                    let val = self.read_ram((0xFF00_u16).wrapping_add(addr));

                    self.set_register_a(val);
                }
                if z == 2 && y == 7 {
                    // ld a, [imm16]
                    let addr = self.fetch_value_u16();
                    let val = self.read_ram(addr);

                    self.set_register_a(val);
                }
                if z == 0 && y == 5 {
                    // add sp, imm8
                    let val = self.fetch_value_u8();
                    let sum = self.sp.wrapping_add(val as i8 as i16 as u16);

                    self.set_zero_flag(false);
                    self.set_subtraction_flag(false);
                    self.set_half_carry_flag((0x0F & self.sp) + (0x0F & (val as u16)) > 0x0F);
                    self.set_carry_flag((self.sp & 0xFF) + (val as u16 & 0xFF) > 0xFF);

                    self.sp = sum;

                    self.increment_cpu_timer(2);
                }
                if z == 0 && y == 7 {
                    // ld hl, sp + imm8
                    let val = self.fetch_value_u8();
                    let sum = self.sp.wrapping_add(val as i8 as i16 as u16);

                    self.set_zero_flag(false);
                    self.set_subtraction_flag(false);
                    self.set_half_carry_flag((self.sp & 0x0F) + (val as u16 & 0x0F) > 0x0F);
                    self.set_carry_flag((self.sp & 0xFF) + (val as u16 & 0xFF) > 0xFF);

                    self.hl = sum;

                    self.increment_cpu_timer(1);
                }
                if z == 1 && y == 7 {
                    // ld sp, hl
                    self.sp = self.hl;
                    self.increment_cpu_timer(1);
                }
                if z == 3 && y == 6 {
                    // di
                    self.interrupt_master_enable = false;
                }
                if z == 3 && y == 7 {
                    // ei
                    self.interrupt_master_enable = true;
                }
            }
            _ => panic!("Invalid OP code {opcode}"),
        }
    }

    fn check_condition(&mut self, y: u8) -> bool {
        let res = match y & !0b100 {
            0 => !self.get_zero_flag(),
            1 => self.get_zero_flag(),
            2 => !self.get_carry_flag(),
            3 => self.get_carry_flag(),
            _ => panic!("Not a valid condition {y}"),
        };

        res
    }

    fn handle_prefix_cb_instruction(&mut self, opcode: u8) {
        let (x, y, z) = (opcode >> 6, (opcode >> 3) & 0x07, opcode & 0x07);
        let register = self.get_r8(z);

        match x {
            0 => {
                let res: u8 = match y {
                    0 => {
                        // rlc r8
                        let last_bit = register >> 7;

                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag(last_bit == 1);

                        register.rotate_left(1)
                    }
                    1 => {
                        // rrc r8
                        let first_bit = register & 1;

                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag(first_bit == 1);

                        register.rotate_right(1)
                    }
                    2 => {
                        // rl r8
                        let carry = self.get_carry_flag() as u8;

                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag((register & 0x80) > 1);

                        (register << 1) | carry
                    }
                    3 => {
                        // rr r8
                        let carry = self.get_carry_flag() as u8;

                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag((register & 1) == 1);

                        (register >> 1) | (carry << 7)
                    }
                    4 => {
                        // sla r8
                        let last_bit = register >> 7;

                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag(last_bit == 1);

                        register << 1
                    }
                    5 => {
                        // sra r8
                        let first_bit = register & 1;
                        let last_bit = register & 0x80;

                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag(first_bit == 1);

                        (register >> 1) | last_bit
                    }
                    6 => {
                        // swap r8
                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag(false);

                        let lower_bits = 0x0F & register;
                        let upper_bits = 0xF0 & register;

                        lower_bits << 4 | upper_bits >> 4
                    }
                    7 => {
                        // srl r8
                        self.set_subtraction_flag(false);
                        self.set_half_carry_flag(false);
                        self.set_carry_flag(register & 1 == 1);

                        register >> 1
                    }
                    _ => {
                        panic!("Invalid op with prefix $CB {opcode}");
                    }
                };

                self.set_zero_flag(res == 0);
                self.set_r8(z, res);
            }
            1 => {
                // bit b3, r8
                let register = self.get_r8(z);

                self.set_zero_flag(!((register >> y) & 1 == 1));
                self.set_subtraction_flag(false);
                self.set_half_carry_flag(true);
            }
            2 => {
                // res b3, r8
                self.set_r8(z, register & !(1 << y));
            }
            3 => {
                // set b3, r8
                self.set_r8(z, register | (1 << y));
            }
            _ => panic!("Invalid OP code {opcode}"),
        }
    }

    fn handle_alu_op(&mut self, y: u8, val: u8) {
        let a = self.get_register_a();

        let res: u8 = match y {
            0 => {
                // add a, r8
                let (sum, did_overflow) = val.overflowing_add(a);

                self.set_subtraction_flag(false);
                self.set_half_carry_flag((0x0F & val) + (0x0F & a) > 0x0F);
                self.set_carry_flag(did_overflow);

                sum
            }
            1 => {
                // adc a, r8
                let overflow = self.get_carry_flag() as u8;

                let (sum1, carry1) = val.overflowing_add(a);
                let (sum, carry2) = sum1.overflowing_add(overflow);

                self.set_subtraction_flag(false);
                self.set_half_carry_flag((0x0F & val) + (0x0F & a) + overflow > 0x0F);
                self.set_carry_flag(carry1 || carry2);

                sum
            }
            2 => {
                // sub a, r8
                let a = self.get_register_a();
                let (diff, did_carry) = a.overflowing_sub(val);

                self.set_subtraction_flag(true);
                self.set_half_carry_flag((a & 0x0F) < (0x0F & val));
                self.set_carry_flag(did_carry);

                diff
            }
            3 => {
                // sbc a, r8
                let overflow = self.get_carry_flag() as u8;

                let (diff1, borrow1) = a.overflowing_sub(val);
                let (diff, borrow2) = diff1.overflowing_sub(overflow);

                self.set_subtraction_flag(true);
                self.set_half_carry_flag((a & 0x0F) < (0x0F & val) + overflow);
                self.set_carry_flag(borrow1 || borrow2);

                diff
            }
            4 => {
                // and a, r8
                let and = val & a;

                self.set_subtraction_flag(false);
                self.set_half_carry_flag(true);
                self.set_carry_flag(false);

                and
            }
            5 => {
                // xor a, r8
                let xor = val ^ a;

                self.set_subtraction_flag(false);
                self.set_half_carry_flag(false);
                self.set_carry_flag(false);

                xor
            }
            6 => {
                // or a, r8
                let or = val | a;

                self.set_subtraction_flag(false);
                self.set_half_carry_flag(false);
                self.set_carry_flag(false);

                or
            }
            7 => {
                // cp a, r8
                let (diff, did_carry) = a.overflowing_sub(val);

                self.set_zero_flag(diff == 0);
                self.set_subtraction_flag(true);
                self.set_half_carry_flag((0x0F & a) < (0x0F & val));
                self.set_carry_flag(did_carry);

                return; // instruction does not update the register A
            }
            _ => panic!("Invalid ALU operation {y}"),
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

    fn get_r16stk(&mut self, y: u8) -> &mut u16 {
        let res = match y >> 1 {
            0x0 => &mut self.bc,
            0x1 => &mut self.de,
            0x2 => &mut self.hl,
            0x3 => &mut self.af,
            _ => panic!("Not supported"),
        };

        res
    }

    fn get_r16mem(&mut self, y: u8) -> u8 {
        let res = match y >> 1 {
            0x0 => self.read_ram(self.bc),
            0x1 => self.read_ram(self.de),
            0x2 => self.read_ram(self.hl),
            0x3 => self.read_ram(self.hl),
            _ => panic!("Not supported"),
        };

        res
    }

    fn set_r16mem(&mut self, y: u8, val: u8) {
        match y >> 1 {
            0x0 => self.write_ram(self.bc, val),
            0x1 => self.write_ram(self.de, val),
            0x2 => self.write_ram(self.hl, val),
            0x3 => self.write_ram(self.hl, val),
            _ => panic!("Not supported"),
        };
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
            6 => self.read_ram(self.hl), 
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
            6 => self.write_ram(self.hl, val),
            7 => self.set_register_a(val),
            _ => panic!("Trying to set r8 with {register}"),
        };
    }

    fn fetch_value_u8(&mut self) -> u8 {
        let val = self.read_ram(self.pc);
        self.pc = self.pc.wrapping_add(1);

        return val;
    }

    fn fetch_value_u16(&mut self) -> u16 {
        return self.fetch_value_u8() as u16 | (self.fetch_value_u8() as u16) << 8;
    }

    fn pop_from_stack(&mut self) -> u16 {
        let mut val = self.read_ram(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        val |= (self.read_ram(self.sp) as u16) << 8;
        self.sp = self.sp.wrapping_add(1);

        return val;
    }

    fn push_to_stack(&mut self, val: u16) {
        self.sp = self.sp.wrapping_sub(1);
        self.write_ram(self.sp, (val >> 8) as u8);

        self.sp = self.sp.wrapping_sub(1);
        self.write_ram(self.sp, val as u8);
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

    fn set_half_carry_flag(&mut self, value: bool) {
        if value {
            self.af |= 0x0020;
        } else {
            self.af &= !0x0020;
        }
    }

    fn set_carry_flag(&mut self, value: bool) {
        if value {
            self.af |= 0x0010;
        } else {
            self.af &= !0x0010;
        }
    }

    fn get_zero_flag(&self) -> bool {
        (self.af & 0x0080) > 0
    }

    fn get_half_carry_flag(&self) -> bool {
        (self.af & 0x0020) > 0
    }

    fn get_carry_flag(&self) -> bool {
        (self.af & 0x0010) > 0
    }

    fn get_subtraction_flag(&self) -> bool {
        (self.af & 0x0040) > 0
    }

    // fn is_vblank_interrupt_requested(&self) -> bool {
    //     self.read_ram(INTERRUPT_FLAG) & 1 == 1
    // }

    // fn is_lcd_interrupt_requested(&self) -> bool {
    //     (self.read_ram(INTERRUPT_FLAG) >> 1) & 1 == 1
    // }

    // fn is_timer_interrupt_requesis(&self) -> bool {
    //     (self.read_ram(INTERRUPT_FLAG) >> 2) & 1 == 1
    // }

    // fn is_serial_interrupt_requested(&self) -> bool {
    //     (self.read_ram(INTERRUPT_FLAG) >> 3) & 1 == 1
    // }

    // fn is_joypad_interrupt_requested(&self) -> bool {
    //     (self.read_ram(INTERRUPT_FLAG) >> 4) & 1 == 1
    // }

    // fn is_vblank_interrupt_enabled(&self) -> bool {
    //     self.interrupt_enable & 1 == 1
    // }

    // fn is_lcd_interrupt_enabled(&self) -> bool {
    //     (self.interrupt_enable >> 1) & 1 == 1
    // }

    // fn is_timer_interrupt_enabled(&self) -> bool {
    //     (self.interrupt_enable >> 2) & 1 == 1
    // }

    // fn is_serial_interrupt_enabled(&self) -> bool {
    //     (self.interrupt_enable >> 3) & 1 == 1
    // }

    // fn is_joypad_interrupt_enabled(&self) -> bool {
    //     (self.interrupt_enable >> 4) & 1 == 1
    // }

    fn clear_interrupt(&mut self, idx: u8) {
        self.interrupt_enable = self.interrupt_enable & !(1 << idx);
        let interrupt_flag = self.read_ram(INTERRUPT_FLAG);
        self.write_ram(INTERRUPT_FLAG, interrupt_flag & !(1 << idx));
    }

    fn increment_cpu_timer(&mut self, value: u32) {
        self.cpu_timer = self.cpu_timer.wrapping_add(value);
    }
}
