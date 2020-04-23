extern crate mario_neural_network;

use mario_neural_network::{nes, start, EmulatorOptions};
use nes::gfx::Scale;
use nes::rom::Rom;

use std::fs::File;
use std::path::Path;

const ROM_PATH: &'static str = "super_mario.nes";
const SCALE: Scale = Scale::Scale3x;
const SAVE_STATE_PATH: &'static str = "state.sav";

fn main() {
    let rom = Rom::load(&mut File::open(&Path::new(ROM_PATH)).unwrap()).unwrap();
    start(EmulatorOptions { rom, scale: SCALE, save_state_path: SAVE_STATE_PATH });
}
