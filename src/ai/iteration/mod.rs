mod game_state;
mod neural_network;

use self::game_state::GameState;
use crate::nes::{cpu, mem};

use std::time::Instant;

#[derive(Default, Copy, Clone)]
pub struct Inputs {
    pub right: bool,
    pub a: bool,
}

#[derive(Eq, PartialEq)]
enum IterationState {
    Playing,
    Stuck,
    Dead,
    Succeeded,
}

#[derive(Copy, Clone)]
pub struct IterationOptions {
    pub stuck_timeout_s: u64,
    pub finish_timeout_s: u64,
}

pub struct Iteration {
    game_state: GameState,
    previous_game_state: GameState,
    state: IterationState,
    start: Instant,
    last_x: u16,
    last_x_update: Instant,

    stuck_timeout_s: u64,
    finish_timeout_s: u64,
}

impl Iteration {
    pub fn new(options: IterationOptions) -> Iteration {
        Iteration {
            game_state: GameState::default(),
            previous_game_state: GameState::default(),
            state: IterationState::Playing,
            start: Instant::now(),
            last_x: 0,
            last_x_update: Instant::now(),
            stuck_timeout_s: options.stuck_timeout_s,
            finish_timeout_s: options.finish_timeout_s,
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
                && now.duration_since(self.last_x_update).as_secs() > self.stuck_timeout_s;
            let took_too_long = now.duration_since(self.start).as_secs() > self.finish_timeout_s;
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

    pub fn has_succeeded(&self) -> bool {
        self.state == IterationState::Succeeded
    }

    // TODO
    fn run_neural_network(&self) -> (f64, f64) {
        (1.0, 0.0)
    }

    pub fn get_inputs(&self) -> Inputs {
        const THRESHOLD: f64 = 0.5;
        let (right_value, a_value) = self.run_neural_network();
        Inputs {
            right: right_value > THRESHOLD,
            a: a_value > THRESHOLD,
        }
    }

    pub fn fitness(&self) -> u64 {
        let success_bonus = if self.state == IterationState::Succeeded {
            1000
        } else {
            0
        };
        (self.game_state.mario_x as u64 + success_bonus) / self.start.elapsed().as_secs()
    }
}
