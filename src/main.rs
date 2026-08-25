use std::fs;

use crate::gameboy::GameBoy;

mod constants;
mod gameboy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rom = fs::read("./roms/01-special.gb")?;
    let mut gameboy = GameBoy::new(rom);
    loop {
        gameboy.main();
    }
}
