mod game_state;

use self::game_state::GameState;
use crate::nes::{cpu, mem};
use crate::utils::{Screen, Tile, SCREEN_SIZE};

use rand::distributions::{Distribution, Uniform};
use rand::seq::SliceRandom;
use rand::Rng;

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const INPUT_NODES: usize = SCREEN_SIZE * SCREEN_SIZE;
const OUTPUT_NODES: usize = 2;

const DESIRED_POPULATION: i64 = 300;

// Maximum number of species in pool before weaker species are removed
const MAX_SPECIES: usize = 30;

// Maximum number of generations that a species can exist for without
// improving its performance
const MAX_SPECIES_STALENESS: u64 = 15;

// Threshold below which a compatibility distance implies that two individuals
// are of the same species
const COMPATIBILITY_THRESHOLD: f64 = 3.0;
// Coefficients used when calculating compatibility distance of two individuals.
// While the NEAT paper distinguishes between 'excess' and 'disjoint' genes,
// here we use the term 'disjoint' to refer to both
const DISJOINT_COEFFICIENT: f64 = 0.4;
const WEIGHTS_COEFFICIENT: f64 = 0.1;

const MUTATION_PROBABILITY: f64 = 0.2;

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

#[derive(Copy, Clone, PartialEq)]
enum NodeType {
    Input,
    Hidden,
    Output,
}

#[derive(Copy, Clone)]
struct Node(NodeType);

impl Node {
    fn is_output_node(&self) -> bool {
        self.0 == NodeType::Output
    }

    fn is_hidden_node(&self) -> bool {
        self.0 == NodeType::Hidden
    }
}

#[derive(Copy, Clone)]
struct Gene {
    in_node: usize,
    out_node: usize,
    weight: f64,
    enabled: bool,
    innovation_number: u64,
}

#[derive(Default)]
struct Individual {
    nodes: Vec<Node>,
    genes: Vec<Gene>,
    fitness: u64,
}

