pub mod game_state;

use self::game_state::GameState;
use super::{cpu, mem};

use std::time::{Duration, Instant};

#[derive(Eq, PartialEq)]
enum IterationState {
    Playing,
    Stuck,
    Dead,
    Succeeded,
}

struct Iteration {
    game_state: GameState,
    previous_game_state: GameState,
    state: IterationState,
    start: Instant,
}

impl Iteration {
    fn new() -> Iteration {
        Iteration {
            game_state: GameState {
                ..Default::default()
            },
            previous_game_state: GameState {
                ..Default::default()
            },
            state: IterationState::Playing,
            start: Instant::now(),
        }
    }

    fn update_game_state(&mut self, mut cpu: &mut cpu::Cpu<mem::MemMap>) {
        self.previous_game_state = self.game_state;
        self.game_state = game_state::get_state(&mut cpu);

        if self.game_state.lives < self.previous_game_state.lives {
            self.state = IterationState::Dead;
        } else if self.game_state.level > self.previous_game_state.level {
            self.state = IterationState::Succeeded;
        }
    }

    fn debug_game_state(&self) {
        if self.game_state != self.previous_game_state {
            println!("{:?}", self.game_state);
        }
    }

    // TODO: If AI does not move for X seconds then consider to be stuck
    fn is_stuck(&self) -> bool {
        self.state == IterationState::Stuck
    }

    fn is_dead(&self) -> bool {
        self.state == IterationState::Dead
    }
}

// TODO
enum InputEvent {
    Reset,
}

pub struct Ai {
    current_iteration: Iteration,
}

impl Ai {
    pub fn new() -> Ai {
        Ai {
            current_iteration: Iteration::new(),
        }
    }

    pub fn update_game_state(&mut self, mut cpu: &mut cpu::Cpu<mem::MemMap>) {
        self.current_iteration.update_game_state(&mut cpu)
    }

    pub fn debug_game_state(&self) {
        self.current_iteration.debug_game_state();
    }

    // TODO: Do stuff with new iteration parameters
    pub fn reset(&mut self) {
        self.current_iteration = Iteration::new();
    }

    pub fn is_stuck(&self) -> bool {
        self.current_iteration.is_stuck()
    }

    pub fn is_dead(&self) -> bool {
        self.current_iteration.is_dead()
    }

    // TODO: If stuck, output 'load state' event
    pub fn get_input(&self) -> Option<sdl2::event::Event> {
        None
    }
}
