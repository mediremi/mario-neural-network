//
// Author: Patrick Walton
//

extern crate libc;
extern crate sdl2;

// NB: This must be first to pick up the macro definitions. What a botch.
#[macro_use]
pub mod util;

#[macro_use]
pub mod cpu;
pub mod ai;
pub mod disasm;
pub mod gfx;
pub mod input;
pub mod mapper;
pub mod mem;
pub mod ppu;
pub mod rom;

use ai::Ai;
use cpu::Cpu;
use gfx::{Gfx, Scale};
use input::{Input, InputResult};
use mapper::Mapper;
use mem::MemMap;
use ppu::{Oam, Ppu, Vram};
use rom::Rom;
use util::Save;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

/// Starts the emulator main loop with a ROM and window scaling. Returns when the user presses ESC.
pub fn start_emulator(rom: Rom, scale: Scale) {
    let ai = Rc::new(RefCell::new(Ai::new()));

    let rom = Box::new(rom);

    let (mut gfx, sdl) = Gfx::new(scale);

    let mapper: Box<dyn Mapper + Send> = mapper::create_mapper(rom);
    let mapper = Rc::new(RefCell::new(mapper));
    let ppu = Ppu::new(Vram::new(mapper.clone()), Oam::new());
    let input = Input::new(sdl, ai.clone());
    let memmap = MemMap::new(ppu, input, mapper);
    let mut cpu = Cpu::new(memmap);

    cpu.reset();

    cpu.load(&mut File::open(&Path::new("state.sav")).unwrap());

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

            {
                let mut ai = ai.borrow_mut();

                if ai.is_stuck() || ai.is_dead() {
                    cpu.load(&mut File::open(&Path::new("state.sav")).unwrap());
                    gfx.status_line.set("AI was stuck/died so reset".to_string());
                    ai.reset();
                    continue;
                }

                ai.update_game_state(&mut cpu);
                ai.debug_game_state();
            }

            if cpu.mem.input.check_input() == InputResult::Quit {
                break;
            }
        }
    }
}
