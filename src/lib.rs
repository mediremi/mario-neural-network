//
// Author: Patrick Walton
//

extern crate libc;
extern crate sdl2;

pub mod ai;
pub mod nes;

use ai::Ai;
use nes::cpu::Cpu;
use nes::gfx::{Gfx, Scale};
use nes::input::{Input, GamepadState};
use nes::mapper::{Mapper, create_mapper};
use nes::mem::MemMap;
use nes::ppu::{Oam, Ppu, Vram};
use nes::rom::Rom;
use nes::util::Save;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

pub struct EmulatorOptions {
    pub rom: Rom,
    pub scale: Scale,
    pub save_state_path: &'static str
}

pub fn start(emulator_options: EmulatorOptions) {
    let rom = Box::new(emulator_options.rom);

    let (mut gfx, sdl) = Gfx::new(emulator_options.scale);

    let mapper: Box<dyn Mapper + Send> = create_mapper(rom);
    let mapper = Rc::new(RefCell::new(mapper));
    let ppu = Ppu::new(Vram::new(mapper.clone()), Oam::new());
    let input = Input::new(sdl);
    let memmap = MemMap::new(ppu, input, mapper);
    let mut cpu = Cpu::new(memmap);

    cpu.reset();

    cpu.load(&mut File::open(&Path::new(emulator_options.save_state_path)).unwrap());

    let mut ai = Ai::new();

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

            if ai.is_stuck() || ai.is_dead() {
                cpu.load(&mut File::open(&Path::new(emulator_options.save_state_path)).unwrap());
                gfx.status_line.set("AI was stuck/died so reset".to_string());
                ai.reset();
                continue;
            }

            if ai.has_succeeded() {
                println!("AI succeeded");
                break;
            }

            ai.update_game_state(&mut cpu);
            ai.debug_game_state();

            let ai_inputs = ai.get_inputs();
            cpu.mem.input.gamepad = GamepadState {
                left: ai_inputs.left,
                right: ai_inputs.right,
                a: ai_inputs.a,
                b: ai_inputs.b,
                ..cpu.mem.input.gamepad
            };

            if cpu.mem.input.shutdown_requested() {
                break;
            }
        }
    }
}
