use raylib::{
    drawing::{RaylibDraw, RaylibDrawHandle},
    ffi::Color,
};

use crate::gameboy::GameBoy;

pub struct PPU;

impl PPU {
    pub fn draw(gb: &GameBoy, d: &mut RaylibDrawHandle) {
        d.clear_background(Color::WHITE);
    }
}
