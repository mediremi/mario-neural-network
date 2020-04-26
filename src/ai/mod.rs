mod iteration;

use self::iteration::{Inputs, Iteration, IterationOptions};
use crate::nes::{cpu, mem};
use crate::utils::Screen;

pub struct AiOptions {
    pub stuck_timeout_s: u64,
    pub finish_timeout_s: u64,
}

pub struct Ai {
    iteration_options: IterationOptions,
    current_iteration: Iteration,
}

impl Ai {
    pub fn new(options: AiOptions) -> Ai {
        let iteration_options = IterationOptions {
            stuck_timeout_s: options.stuck_timeout_s,
            finish_timeout_s: options.finish_timeout_s,
        };
        Ai {
            iteration_options,
            current_iteration: Iteration::new(iteration_options),
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
        // TODO: Store `self.current_iteration.fitness()`
        self.current_iteration = Iteration::new(self.iteration_options);
    }

    pub fn is_stuck(&self) -> bool {
        self.current_iteration.is_stuck()
    }

    pub fn is_dead(&self) -> bool {
        self.current_iteration.is_dead()
    }

    pub fn has_succeeded(&self) -> bool {
        self.current_iteration.has_succeeded()
    }

    pub fn get_inputs(&self) -> Inputs {
        self.current_iteration.get_inputs()
    }

    pub fn get_screen(&self) -> Screen {
        self.current_iteration.get_screen()
    }
}
