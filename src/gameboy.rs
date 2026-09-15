use std::{
    thread,
    time::{Duration, Instant},
};

use raylib::{
    RaylibHandle, RaylibThread,
    drawing::RaylibDraw,
    ffi::{Color, KeyboardKey},
};

use crate::{
    constants::*,
    gameboy::LCDState::{Mode2, Mode3},
};

#[derive(Debug, PartialEq)]
enum LCDState {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}

enum Interrupt {
    VBlank,
    LcdStat,
    Timer,
    Serial,
    Joypad,
}

enum LCDControlRegister {
    PPUEnable,
    WindowTileMap,
    WindowEnable,
    BGTileMap,
    BGWindowTileArea,
    ObjSize,
    ObjEnable,
    Priority,
}

#[derive(Debug)]
pub struct GameBoy {
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
    ei_executed: bool,             // Flag to show whether EI instruction is called
    interrupt_enable: u8,          // Interrupt Enable

    // Timer
    m_cycles: u64,
    instruction_m_cycle: u16,
    div_counter: u16,
    tima_overflowed: bool,
    tma_value: u8,
    tima_offset: u16,

    // Catridge
    catrigde_rom: [u8; 1024 * 1024],
    catridge_ram: [u8; 1024 * 1024],
    boot_rom: [u8; 256],
    catridge_selected_rom: u8,
    catridge_selected_ram: u8,
    external_ram_enabled: bool,
    boot_rom_disabled: bool,

    // HALT
    cpu_halted: bool,
    double_speed_mode: bool,

    // PPU
    is_vblank: bool,
    ppu_timer: u32,
    framebuffer: [u8; 23040],
    lcdstate: LCDState,

    //Joypad
    joypad: u8,
}

impl GameBoy {
    pub fn new(boot_rom: Vec<u8>, rom: Vec<u8>) -> Self {
        let mut gb = GameBoy {
            af: 0x0000,
            bc: 0x0000,
            de: 0x0000,
            hl: 0x0000,
            sp: 0x0000,
            pc: 0x0000,

            w_ram: [0; 32 * 1024],
            v_ram: [0; 16 * 1024],
            h_ram: [0; 128],
            oam: [0; 160],
            io_registers: [0; 128],

            catrigde_rom: [0; 1024 * 1024],
            catridge_ram: [0; 1024 * 1024],
            boot_rom: [0; 256],
            external_ram_enabled: false,
            catridge_selected_rom: 1,
            catridge_selected_ram: 0,
            boot_rom_disabled: false,

            interrupt_master_enable: false, // disabled when game starts running
            interrupt_enable: 0,
            ei_executed: false,

            m_cycles: 0,
            instruction_m_cycle: 0,
            div_counter: 0,
            tima_overflowed: false,
            tma_value: 0,
            tima_offset: 0,

            cpu_halted: false,
            double_speed_mode: false,

            is_vblank: false,
            ppu_timer: 0,
            framebuffer: [0; 23040],
            lcdstate: LCDState::Mode2,

            joypad: 0,
        };

        gb.load_rom(boot_rom, rom);
        gb.reset();

        gb
    }

    fn load_rom(&mut self, boot_rom: Vec<u8>, rom: Vec<u8>) {
        for i in 0..rom.len() {
            self.catrigde_rom[i] = rom[i];
        }

        for i in 0..boot_rom.len() {
            self.boot_rom[i] = boot_rom[i];
        }
    }

    fn reset(&mut self) {
        self.pc = 0;
        self.af = 0;
        self.bc = 0;
        self.de = 0;
        self.hl = 0;
        self.sp = 0;
    }

