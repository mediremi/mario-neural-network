use crate::nes::{cpu, mem};
use crate::utils::{Screen, Tile};

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

// Source for memory addresses: https://datacrystal.romhacking.net/wiki/Super_Mario_Bros.:RAM_map
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

pub fn get_screen(cpu: &mut cpu::Cpu<mem::MemMap>, game_state: GameState) -> Screen {
    fn get_tile(x: i32, y: i32, cpu: &mut cpu::Cpu<mem::MemMap>) -> Tile {
        let sub_y = (y - 32) / 16;
        if sub_y >= 13 || sub_y < 0 {
            return Tile::Nothing;
        }
        let sub_x = (x % 256) / 16;
        let page = (x / 256) % 2;
        let addr = 0x500 + (page * 13 * 16) + (sub_y * 16) + sub_x;
        if cpu.loadb(addr as u16) == 0 {
            Tile::Nothing
        } else {
            Tile::Block
        }
    }

    fn get_enemies(cpu: &mut cpu::Cpu<mem::MemMap>) -> Vec<(u16, u16)> {
        let mut enemies = vec![];
        for slot in 0..=4 {
            let enemy = cpu.loadb(0xF + slot);
            if enemy != 0 {
                let e_x = (cpu.loadb(0x6E + slot) as u16 * 0x100) + cpu.loadb(0x87 + slot) as u16;
                let e_y = cpu.loadb(0xCF + slot) as u16 + 24;
                enemies.push((e_x, e_y));
            }
        }
        enemies
    }

    // How many blocks NN sees to left, right, top and bottom of Mario
    const VIEW_SIZE: i32 = 6;
    const BLOCK_SIZE: i32 = 16;
    let mut screen = [[Tile::Nothing; 13]; 13];
    for i in -VIEW_SIZE..=VIEW_SIZE {
        let dy = i * BLOCK_SIZE;
        let y = game_state.mario_y as i32 + dy - BLOCK_SIZE;

        for j in -VIEW_SIZE..=VIEW_SIZE {
            let dx = j * BLOCK_SIZE;
            let x = game_state.mario_x as i32 + dx + 8;
            screen[(i + VIEW_SIZE) as usize][(j + VIEW_SIZE) as usize] = get_tile(x, y, cpu)
        }
    }
    screen[VIEW_SIZE as usize + 1][VIEW_SIZE as usize] = Tile::Mario;

    for (e_x, e_y) in get_enemies(cpu) {
        let i = (e_y as i32 - game_state.mario_y as i32) / BLOCK_SIZE + VIEW_SIZE;
        let j = (e_x as i32 - game_state.mario_x as i32) / BLOCK_SIZE + VIEW_SIZE;
        if (0 <= i && i < 13) && (0 <= j && j < 13) {
            screen[i as usize][j as usize] = Tile::Enemy;
        }
    }

    screen
}
