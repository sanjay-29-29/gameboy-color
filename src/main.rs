use std::fs;

use crate::gameboy::GameBoy;

mod constants;
mod display;
mod gameboy;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let rom_path = args.get(1).expect("A ROM path is required.");
    let rom = fs::read(rom_path).expect("Invalid ROM path");

    let mut gameboy = GameBoy::new(rom);

    gameboy.main();
}
