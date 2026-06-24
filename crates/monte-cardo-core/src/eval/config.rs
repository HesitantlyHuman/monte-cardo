use std::collections::HashMap;

use quick_cache::unsync::Cache;
use rand::rngs::SmallRng;
use rand::SeedableRng;

use crate::eval::normalize::NormalizedIncompleteInformation;
use crate::eval::puct::{ActionProbabilities, PUCTNode};

pub trait ActionPriorHeuristic {
    fn action_priors(&mut self, state: &NormalizedIncompleteInformation) -> ActionProbabilities;
}

pub struct SearchConfig {
    pub full_tree_depth: usize,
    pub num_worlds: usize,
    pub puct_rollouts_per_leaf: usize,
    pub exploration_factor: f32,
    pub temperature: f32,
    pub greediness: f32,

    pub node_capacity: usize,
}

impl SearchConfig {
    pub fn inference() -> Self {
        Self {
            full_tree_depth: 2,
            num_worlds: 50,
            puct_rollouts_per_leaf: 200,
            exploration_factor: 2.0,
            temperature: 0.25,
            greediness: 1.5,

            node_capacity: 100_000,
        }
    }

    pub fn training(temperature_schedule: f32) -> Self {
        Self {
            full_tree_depth: 2,
            num_worlds: 50,
            puct_rollouts_per_leaf: 200,
            exploration_factor: 2.0,
            temperature: temperature_schedule,
            greediness: 1.5,

            node_capacity: 100_000,
        }
    }
}

pub struct SearchContext<'a, H: ActionPriorHeuristic> {
    pub heuristic: &'a mut H,
    pub nodes: Cache<NormalizedIncompleteInformation, PUCTNode>,
    pub config: SearchConfig,
    pub rng: SmallRng,
}

impl<'a, H: ActionPriorHeuristic> SearchContext<'a, H> {
    pub fn new(heuristic: &'a mut H, config: SearchConfig) -> Self {
        Self {
            heuristic: heuristic,
            nodes: Cache::new(config.node_capacity),
            config: config,
            rng: SmallRng::seed_from_u64(42),
        }
    }

    pub fn with_seed(heuristic: &'a mut H, config: SearchConfig, seed: u64) -> Self {
        Self {
            heuristic: heuristic,
            nodes: Cache::new(config.node_capacity),
            config: config,
            rng: SmallRng::seed_from_u64(seed),
        }
    }
}
