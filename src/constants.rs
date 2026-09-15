pub const VBK_ADDR: usize = 0xFF4F;
pub const WRAM_BANK_SELECT: usize = 0xFF70;
pub const DIV_REGISTER: usize = 0xFF04;
pub const TAC_REGISTER: usize = 0xFF07;
pub const TIMA_REGISTER: usize = 0xFF05;
pub const TMA_REGISTER: usize = 0xFF06;
pub const LCDC_REGISTER: usize = 0xFF40;
pub const CLOCK_SPEED: u32 = 8_388_608;
pub const INTERRUPT_FLAG: usize = 0xFF0F;
pub const BANK_REGISTER: usize = 0xFF50;

pub const WINDOW_X: usize = 0xFF4A;
pub const WINDOW_Y: usize = 0xFF4B;

pub const LCDC: usize = 0xFF40; // LCD Control
pub const LY: usize = 0xFF44; // LCD Y coordinate [read-only]
pub const LYC: usize = 0xFF45; // LY compare
pub const STAT: usize = 0xFF41; // LCD status
pub const SCY: usize = 0xFF42; // Scroll Y
pub const SCX: usize = 0xFF43; // Scroll X
pub const SPD: usize = 0xFF4D; // Prepare speed switch
pub const DMA: usize = 0xFF46; // OAM DMA source address & start
pub const JOYPAD: usize = 0xFF00; // Joypad
