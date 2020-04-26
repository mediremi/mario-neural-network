extern crate mario_neural_network;

use mario_neural_network::{ai, dashboard, nes, start, EmulatorOptions};

use ai::AiOptions;
use dashboard::DashboardOptions;
use nes::gfx::Scale;
use nes::rom::Rom;

use std::fs::File;
use std::path::Path;

// Emulator options
const ROM_PATH: &'static str = "super_mario.nes";
const SCALE: Scale = Scale::Scale3x;
const SAVE_STATE_PATH: &'static str = "state.sav";
const VSYNC: bool = true;

// AI options
const STUCK_TIMEOUT_S: u64 = 2;
const FINISH_TIMEOUT_S: u64 = 20;

// Dashboard options
const HOST: &'static str = "localhost";
const PORT: u64 = 8080;

fn main() {
    let rom = Rom::load(&mut File::open(&Path::new(ROM_PATH)).unwrap()).unwrap();
    start(
        EmulatorOptions {
            rom,
            scale: SCALE,
            save_state_path: SAVE_STATE_PATH,
            vsync: VSYNC,
        },
        AiOptions {
            stuck_timeout_s: STUCK_TIMEOUT_S,
            finish_timeout_s: FINISH_TIMEOUT_S,
        },
        DashboardOptions {
            host: HOST,
            port: PORT
        }
    );
}
