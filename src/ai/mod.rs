mod iteration;

use self::iteration::{Iteration, Inputs};
use crate::nes::{cpu, mem};

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

    pub fn get_inputs(&self) -> Inputs {
        self.current_iteration.get_inputs()
    }

    pub fn has_succeeded(&self) -> bool {
        unimplemented!()
    }
}
