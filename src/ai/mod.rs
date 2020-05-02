mod game_state;

use self::game_state::GameState;
use crate::nes::{cpu, mem};
use crate::utils::{Screen, Tile, SCREEN_SIZE};

use rand::distributions::{Distribution, Uniform};

use std::cmp::Reverse;
use std::time::Instant;

const DESIRED_POPULATION: i64 = 300;
// Maximum number of species in pool before weaker species are removed
const MAX_SPECIES: usize = 30;
// Maximum number of generations that a species can exist for without
// improving its performance
const MAX_SPECIES_STALENESS: u64 = 15;

impl Tile {
    fn as_nn_input(self) -> f64 {
        use self::Tile::*;

        match self {
            Nothing | Mario => 0.0,
            Block => 1.0,
            Enemy => -1.0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Inputs {
    pub right: bool,
    pub a: bool,
}

pub struct AiOptions {
    pub stuck_timeout_s: u64,
    pub finish_timeout_s: u64,
}

// TODO: Find better name for this
#[derive(Eq, PartialEq)]
enum XState {
    Playing,
    Stuck,
    Dead,
    Succeeded,
}

struct IndividualStateOptions {
    stuck_timeout_s: u64,
    finish_timeout_s: u64,
}

struct IndividualState {
    game_state: GameState,
    previous_game_state: GameState,
    screen: Screen,
    state: XState,
    start: Instant,
    last_x: u16,
    last_x_update: Instant,
    stuck_timeout_s: u64,
    finish_timeout_s: u64,
}

impl IndividualState {
    pub fn new(options: IndividualStateOptions) -> Self {
        Self {
            game_state: GameState::default(),
            previous_game_state: GameState::default(),
            screen: Screen::default(),
            state: XState::Playing,
            start: Instant::now(),
            last_x: 0,
            last_x_update: Instant::now(),
            stuck_timeout_s: options.stuck_timeout_s,
            finish_timeout_s: options.finish_timeout_s,
        }
    }

    fn update_state(&mut self) {
        use self::XState::*;

        let not_moving = self.game_state.mario_x == self.last_x
            && self.last_x_update.elapsed().as_secs() > self.stuck_timeout_s;
        let took_too_long = self.start.elapsed().as_secs() > self.finish_timeout_s;

        if self.game_state.lives < self.previous_game_state.lives {
            self.state = Dead;
        } else if self.game_state.level > self.previous_game_state.level {
            self.state = Succeeded;
        } else if not_moving || took_too_long {
            self.state = Stuck;
        } else {
            self.last_x = self.game_state.mario_x;
            self.last_x_update = Instant::now();
        }
    }

    pub fn update(&mut self, mut cpu: &mut cpu::Cpu<mem::MemMap>) {
        self.previous_game_state = self.game_state;
        self.game_state = game_state::get_state(&mut cpu);
        self.screen = game_state::get_screen(&mut cpu, self.game_state);

        self.update_state();
    }

    pub fn debug_game_state(&self) {
        if self.game_state != self.previous_game_state {
            println!("{:?}", self.game_state);
        }
    }

    pub fn is_stuck(&self) -> bool {
        self.state == XState::Stuck
    }

    pub fn is_dead(&self) -> bool {
        self.state == XState::Dead
    }

    pub fn has_succeeded(&self) -> bool {
        self.state == XState::Succeeded
    }

    pub fn get_screen(&self) -> Screen {
        self.screen
    }

    pub fn fitness(&self) -> u64 {
        let success_bonus = if self.has_succeeded() { 1000 } else { 0 };
        (self.game_state.mario_x as u64 / self.start.elapsed().as_secs()) + success_bonus
    }
}

enum NodeType {
    Input,
    Hidden,
    Output,
}

struct Node(NodeType);

struct Gene {
    in_node: usize,
    out_node: usize,
    weight: f64,
    enabled: bool,
    innovation_number: u64,
}

struct Individual {
    nodes: Vec<Node>,
    genes: Vec<Gene>,
    fitness: u64,
}

impl Individual {
    pub fn new(max_innovation_number: u64) -> Self {
        const INPUT_NODES: usize = SCREEN_SIZE * SCREEN_SIZE;
        let mut nodes = vec![];
        for _ in 0..INPUT_NODES {
            nodes.push(Node(NodeType::Input));
        }
        for _ in 0..2 {
            nodes.push(Node(NodeType::Output));
        }

        let mut rng = rand::thread_rng();
        let input_distribution = Uniform::from(0..INPUT_NODES);
        let output_distribtion = Uniform::from(INPUT_NODES..(INPUT_NODES + 2));
        let genes = vec![
            Gene {
                in_node: input_distribution.sample(&mut rng),
                out_node: output_distribtion.sample(&mut rng),
                innovation_number: max_innovation_number + 1,
                weight: 1.0,
                enabled: true,
            },
            Gene {
                in_node: input_distribution.sample(&mut rng),
                out_node: output_distribtion.sample(&mut rng),
                innovation_number: max_innovation_number + 2,
                weight: 1.0,
                enabled: true,
            },
        ];

        Self {
            nodes,
            genes,
            fitness: 0,
        }
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    pub fn evaluate(&self, input: [f64; SCREEN_SIZE * SCREEN_SIZE]) -> (f64, f64) {
        unimplemented!()
    }
}

#[derive(Default)]
struct Species {
    id: u64,
    members: Vec<Individual>,
    staleness: u64,
    top_fitness: u64,
}

impl Species {
    fn size(&self) -> usize {
        self.members.len()
    }

    fn sort(&mut self) {
        self.members.sort_by_key(|individual| Reverse(individual.fitness));
    }
}

pub struct Ai {
    pool: Vec<Species>,
    generation: u64,
    max_fitness: u64,
    // (species_index, individual_index)
    current_individual: (usize, usize),
    current_individual_state: IndividualState,
    stuck_timeout_s: u64,
    finish_timeout_s: u64,
}

impl Ai {
    pub fn new(options: AiOptions) -> Self {
        Self {
            pool: vec![Species {
                id: 0,
                members: vec![Individual::new(0)],
                ..Species::default()
            }, Species {
                id: 1,
                members: vec![Individual::new(3)],
                ..Species::default()
            }],
            generation: 0,
            max_fitness: 0,
            current_individual: (0, 0),
            current_individual_state: IndividualState::new(IndividualStateOptions {
                stuck_timeout_s: options.stuck_timeout_s,
                finish_timeout_s: options.finish_timeout_s,
            }),
            stuck_timeout_s: options.stuck_timeout_s,
            finish_timeout_s: options.finish_timeout_s,
        }
    }

    fn adjusted_fitness(fitness: u64, species_size: usize) -> u64 {
        let species_size = if species_size < 20 {
            1
        } else {
            species_size as u64
        };
        fitness / species_size
    }

    fn update_max_fitness(&mut self) {
        self.max_fitness = self
            .pool
            .iter()
            .map(|species| species.members.iter().map(|individual| individual.fitness))
            .flatten()
            .max()
            .unwrap();
    }

    fn sort_species(&mut self) {
        for species in &mut self.pool {
            species.sort();
        }
    }

    // Remove bottom half of each species
    fn cull_species(&mut self) {
        for species in &mut self.pool {
            species.members.truncate(species.members.len() / 2);
        }
    }

    // Remove species that have not improved after a certain amount of
    // iterations
    fn remove_stale_species(&mut self) {
        for species in &mut self.pool {
            let current_top_fitness = species.members[0].fitness;
            if current_top_fitness > species.top_fitness {
                species.top_fitness = current_top_fitness;
                species.staleness = 0;
            } else {
                species.staleness += 1;
            }
        }
        let max_fitness = self.max_fitness;
        self.pool.retain(|species| {
            species.staleness < MAX_SPECIES_STALENESS || species.top_fitness == max_fitness
        });
    }

    fn remove_weak_species(&mut self) {
        self.pool.sort_by_key(|species| Reverse(species.members[0].fitness));
        self.pool.truncate(MAX_SPECIES / 2);
    }

    fn population(&self) -> i64 {
        self.pool.iter().map(|species| species.size()).sum::<usize>() as i64
    }

    // Breed individuals of same species
    fn cross_over_within_species(&self) {
        unimplemented!()
    }

    // Breed individuals of different species
    fn cross_over_between_species(&self) {
        let children_needed = DESIRED_POPULATION - self.population();
        for _ in 0..children_needed {
            // TODO: breed random species
        }
        unimplemented!()
    }

    // Mutate random individuals
    fn mutate(&self) {
        unimplemented!()
    }

    fn next_generation(&mut self) {
        self.update_max_fitness();
        self.sort_species();
        self.cull_species();
        self.remove_stale_species();
        if self.pool.len() > MAX_SPECIES {
            self.remove_weak_species();
        }
        self.cross_over_within_species();
        self.cross_over_between_species();
        self.mutate();
        self.generation += 1;
    }

    pub fn next_individual(&mut self) {
        let (species_index, individual_index) = self.current_individual;
        let species_size = self.pool[species_index].size();
        let individual = &mut self.pool[species_index].members[individual_index];
        individual.fitness =
            Self::adjusted_fitness(self.current_individual_state.fitness(), species_size);

        let pool_size = self.pool.len();

        self.current_individual = if individual_index == species_size - 1 {
            if species_index == pool_size - 1 {
                self.next_generation();
                (0, 0)
            } else {
                (species_index + 1, 0)
            }
        } else {
            (species_index, individual_index + 1)
        };

        self.current_individual_state = IndividualState::new(IndividualStateOptions {
            stuck_timeout_s: self.stuck_timeout_s,
            finish_timeout_s: self.finish_timeout_s,
        });
    }

    pub fn update_game_state(&mut self, mut cpu: &mut cpu::Cpu<mem::MemMap>) {
        self.current_individual_state.update(&mut cpu);
    }

    pub fn debug_game_state(&self) {
        self.current_individual_state.debug_game_state();
    }

    pub fn is_stuck(&self) -> bool {
        self.current_individual_state.is_stuck()
    }

    pub fn is_dead(&self) -> bool {
        self.current_individual_state.is_dead()
    }

    pub fn has_succeeded(&self) -> bool {
        self.current_individual_state.has_succeeded()
    }

    pub fn get_screen(&self) -> Screen {
        self.current_individual_state.get_screen()
    }

    pub fn get_inputs(&self) -> Inputs {
        const THRESHOLD: f64 = 0.5;

        let (species_index, individual_index) = self.current_individual;
        let individual = &self.pool[species_index].members[individual_index];
        let input = {
            let screen = self.get_screen();
            let mut input = [0.0; SCREEN_SIZE * SCREEN_SIZE];
            for i in 0..SCREEN_SIZE {
                for j in 0..SCREEN_SIZE {
                    input[(i * SCREEN_SIZE) + j] = screen[i][j].as_nn_input();
                }
            }
            input
        };
        let (right_value, a_value) = individual.evaluate(input);

        Inputs {
            right: right_value > THRESHOLD,
            a: a_value > THRESHOLD,
        }
    }
}
