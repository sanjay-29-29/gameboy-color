use crate::gameboy::GameBoy;

mod gameboy;

fn main() {
    let mut gameboy = GameBoy::new();

    loop {
        gameboy.main();
    }
}
