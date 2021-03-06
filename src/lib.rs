//
// Author: Patrick Walton
//

extern crate crossbeam;
extern crate libc;
extern crate sdl2;
extern crate simple_server;
extern crate tungstenite;
extern crate rand;
extern crate serde;
extern crate serde_json;

pub mod ai;
pub mod dashboard;
pub mod nes;
mod utils;

use ai::{Ai, AiOptions};
use dashboard::{Dashboard, DashboardOptions};
use nes::cpu::Cpu;
use nes::gfx::{Gfx, GfxOptions, Scale};
use nes::input::Input;
use nes::mapper::{create_mapper, Mapper};
use nes::mem::MemMap;
use nes::ppu::{Oam, Ppu, Vram};
use nes::rom::Rom;
use nes::util::Save;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub struct EmulatorOptions {
    pub rom: Rom,
    pub scale: Scale,
    pub save_state_path: &'static str,
    pub vsync: bool,
}

fn init_emulator(options: EmulatorOptions) -> (Cpu<MemMap>, Gfx) {
    let rom = Box::new(options.rom);

    let (gfx, sdl) = Gfx::new(GfxOptions {
        scale: options.scale,
        vsync: options.vsync,
    });

    let mapper: Box<dyn Mapper + Send> = create_mapper(rom);
    let mapper = Rc::new(RefCell::new(mapper));
    let ppu = Ppu::new(Vram::new(mapper.clone()), Oam::new());
    let input = Input::new(sdl);
    let memmap = MemMap::new(ppu, input, mapper);
    let mut cpu = Cpu::new(memmap);

    cpu.reset();

    cpu.load(&mut File::open(&Path::new(options.save_state_path)).unwrap());

    (cpu, gfx)
}

pub fn start(
    emulator_options: EmulatorOptions,
    ai_options: AiOptions,
    dashboard_options: DashboardOptions,
) {
    let save_state_path = emulator_options.save_state_path;
    let (mut cpu, mut gfx) = init_emulator(emulator_options);
    let mut ai = Ai::new(ai_options);
    // ai.load_snapshot("snapshots/g-1.json");
    let dashboard = Dashboard::new(dashboard_options);

    let mut last_dashboard_update = Instant::now();
    let dashboard_update_interval = Duration::from_millis(30);

    loop {
        cpu.step();

        let ppu_result = cpu.mem.ppu.step(cpu.cy);
        if ppu_result.vblank_nmi {
            cpu.nmi();
        } else if ppu_result.scanline_irq {
            cpu.irq();
        }

        if ppu_result.new_frame {
            gfx.tick();
            gfx.composite(&mut *cpu.mem.ppu.screen);

            if ai.has_succeeded() {
                println!("AI succeeded");
                // TODO: Save successful neural network
                break;
            } else if ai.is_stuck() || ai.is_dead() {
                cpu.load(&mut File::open(&Path::new(save_state_path)).unwrap());
                let reason = if ai.is_stuck() { "was stuck" } else { "died" };
                let msg = format!("Reset because AI {}", reason).to_string();
                println!("{}", msg);
                gfx.status_line.set(msg);
                ai.next_individual();
                continue;
            }

            ai.update_game_state(&mut cpu);
            // ai.debug_game_state();

            let ai_inputs = ai.get_inputs();
            cpu.mem.input.gamepad.right = ai_inputs.right;
            cpu.mem.input.gamepad.a = ai_inputs.a;

            if last_dashboard_update.elapsed() > dashboard_update_interval {
                dashboard.update_screen(ai.get_screen());
                last_dashboard_update = Instant::now();
            }

            if cpu.mem.input.shutdown_requested() {
                break;
            }
        }
    }
}
