//
// Author: Patrick Walton
//

use super::mem::Mem;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::Sdl;

use std::ops::Deref;

//
// The "strobe state": the order in which the NES reads the buttons.
//

const STROBE_STATE_A: u8 = 0;
const STROBE_STATE_B: u8 = 1;
const STROBE_STATE_SELECT: u8 = 2;
const STROBE_STATE_START: u8 = 3;
const STROBE_STATE_UP: u8 = 4;
const STROBE_STATE_DOWN: u8 = 5;
const STROBE_STATE_LEFT: u8 = 6;
const STROBE_STATE_RIGHT: u8 = 7;

pub struct StrobeState {
    val: u8,
}

impl Deref for StrobeState {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.val
    }
}

impl StrobeState {
    // Given a GamepadState structure, returns the state of the given button.
    fn get(&self, state: &GamepadState) -> bool {
        match **self {
            STROBE_STATE_A => state.a,
            STROBE_STATE_B => state.b,
            STROBE_STATE_SELECT => state.select,
            STROBE_STATE_START => state.start,
            STROBE_STATE_UP => state.up,
            STROBE_STATE_DOWN => state.down,
            STROBE_STATE_LEFT => state.left,
            STROBE_STATE_RIGHT => state.right,
            _ => panic!("shouldn't happen"),
        }
    }

    fn next(&mut self) {
        *self = StrobeState {
            val: (**self + 1) & 7,
        };
    }

    fn reset(&mut self) {
        *self = StrobeState {
            val: STROBE_STATE_A,
        };
    }
}

//
// The standard NES game pad state
//

pub struct GamepadState {
    pub left: bool,
    pub down: bool,
    pub up: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,

    pub strobe_state: StrobeState,
}

pub struct Input {
    pub gamepad: GamepadState,
    sdl: Sdl, // FIXME: Use a `&'a mut EventPump` instead
}

impl Input {
    pub fn new(sdl: Sdl) -> Input {
        Input {
            gamepad: GamepadState {
                left: false,
                down: false,
                up: false,
                right: false,
                a: false,
                b: false,
                select: false,
                start: false,

                strobe_state: StrobeState {
                    val: STROBE_STATE_A,
                },
            },
            sdl: sdl,
        }
    }

    pub fn shutdown_requested(&mut self) -> bool {
        match self.sdl.event_pump().unwrap().poll_event() {
            Some(Event::Quit { .. })
            | Some(Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            }) => true,
            _ => false,
        }
    }
}

impl Mem for Input {
    fn loadb(&mut self, addr: u16) -> u8 {
        if addr == 0x4016 {
            let result = self.gamepad.strobe_state.get(&self.gamepad) as u8;
            self.gamepad.strobe_state.next();
            result
        } else {
            0
        }
    }

    fn storeb(&mut self, addr: u16, _: u8) {
        if addr == 0x4016 {
            // FIXME: This is not really accurate; you're supposed to not reset until you see
            // 1 strobed than 0. But I doubt this will break anything.
            self.gamepad.strobe_state.reset();
        }
    }
}
