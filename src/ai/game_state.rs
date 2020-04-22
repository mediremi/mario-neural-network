use super::super::{cpu, mem};

use std::fmt;

#[derive(Eq, PartialEq, Default, Clone, Copy)]
pub struct GameState {
    pub mario_x: u16,
    pub mario_y: u16,
    pub screen_x: u8,
    pub lives: u8,
    pub level: u8,
}

impl fmt::Debug for GameState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_fmt(format_args!(
            "Mario coords: ({}, {}). Lives: {}. Screen X: {}. Level: {}",
            self.mario_x, self.mario_y, self.lives, self.screen_x, self.level
        ))
    }
}

// Source: https://datacrystal.romhacking.net/wiki/Super_Mario_Bros.:RAM_map
pub fn get_state(cpu: &mut cpu::Cpu<mem::MemMap>) -> GameState {
    let mario_x = {
        let mario_level_x = cpu.loadb(0x6D) as u16;
        let mario_screen_x = cpu.loadb(0x86) as u16;
        mario_level_x * 0x100 + mario_screen_x
    };
    let mario_y = {
        let mario_screen_y = cpu.loadb(0x3B8) as u16;
        mario_screen_y + 16
    };
    let screen_x = cpu.loadb(0x3AD);
    let lives = cpu.loadb(0x75A);
    let level = cpu.loadb(0x760);
    GameState {
        mario_x,
        mario_y,
        screen_x,
        lives,
        level,
    }
}