    fn write_ram(&mut self, addr: u16, val: u8) {
        self.increment_cpu_timer(1);

        let addr_usize = addr as usize;

        if addr == 0xFF02 {
            // If bit 7 (0x80) is set, a transfer is starting
            if val == 0x81 {
                let char_to_print = self.io_registers[1] as char;
                print!("{}", char_to_print);
            }
        }

        match addr {
            0x0000..=0x00ff => {
                if val & 0x0F == 0x0A && self.boot_rom_disabled {
                    self.external_ram_enabled = true;
                }
            }
            0x0100..=0x1fff => {
                // external RAM enabled by writing $A
                if val & 0x0F == 0x0A {
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
                self.catridge_selected_ram = 0b11 & val;
            }
            0x6000..=0x7fff => {
                // ROM
            }
            0x8000..=0x9fff => {
                // VRAM
                let mut base_addr: usize = 0;

                if self.io_registers[VBK_ADDR - 0xff00] & 1 == 1 {
                    base_addr = 0x2000;
                }

                self.v_ram[base_addr + (addr_usize - 0x8000)] = val;
            }
            0xa000..=0xbfff => {
                // 8 KiB External RAM
                self.catridge_ram
                    [(0x2000 * self.catridge_selected_ram as usize) + (addr_usize - 0xa000)] = val;
            }
            0xc000..=0xcfff => {
                // 4 KiB Work RAM (WRAM)
                // Bank 0
                self.w_ram[addr_usize - 0xc000] = val;
            }
            0xd000..=0xdfff => {
                // 4 KiB Work RAM (WRAM)
                // switchable bank 1–7
                let mut selected_bank =
                    (self.io_registers[WRAM_BANK_SELECT - 0xff00] & 0b111) as usize;

                if selected_bank == 0 {
                    selected_bank = 1; // 0 maps to Bank 1
                }

                self.w_ram[(0x1000 * selected_bank) + (addr_usize - 0xd000)] = val;
            }
            0xe000..=0xfdff => {
                // Echo RAM (mirror of C000–DDFF)
                self.w_ram[addr_usize - 0xe000] = val;
            }
            0xfe00..=0xfe9f => {
                // Object attribute memory (OAM)
                self.oam[addr_usize - 0xfe00] = val;
            }
            0xfea0..=0xfeff => {
                // return 0xFF;
            }
            0xff00..=0xff7f => {
                // I/O Registers

                match addr_usize {
                    DIV_REGISTER => {
                        self.io_registers[addr_usize - 0xff00] = 0;
                        self.div_counter = 0;
                    }
                    TAC_REGISTER => {
                        let tima = self.io_registers[TIMA_REGISTER - 0xff00];
                        let (sum, did_overflow) = tima.overflowing_add(1);

                        if did_overflow {
                            self.tima_overflowed = true;
                        }

                        self.io_registers[TIMA_REGISTER - 0xff00] = sum;
                        self.io_registers[TAC_REGISTER - 0xff00] = val
                    }
                    TMA_REGISTER => {
                        self.tma_value = val;
                    }
                    DMA => {
                        let (start_addr, end_addr): (usize, usize) =
                            ((val as usize) << 4, 0x9F | (val as usize) << 4);

                        for addr in start_addr..=end_addr {
                            self.oam[addr - start_addr] = self.read_ram(addr as u16);
                        }
                    }
                    BANK_REGISTER => {
                        self.boot_rom_disabled = true;
                    }
                    _ => self.io_registers[addr_usize - 0xff00] = val,
                }
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
            0x0000..=0x00ff => {
                if !self.boot_rom_disabled {
                    return self.boot_rom[addr_usize];
                } else {
                    return self.catrigde_rom[addr_usize];
                }
            }
            0x0100..=0x3fff => {
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

                if self.io_registers[VBK_ADDR - 0xff00] & 1 == 1 {
                    base_addr = 0x2000;
                }

                // println!("{}", self.io_registers[VBK_ADDR - 0xff00]);

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
                let mut selected_bank =
                    (self.io_registers[WRAM_BANK_SELECT - 0xff00] & 0b111) as usize;

                if selected_bank == 0 {
                    selected_bank = 1; // 0 maps to Bank 1
                }

                return self.w_ram[(0x1000 * selected_bank) + (addr_usize - 0xd000)];
            }
            0xe000..=0xfdff => {
                // Echo RAM (mirror of C000–DDFF)
                return self.w_ram[addr_usize - 0xe000];
            }
            0xfe00..=0xfe9f => {
                // Object attribute memory (OAM)
                return self.oam[addr_usize - 0xfe00];
            }
            0xfea0..=0xfeff => {
                return 0xFF;
            }
            0xff00..=0xff7f => {
                if addr_usize == JOYPAD {
                    match (self.io_registers[0] >> 4) & 0b11 {
                        0b00 => {}
                        0b01 => {
                            self.io_registers[0] |= !(self.joypad >> 4) & 0x0F;
                        }
                        0b10 => {
                            self.io_registers[0] |= !(self.joypad & 0x0F) & 0x0F;
                        }
                        0b11 => {
                            self.io_registers[0] |= 0x0F;
                        }
                        _ => {
                            unreachable!("")
                        }
                    }
                }
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
        let (mut rl, thread) = raylib::init().size(380, 288).title("Gameboy").build();

        let cpu_time = Duration::from_secs_f64(1.0 / 4096.0);

        let mut cpu = Instant::now();

        while !rl.window_should_close() {
            if self.cpu_halted {
                if self.io_registers[INTERRUPT_FLAG - 0xff00] & self.interrupt_enable & 0x1F > 0 {
                    self.cpu_halted = false;
                }
                self.increment_cpu_timer(1);
            }

            if !self.cpu_halted && !self.handle_interrupt() {
                self.fde();
            }

            self.update_timers();

            self.ppu(&mut rl, &thread);

            self.handle_input(&mut rl);

            self.io_registers[TMA_REGISTER - 0xff00] = self.tma_value;
            self.instruction_m_cycle = 0;

            if self.m_cycles >= 256 {
                let cpu_elapsed = cpu.elapsed();

                if cpu_time > cpu_elapsed {
                    thread::sleep(cpu_time - cpu_elapsed);
                }

                cpu = Instant::now();
                self.m_cycles -= 256;
            }
        }
    }

    pub fn handle_input(&mut self, rl: &mut RaylibHandle) {
        self.joypad = 0;

        if rl.is_key_down(KeyboardKey::KEY_W) {
            self.joypad = self.joypad | 1 << 2;
        }
        if rl.is_key_down(KeyboardKey::KEY_A) {
            self.joypad = self.joypad | 1 << 1;
        }
        if rl.is_key_down(KeyboardKey::KEY_S) {
            self.joypad = self.joypad | 1;
        }
        if rl.is_key_down(KeyboardKey::KEY_D) {
            self.joypad = self.joypad | 1 << 3;
        }
        if rl.is_key_down(KeyboardKey::KEY_ENTER) {
            self.joypad = self.joypad | 1 << 6;
        }
        if rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT) {
            self.joypad = self.joypad | 1 << 7;
        }
        if rl.is_key_down(KeyboardKey::KEY_J) {
            self.joypad = self.joypad | 1 << 5;
        }
        if rl.is_key_down(KeyboardKey::KEY_K) {
            self.joypad = self.joypad | 1 << 4;
        }
    }

    pub fn handle_interrupt(&mut self) -> bool {
        if self.ei_executed {
            // The effect of ei is delayed by one instruction
            self.ei_executed = false;
            self.interrupt_master_enable = true;
            return false;
        }

        if !self.interrupt_master_enable {
            return false;
        }

        for i in 0..=4 {
            if (self.interrupt_enable >> i) & (self.io_registers[INTERRUPT_FLAG - 0xff00] >> i) & 1
                == 0
            {
                continue;
            }

            self.increment_cpu_timer(3);
            self.clear_interrupt(i);
            self.push_to_stack(self.pc);

            let addr = match i {
                0 => 0x0040, // VBlank
                1 => 0x0048, // LCD STAT
                2 => 0x0050, // Timer
                3 => 0x0058, // Serial
                4 => 0x0060, // Joypad
                _ => panic!("Invalid Interrupt {i}"),
            };

            self.pc = addr;

            return true;
        }

        false
    }

    fn update_timers(&mut self) {
        let t_cycles = self.instruction_m_cycle * 4;

        self.m_cycles = self.m_cycles.wrapping_add(self.instruction_m_cycle as u64);
        self.ppu_timer = self
            .ppu_timer
            .wrapping_add(self.instruction_m_cycle as u32 * 4);

        self.div_counter = self.div_counter.wrapping_add(t_cycles);
        self.io_registers[DIV_REGISTER - 0xff00] = (self.div_counter >> 8) as u8;

        if self.tima_overflowed {
            self.io_registers[TIMA_REGISTER - 0xff00] = self.io_registers[TMA_REGISTER - 0xff00];
            self.request_interrupt(Interrupt::Timer);
            self.tima_overflowed = false;
        }

        self.tima_offset = self.tima_offset.wrapping_add(self.instruction_m_cycle);

        let tac_register = self.io_registers[TAC_REGISTER - 0xff00];

        if (tac_register & 0b100) >> 2 == 0 {
            return;
        }

        let speed = match tac_register & 0b11 {
            0 => 256,
            1 => 4,
            2 => 16,
            3 => 64,
            _ => {
                unreachable!("Error occurred while updating timers: TAC Register: {tac_register}")
            }
        };

        while self.tima_offset >= speed {
            let timer_counter = self.io_registers[TIMA_REGISTER - 0xff00];
            let (sum, did_overflow) = timer_counter.overflowing_add(1);

            if did_overflow {
                self.tima_overflowed = true;
            }

            self.io_registers[TIMA_REGISTER - 0xff00] = sum;
            self.tima_offset -= speed;
        }
    }

    fn fde(&mut self) {
        let opcode = self.fetch_value_u8();
        let (x, y, z) = (opcode >> 6, (opcode >> 3) & 0x07, opcode & 0x07);

        // println!("{:x} {:x}", opcode, self.pc);
        // thread::sleep(Duration::from_millis(1));

        match x {
            0 => {
                if z == 0 {
                    if y == 0 {
                        // nop
                    }
                    if y == 2 {
                        // TODO: stop
                        println!("Stop executed.");
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
                    let register = self.get_r16_mut(y);

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
                    let register = self.get_r16_mut(y);
                    let sum = (*register).wrapping_add(1);

                    *register = sum;

                    self.increment_cpu_timer(1);
                }
                if z == 3 && (y & 0b001) == 1 {
                    // dec r16
                    let register = self.get_r16_mut(y);
                    let sum = (*register).wrapping_sub(1);

                    *register = sum;

                    self.increment_cpu_timer(1);
                }
                if z == 1 && (y & 0b001) == 1 {
                    // add hl, r16
                    let register_val = *self.get_r16_mut(y);
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
                        let register = self.get_r16stk_mut(y);
                        *register = val;
                    }
                }
                if z == 5 && (y & 0b001) == 0 {
                    // push r16stk
                    let mut register = *self.get_r16stk_mut(y);

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
                    self.ei_executed = true;
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

    fn get_r16_mut(&mut self, y: u8) -> &mut u16 {
        let res = match y >> 1 {
            0x0 => &mut self.bc,
            0x1 => &mut self.de,
            0x2 => &mut self.hl,
            0x3 => &mut self.sp,
            _ => panic!("Not supported"),
        };

        res
    }

    fn get_r16stk_mut(&mut self, y: u8) -> &mut u16 {
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

    fn get_scy_mut(&mut self) -> &mut u8 {
        return &mut self.io_registers[0xFF42 - 0xFF00];
    }

    fn get_scx_mut(&mut self) -> &mut u8 {
        return &mut self.io_registers[0xFF43 - 0xFF00];
    }

    fn request_interrupt(&mut self, interrupt: Interrupt) {
        let bit = match interrupt {
            Interrupt::VBlank => 0,
            Interrupt::LcdStat => 1,
            Interrupt::Timer => 2,
            Interrupt::Serial => 3,
            Interrupt::Joypad => 4,
        };

        self.io_registers[INTERRUPT_FLAG - 0xff00] =
            self.io_registers[INTERRUPT_FLAG - 0xff00] | (1 << bit);
    }

    fn clear_interrupt(&mut self, idx: u8) {
        let interrupt_flag = self.io_registers[INTERRUPT_FLAG - 0xff00];
        self.io_registers[INTERRUPT_FLAG - 0xff00] = interrupt_flag & !(1 << idx);
        self.interrupt_master_enable = false;
    }

    fn increment_cpu_timer(&mut self, value: u16) {
        self.instruction_m_cycle = self.instruction_m_cycle.wrapping_add(value);
    }

    fn get_speed_reg_mut(&mut self) -> &mut u8 {
        &mut self.io_registers[SPD - 0xFF00]
    }

    fn read_vram(&self, addr: u16) -> u8 {
        let mut base_addr: usize = 0;

        if self.io_registers[VBK_ADDR - 0xff00] & 1 == 1 {
            base_addr = 0x2000;
        }

        return self.v_ram[base_addr + (addr as usize - 0x8000)];
    }

    fn get_lcd_control_register(&self, register: LCDControlRegister) -> bool {
        let register_val = self.io_registers[LCDC_REGISTER - 0xFF00];

        let val = match register {
            LCDControlRegister::PPUEnable => register_val >> 7,
            LCDControlRegister::WindowTileMap => register_val >> 6,
            LCDControlRegister::WindowEnable => register_val >> 5,
            LCDControlRegister::BGWindowTileArea => register_val >> 4,
            LCDControlRegister::BGTileMap => register_val >> 3,
            LCDControlRegister::ObjSize => register_val >> 2,
            LCDControlRegister::ObjEnable => register_val >> 1,
            LCDControlRegister::Priority => register_val,
        };

        val & 1 == 1
    }

    fn get_stat_reg_mut(&mut self) -> &mut u8 {
        return &mut self.io_registers[STAT - 0xFF00];
    }

    fn get_lc_reg_mut(&mut self) -> &mut u8 {
        return &mut self.io_registers[LY - 0xFF00];
    }

    fn ppu(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        // println!("{:?}", self.lcdstate);

        match self.lcdstate {
            LCDState::Mode1 => {
                if self.ppu_timer > 456 {
                    let sy = self.get_lc_reg_mut();
                    *sy = (*sy + 1) % 154;

                    if *sy == 0 {
                        self.lcdstate = LCDState::Mode2;
                    }

                    self.ppu_timer -= 456;
                }
            }
            LCDState::Mode2 => {
                let screen_y = *self.get_lc_reg_mut() as u16;

                if screen_y == 144 {
                    self.request_interrupt(Interrupt::VBlank);
                    self.lcdstate = LCDState::Mode1;
                } else if self.ppu_timer > 80 {
                    self.ppu_timer -= 80;
                    self.lcdstate = LCDState::Mode3;
                }
            }
            LCDState::Mode0 => {
                if self.ppu_timer > 204 {
                    self.ppu_timer -= 204;
                    self.lcdstate = LCDState::Mode2;
                    let sy = self.get_lc_reg_mut();
                    *sy = (*sy + 1) % 154;
                }
            }
            LCDState::Mode3 => {
                if self.ppu_timer < 172 {
                    return;
                }

                self.draw(rl, thread);

                self.ppu_timer -= 172;
                self.lcdstate = LCDState::Mode0;
            }
        }

        // if self.ppu_timer > 0 && self.lcdstate == LCDState::Mode2 {
        //     let stat_mut = self.get_stat_reg_mut();
        //     *stat_mut |= 1 << 5;
        //     // self.request_interrupt(Interrupt::LcdStat);
        //     self.lcdstate = LCDState::Mode0;
        // }

        // if self.ppu_timer > 252 && self.lcdstate == LCDState::Mode0 {
        //     let stat_mut = self.get_stat_reg_mut();
        //     *stat_mut |= 1 << 3;
        //     // self.request_interrupt(Interrupt::LcdStat);
        //     self.lcdstate = LCDState::Mode0;
        // }
    }

    fn draw(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let mut d = rl.begin_drawing(&thread);
        let screen_y = *self.get_lc_reg_mut() as u16;

        for screen_x in 0..160_u16 {
            let (scx, scy) = (*self.get_scx_mut(), *self.get_scy_mut());

            let tile_map_base_addr =
                match self.get_lcd_control_register(LCDControlRegister::BGTileMap) {
                    false => 0x9800,
                    true => 0x9C00,
                };

            let a = (scx as u16 + screen_x) % 256;
            let b = (scy as u16 + screen_y) % 256;

            let tile_map = self.read_vram(tile_map_base_addr + (b / 8) * 32 + (a / 8));

            let tile_data_addr: usize =
                match self.get_lcd_control_register(LCDControlRegister::BGWindowTileArea) {
                    true => 0x8000 + (tile_map as usize * 16),
                    false => (0x9000 + (tile_map as i8 as i32 * 16)) as usize,
                };

            let (tile_row, tile_col) = (screen_y % 8, screen_x % 8);

            let byte1 = self.read_vram(tile_data_addr as u16 + tile_row * 2);
            let byte2 = self.read_vram(tile_data_addr as u16 + (tile_row * 2) + 1);

            let color_value = ((byte2 >> (7 - tile_col)) & 1) << 1 | (byte1 >> (7 - tile_col)) & 1;

            match color_value {
                0 => d.draw_rectangle(screen_x as i32 * 2, screen_y as i32 * 2, 2, 2, Color::WHITE),
                1 => d.draw_rectangle(
                    screen_x as i32 * 2,
                    screen_y as i32 * 2,
                    2,
                    2,
                    Color::LIGHTGRAY,
                ),
                2 => d.draw_rectangle(screen_x as i32 * 2, screen_y as i32 * 2, 2, 2, Color::GRAY),
                3 => d.draw_rectangle(screen_x as i32 * 2, screen_y as i32 * 2, 2, 2, Color::BLACK),
                _ => unreachable!("{color_value}"),
            }
        }

        if self.get_lcd_control_register(LCDControlRegister::ObjEnable) {
            return;
        }

        for object_idx in 0..40 {
            let (y_pos, x_pos, tile_idx, flags) = (
                self.oam[object_idx + 0] as i32 - 16,
                self.oam[object_idx + 1] as i32 - 8,
                self.oam[object_idx + 2],
                self.oam[object_idx + 3],
            );

            if (flags >> 7) & 1 == 1 {
                return;
            }

            let obj_size = match self.get_lcd_control_register(LCDControlRegister::ObjSize) {
                true => 16,
                false => 8,
            };

            for y in 0..obj_size {
                let byte1 = self.read_vram(0x8000 + (tile_idx as u16 * obj_size) + y * 2);
                let byte2 = self.read_vram(0x8000 + (tile_idx as u16 * obj_size) + (y * 2) + 1);

                for x in 0..8 {
                    if x as i32 + x_pos < 0 {
                        continue;
                    }
                    if y as i32 + y_pos < 0 {
                        continue;
                    }

                    let color_value = ((byte2 >> (7 - x)) & 1) << 1 | (byte1 >> (7 - x)) & 1;

                    match color_value {
                        0 => d.draw_rectangle(
                            x_pos + x as i32 * 2,
                            y_pos + y as i32 * 2,
                            2,
                            2,
                            Color::WHITE,
                        ),
                        1 => d.draw_rectangle(
                            x_pos + x as i32 * 2,
                            y_pos + y as i32 * 2,
                            2,
                            2,
                            Color::LIGHTGRAY,
                        ),
                        2 => d.draw_rectangle(
                            x_pos + x as i32 * 2,
                            y_pos + y as i32 * 2,
                            2,
                            2,
                            Color::GRAY,
                        ),
                        3 => d.draw_rectangle(
                            x_pos + x as i32 * 2,
                            y_pos + y as i32 * 2,
                            2,
                            2,
                            Color::BLACK,
                        ),
                        _ => unreachable!("{color_value}"),
                    }
                }
            }
        }
    }
}
