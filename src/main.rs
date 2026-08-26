use std::fs;

use crate::gameboy::GameBoy;

mod constants;
mod gameboy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let rom_path = args.get(1).expect("A ROM path is required.");
    let rom = fs::read(rom_path)?;

    let mut gameboy = GameBoy::new(rom);

    loop {
        gameboy.main();
    }
}
