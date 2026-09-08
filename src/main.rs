use std::fs;

use crate::gameboy::GameBoy;

mod constants;
mod display;
mod gameboy;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let rom_path = args.get(1).expect("A ROM path is required.");
    let boot_rom_path = args.get(2).expect("A Boot ROM path is required.");

    let rom = fs::read(rom_path).expect("Invalid ROM path");
    let boot_rom = fs::read(boot_rom_path).expect("Invalid Boot ROM path");

    let mut gameboy = GameBoy::new(boot_rom, rom);

    gameboy.main();
}
