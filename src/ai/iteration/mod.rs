mod game_state;

use self::game_state::GameState;
use crate::nes::{cpu, mem};

use std::time::Instant;

const STUCK_TIMEOUT_S: u64 = 2;
const FINISH_TIMEOUT_S: u64 = 20;

#[derive(Default, Copy, Clone)]
pub struct Inputs {
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
}

#[derive(Eq, PartialEq)]
enum IterationState {
    Playing,
    Stuck,
    Dead,
    Succeeded,
}

pub struct Iteration {
    game_state: GameState,
    previous_game_state: GameState,
    state: IterationState,
    inputs: Inputs,
    start: Instant,
    last_x: u16,
    last_x_update: Instant,
}

impl Iteration {
    pub fn new() -> Iteration {
        Iteration {
            game_state: GameState::default(),
            previous_game_state: GameState::default(),
            state: IterationState::Playing,
            inputs: Inputs::default(),
            start: Instant::now(),
            last_x: 0,
            last_x_update: Instant::now(),
        }
    }

    fn update_state(&mut self) {
        let now = Instant::now();

        if self.game_state.lives < self.previous_game_state.lives {
            self.state = IterationState::Dead;
        } else if self.game_state.level > self.previous_game_state.level {
            self.state = IterationState::Succeeded;
        } else {
            let not_moving = self.game_state.mario_x == self.last_x
                && now.duration_since(self.last_x_update).as_secs() > STUCK_TIMEOUT_S;
            let took_too_long = now.duration_since(self.start).as_secs() > FINISH_TIMEOUT_S;
            if not_moving || took_too_long {
                self.state = IterationState::Stuck;
            }
        }

        if self.game_state.mario_x != self.last_x {
            self.last_x = self.game_state.mario_x;
            self.last_x_update = now;
        }
    }

    pub fn update_game_state(&mut self, mut cpu: &mut cpu::Cpu<mem::MemMap>) {
        self.previous_game_state = self.game_state;
        self.game_state = game_state::get_state(&mut cpu);

        self.update_state();
    }

    pub fn debug_game_state(&self) {
        if self.game_state != self.previous_game_state {
            println!("{:?}", self.game_state);
        }
    }

    pub fn is_stuck(&self) -> bool {
        self.state == IterationState::Stuck
    }

    pub fn is_dead(&self) -> bool {
        self.state == IterationState::Dead
    }

    pub fn get_inputs(&self) -> Inputs {
        self.inputs
    }
}