impl Individual {
    pub fn new(max_innovation_number: u64) -> Self {
        let mut nodes = vec![Node(NodeType::Input); INPUT_NODES];
        nodes.append(&mut vec![Node(NodeType::Output); OUTPUT_NODES]);
        let mut rng = rand::thread_rng();
        let input_distribution = Uniform::from(0..INPUT_NODES);
        let output_distribution = Uniform::from(INPUT_NODES..(INPUT_NODES + OUTPUT_NODES));
        let genes = vec![
            Gene {
                in_node: input_distribution.sample(&mut rng),
                out_node: output_distribution.sample(&mut rng),
                innovation_number: max_innovation_number + 1,
                weight: 1.0,
                enabled: true,
            },
            Gene {
                in_node: input_distribution.sample(&mut rng),
                out_node: output_distribution.sample(&mut rng),
                innovation_number: max_innovation_number + 2,
                weight: 1.0,
                enabled: true,
            },
        ];

        Self {
            nodes,
            genes,
            ..Self::default()
        }
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    pub fn enabled_genes_mut(&mut self) -> Vec<&mut Gene> {
        self.genes.iter_mut().filter(|g| g.enabled).collect()
    }

    pub fn evaluate(&self, input: [f64; INPUT_NODES]) -> (f64, f64) {
        let mut nodes = vec![0.0; self.nodes.len()];
        for i in 0..INPUT_NODES {
            nodes[i] = input[i];
        }
        let mut incoming = HashMap::new();
        for gene in &self.genes {
            if !gene.enabled {
                continue;
            }
            let entry = incoming.entry(gene.out_node).or_insert(vec![]);
            entry.push((gene.in_node, gene.weight));
        }
        for i in (INPUT_NODES + OUTPUT_NODES)..self.nodes.len() {
            if let Some(incoming) = incoming.get(&i) {
                let sum = incoming
                    .iter()
                    .fold(0.0, |acc, (node, weight)| acc + (nodes[*node] * weight));
                nodes[i] = Self::sigmoid(sum);
            }
        }
        for i in INPUT_NODES..(INPUT_NODES + OUTPUT_NODES) {
            if let Some(incoming) = incoming.get(&i) {
                let sum = incoming
                    .iter()
                    .fold(0.0, |acc, (node, weight)| acc + (nodes[*node] * weight));
                nodes[i] = Self::sigmoid(sum);
            }
        }
        (nodes[INPUT_NODES], nodes[INPUT_NODES + 1])
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
    fn len(&self) -> usize {
        self.members.len()
    }

    fn sort(&mut self) {
        self.members
            .sort_by_key(|individual| Reverse(individual.fitness));
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
            pool: vec![
                Species {
                    id: 0,
                    members: vec![Individual::new(0)],
                    ..Species::default()
                },
                Species {
                    id: 1,
                    members: vec![Individual::new(3)],
                    ..Species::default()
                },
            ],
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

    fn adjusted_fitness(fitness: u64, species_size: u64) -> u64 {
        fitness / species_size
    }

    fn cross_over(a: &Individual, b: &Individual) -> Individual {
        // Ensure A is fitter than B
        let (a, b) = if a.fitness < b.fitness {
            (b, a)
        } else {
            (a, b)
        };
        let a_innos: HashSet<_> = a.genes.iter().map(|g| g.innovation_number).collect();
        let b_innos: HashSet<_> = b.genes.iter().map(|g| g.innovation_number).collect();
        let mut matching_genes: HashMap<_, _> = a_innos
            .union(&b_innos)
            .cloned()
            .map(|i| (i, (None, None)))
            .collect();
        let mut disjoint_genes: HashMap<_, _> = a_innos
            .difference(&b_innos)
            .cloned()
            .map(|i| (i, None))
            .collect();
        for gene in &a.genes {
            matching_genes
                .entry(gene.innovation_number)
                .and_modify(|(g, _)| *g = Some(gene));
            disjoint_genes
                .entry(gene.innovation_number)
                .and_modify(|g| *g = Some(gene));
        }
        for gene in &b.genes {
            matching_genes
                .entry(gene.innovation_number)
                .and_modify(|(_, g)| *g = Some(gene));
        }
        let mut genes: Vec<Gene> = matching_genes
            .values()
            .map(|(g_a, g_b)| {
                if rand::random() {
                    (*g_a.unwrap()).clone()
                } else {
                    (*g_b.unwrap()).clone()
                }
            })
            .collect();
        let mut disjoint_genes = disjoint_genes
            .values()
            .map(|g| (*g.unwrap()).clone())
            .collect();
        genes.append(&mut disjoint_genes);
        Individual {
            nodes: a.nodes.clone(),
            genes,
            ..Individual::default()
        }
    }

    fn mutate_add_connection(individual: &mut Individual, max_innovation_number: u64) -> u64 {
        let mut rng = rand::thread_rng();
        let in_distribution = Uniform::from(0..individual.nodes.len());
        let mut in_node = in_distribution.sample(&mut rng);
        while individual.nodes[in_node].is_output_node() || in_node == individual.nodes.len() - 1 {
            in_node = in_distribution.sample(&mut rng);
        }
        let out_distribution = Uniform::from(INPUT_NODES..individual.nodes.len());
        let mut out_node = out_distribution.sample(&mut rng);
        // To prevent cycles, ensure out node comes after in node if out node is
        // hidden node (output nodes come before hidden nodes, so only perform
        // check for hidden nodes)
        while individual.nodes[out_node].is_hidden_node() && out_node < in_node {
            out_node = out_distribution.sample(&mut rng);
        }
        let innovation_number = max_innovation_number + 1;
        individual.genes.push(Gene {
            in_node,
            out_node,
            weight: 1.0,
            enabled: true,
            innovation_number,
        });
        innovation_number
    }

    fn mutate_add_node(individual: &mut Individual, max_innovation_number: u64) -> u64 {
        let gene = {
            let mut rng = rand::thread_rng();
            let mut enabled_genes = individual.enabled_genes_mut();
            let gene: &mut Gene = enabled_genes.choose_mut(&mut rng).unwrap();
            gene.enabled = false;
            gene.clone()
        };

        let new_node = Node(NodeType::Hidden);
        individual.nodes.push(new_node);
        let new_node_index = individual.nodes.len() - 1;

        individual.genes.push(Gene {
            in_node: gene.in_node,
            out_node: new_node_index,
            weight: 1.0,
            enabled: true,
            innovation_number: max_innovation_number + 1,
        });
        individual.genes.push(Gene {
            in_node: new_node_index,
            out_node: gene.out_node,
            weight: 1.0,
            enabled: true,
            innovation_number: max_innovation_number + 2,
        });

        max_innovation_number + 2
    }

    fn mutate_change_weight(individual: &mut Individual, max_innovation_number: u64) -> u64 {
        let mut rng = rand::thread_rng();
        let mut enabled_genes = individual.enabled_genes_mut();
        let gene: &mut Gene = enabled_genes.choose_mut(&mut rng).unwrap();
        gene.weight = rng.gen_range(-2.0, 2.0);
        max_innovation_number
    }

    fn mutate(mut individual: &mut Individual, max_innovation_number: u64) -> u64 {
        let mut rng = rand::thread_rng();
        let f = match rng.gen_range(0, 3) {
            0 => Self::mutate_add_connection,
            1 => Self::mutate_add_node,
            _ => Self::mutate_change_weight,
        };
        f(&mut individual, max_innovation_number)
    }

    fn compatibility_distance(a: &Individual, b: &Individual) -> f64 {
        let n = a.genes.len().max(b.genes.len());
        let n = if n < 20 { 1.0 } else { n as f64 };
        let a_innos: HashSet<_> = a.genes.iter().map(|g| g.innovation_number).collect();
        let b_innos: HashSet<_> = b.genes.iter().map(|g| g.innovation_number).collect();
        let disjoint_size = a_innos
            .symmetric_difference(&b_innos)
            .collect::<HashSet<_>>()
            .len() as f64;
        let mut weights: HashMap<_, _> = a_innos
            .union(&b_innos)
            .cloned()
            .map(|i| (i, (0.0, 0.0)))
            .collect();
        for gene in &a.genes {
            weights
                .entry(gene.innovation_number)
                .and_modify(|(w, _)| *w = gene.weight);
        }
        for gene in &b.genes {
            weights
                .entry(gene.innovation_number)
                .and_modify(|(_, w)| *w = gene.weight);
        }
        let sum_of_differences = weights.values().fold(0.0, |sum, (weight_a, weight_b)| {
            sum + (weight_a - weight_b).abs()
        });
        let average_weights_difference = sum_of_differences / weights.len() as f64;
        (DISJOINT_COEFFICIENT * disjoint_size / n)
            + (WEIGHTS_COEFFICIENT * average_weights_difference)
    }

    fn is_same_species(a: &Individual, b: &Individual) -> bool {
        Self::compatibility_distance(a, b) < COMPATIBILITY_THRESHOLD
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
        if self.pool.len() > MAX_SPECIES {
            self.pool
                .sort_by_key(|species| Reverse(species.members[0].fitness));
            self.pool.truncate(MAX_SPECIES / 2);
        }
    }

    fn population(&self) -> i64 {
        self.pool.iter().map(|species| species.len()).sum::<usize>() as i64
    }

    // Breed individuals of same species
    fn cross_over_within_species(&mut self) {
        let mut rng = rand::thread_rng();
        for species in &mut self.pool {
            if species.len() < 2 {
                continue;
            }
            for _ in 0..species.len() {
                let parents: Vec<&Individual> =
                    species.members.choose_multiple(&mut rng, 2).collect();
                let child = Self::cross_over(parents[0], parents[1]);
                species.members.push(child);
            }
        }
    }

    fn add_to_pool(&mut self, individual: Individual) {
        let mut species: Option<&mut Species> = None;
        for s in &mut self.pool {
            if Self::is_same_species(&individual, &s.members[0]) {
                species = Some(s);
                break;
            }
        }
        match species {
            Some(s) => s.members.push(individual),
            None => {
                let new_species = Species {
                    id: self.pool.iter().map(|species| species.id).max().unwrap() + 1,
                    members: vec![individual],
                    ..Species::default()
                };
                self.pool.push(new_species);
            }
        }
    }

    // Breed individuals of different species
    fn cross_over_between_species(&mut self) {
        let mut rng = rand::thread_rng();
        let children_needed = DESIRED_POPULATION - self.population();
        for _ in 0..children_needed {
            let species: Vec<&Species> = self.pool.choose_multiple(&mut rng, 2).collect();
            let parent_a = species[0].members.choose(&mut rng).unwrap();
            let parent_b = species[1].members.choose(&mut rng).unwrap();
            let child = Self::cross_over(parent_a, parent_b);
            self.add_to_pool(child);
        }
    }

    fn mutate_random_invididuals(&mut self) {
        let mut max_innovation_number = self
            .pool
            .iter()
            .map(|species| &species.members)
            .flatten()
            .map(|individual| &individual.genes)
            .flatten()
            .map(|gene| gene.innovation_number)
            .max()
            .unwrap();
        for species in &mut self.pool {
            for mut individual in &mut species.members {
                if rand::random::<f64>() < MUTATION_PROBABILITY {
                    max_innovation_number = Self::mutate(&mut individual, max_innovation_number);
                }
            }
        }
    }

    fn next_generation(&mut self) {
        self.update_max_fitness();
        self.sort_species();
        self.cull_species();
        self.remove_stale_species();
        self.remove_weak_species();
        self.cross_over_within_species();
        self.cross_over_between_species();
        self.mutate_random_invididuals();
        self.generation += 1;
    }

    pub fn next_individual(&mut self) {
        let (species_index, individual_index) = self.current_individual;
        let species_size = self.pool[species_index].len();
        let individual = &mut self.pool[species_index].members[individual_index];
        individual.fitness =
            Self::adjusted_fitness(self.current_individual_state.fitness(), species_size as u64);

        let pool_size = self.pool.len();

        self.current_individual = if individual_index == species_size - 1 {
            if species_index == pool_size - 1 {
                self.next_generation();
                println!(
                    "New generation (g = {}). Population = {}. Species = {}",
                    self.generation,
                    self.population(),
                    self.pool.len()
                );
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
