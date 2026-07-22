use quick_cache::unsync::Cache;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use web_time::{Duration, Instant};

use crate::eval::normalize::{NormalizedIncompleteInformation, RankCompressed};
use crate::eval::puct::{ActionProbabilities, PUCTNode};
use crate::eval::zobrist::{ZobristHash, ZobristTable};
pub trait ActionPriorHeuristic {
    fn action_priors(
        &mut self,
        state: &NormalizedIncompleteInformation,
    ) -> RankCompressed<ActionProbabilities>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchConfig {
    pub exploration_factor: f32,
    pub temperature: f32,
    pub greediness: f32,

    pub full_tree_depth: usize,

    pub num_worlds: usize,

    pub node_budget: usize,
    pub min_root_visits: usize,
    pub puct_rollout_bounds: (usize, usize),
    pub puct_mature_node_min_visits: usize,

    pub puct_node_capacity: usize,
}

impl SearchConfig {
    pub fn inference() -> Self {
        Self {
            exploration_factor: 1.8,
            temperature: 0.2,
            greediness: 1.5,
            full_tree_depth: 0,
            num_worlds: 30,
            node_budget: 4_000_000,
            min_root_visits: 5,
            puct_rollout_bounds: (8, 60),
            puct_mature_node_min_visits: 256,
            puct_node_capacity: 4_000_000,
        }
    }

    pub fn training(temperature_schedule: f32) -> Self {
        Self {
            exploration_factor: 1.25,
            temperature: temperature_schedule,
            greediness: 1.5,
            full_tree_depth: 1,
            num_worlds: 200,
            node_budget: 4_000_000,
            min_root_visits: 5,
            puct_rollout_bounds: (8, 60),
            puct_mature_node_min_visits: 256,
            puct_node_capacity: 4_000_000,
        }
    }
}

#[derive(Debug)]
pub struct SearchStats {
    pub puct_num_rollouts: usize,
    pub puct_nodes_visited: usize,
    pub puct_nodes_created: usize,
    pub puct_valid_actions_seen: usize,

    pub full_tree_nodes_visited: usize,
    pub full_tree_edges_evaluated: usize,
    pub full_tree_puct_calls: usize,

    pub total_sampled_worlds: usize,

    pub start_time: Instant,
}

impl SearchStats {
    pub fn new() -> Self {
        Self {
            puct_num_rollouts: 0,
            puct_nodes_visited: 0,
            puct_nodes_created: 0,
            puct_valid_actions_seen: 0,
            full_tree_nodes_visited: 0,
            full_tree_edges_evaluated: 0,
            full_tree_puct_calls: 0,
            total_sampled_worlds: 0,
            start_time: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        return self.start_time.elapsed();
    }

    pub fn print_stats(&self) {
        let elapsed = self.elapsed();

        println!("=== RAW STATS ===");
        println!("Full-Tree Stats:");
        println!("    Nodes Visited: {}", self.full_tree_nodes_visited);
        println!("    Edges Evaluated: {}", self.full_tree_edges_evaluated);
        println!("    PUCT Calls: {}", self.full_tree_puct_calls);
        println!("PUCT Stats:");
        println!("    Num Rollouts: {}", self.puct_num_rollouts);
        println!("    Nodes Visited: {}", self.puct_nodes_visited);
        println!("    Nodes Created: {}", self.puct_nodes_created);
        println!("    Valid Actions Seen: {}", self.puct_valid_actions_seen);
        println!("General:");
        println!("    Sampled Worlds: {}", self.total_sampled_worlds);
        println!("    Elapsed Time: {:?}", elapsed);
        println!("");
        println!("=== CALCULATED STATS ===");
        // Cache hit rate
        let num_hits = self.puct_nodes_visited - self.puct_nodes_created;
        let hit_rate = num_hits as f64 / self.puct_nodes_visited as f64;
        println!("Cache Hit Rate: {}", hit_rate);
        // Average rollout length
        let avg_rollout = self.puct_nodes_visited as f64 / self.puct_num_rollouts as f64;
        println!("Average Rollout Length: {}", avg_rollout);
        // Average valid actions
        let avg_valid = self.puct_valid_actions_seen as f64 / self.puct_nodes_visited as f64;
        println!("Average Valid per Node: {}", avg_valid);
        // Nodes per second
        let total_nodes = self.full_tree_nodes_visited + self.puct_nodes_visited;
        let nodes_per_second = total_nodes as f64 / elapsed.as_secs_f64();
        println!("Average Nodes per Second: {}", nodes_per_second);
    }
}

// TODO: improve the cache with a custom weighter
// We should check out the current weighter implementation.
pub struct SearchContext<'a, H: ActionPriorHeuristic> {
    pub heuristic: &'a mut H,
    pub puct_nodes: Cache<ZobristHash, PUCTNode>,
    pub zobrist_hash: ZobristTable,
    pub config: SearchConfig,
    pub rng: SmallRng,
    pub stats: SearchStats,
}

impl<'a, H: ActionPriorHeuristic> SearchContext<'a, H> {
    pub fn new(heuristic: &'a mut H, config: SearchConfig) -> Self {
        return Self::with_seed(heuristic, config, 42);
    }

    pub fn with_seed(heuristic: &'a mut H, config: SearchConfig, seed: u64) -> Self {
        Self {
            heuristic: heuristic,
            puct_nodes: Cache::new(config.puct_node_capacity),
            zobrist_hash: ZobristTable::new(seed),
            config: config,
            rng: SmallRng::seed_from_u64(seed),
            stats: SearchStats::new(),
        }
    }

    pub fn reset_stats(&mut self) {
        self.stats = SearchStats::new();
    }
}
