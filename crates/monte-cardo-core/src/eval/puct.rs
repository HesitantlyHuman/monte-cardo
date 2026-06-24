use std::ops::Index;
use std::ops::IndexMut;

use crate::consts;
use crate::eval::actions::{ActionMask, MoveID, NUM_ACTIONS};
use crate::eval::config::{ActionPriorHeuristic, SearchContext};
use crate::eval::evaluate::EvaluationError;
use crate::eval::evaluate::{update_world, ActionValueMatrix, PlayerValues};
use crate::eval::normalize::{
    left_rotate_array, normalize_incomplete_information_state, NormalizedIncompleteInformation,
};
use crate::game;

#[derive(Debug, Clone)]
pub struct ActionProbabilities([f32; NUM_ACTIONS]);

impl ActionProbabilities {
    #[inline]
    pub fn zeros() -> Self {
        return Self([0.0; NUM_ACTIONS]);
    }

    #[inline]
    pub fn ones() -> Self {
        return Self([1.0; NUM_ACTIONS]);
    }

    #[inline]
    pub fn sum(&self) -> f32 {
        return self.0.iter().sum();
    }

    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        return self.0.iter();
    }

    pub fn normalize(&mut self) {
        let sum = self.sum();
        self.0.iter_mut().for_each(|val| *val /= sum);
    }
}

impl Index<MoveID> for ActionProbabilities {
    type Output = f32;

    fn index(&self, index: MoveID) -> &Self::Output {
        return &self.0[index.get()];
    }
}

impl IndexMut<MoveID> for ActionProbabilities {
    fn index_mut(&mut self, index: MoveID) -> &mut Self::Output {
        return &mut self.0[index.get()];
    }
}

pub struct PUCTNode {
    action_mask: ActionMask,
    action_priors: ActionProbabilities,
    visit_counts: [u32; NUM_ACTIONS],
    accumulated_values: ActionValueMatrix,
}

impl PUCTNode {
    fn new(action_mask: ActionMask, action_priors: ActionProbabilities) -> Self {
        PUCTNode {
            action_mask: action_mask,
            action_priors: action_priors,
            visit_counts: [0; NUM_ACTIONS],
            accumulated_values: ActionValueMatrix::zeros(),
        }
    }
}

fn puct_score(
    node: &PUCTNode,
    action_id: MoveID,
    player_id: game::PlayerID,
    exploration_factor: f32,
) -> f32 {
    let n_action_taken = node.visit_counts[action_id.get()];
    let n_times_at_node: u32 = node.visit_counts.iter().sum::<u32>().max(1);

    let q_term = if n_action_taken == 0 {
        0.0
    } else {
        node.accumulated_values[action_id][player_id] / n_action_taken as f32
    };

    let prior = node.action_priors[action_id];
    let exploration_term = exploration_factor
        * prior
        * ((n_times_at_node as f32).sqrt() / (1.0 + n_action_taken as f32));

    return q_term + exploration_term;
}

fn select_puct_action(
    node: &PUCTNode,
    player_id: game::PlayerID,
    exploration_factor: f32,
) -> MoveID {
    let mut best_action = None;
    let mut best_score = f32::NEG_INFINITY;

    for action_id in MoveID::all() {
        if !node.action_mask[action_id] {
            continue;
        }

        let score = puct_score(node, action_id, player_id, exploration_factor);

        if score > best_score {
            best_action = Some(action_id);
            best_score = score;
        }
    }

    // TODO: Consider returning a result
    best_action.expect("PUCTNode has no valid actions!")
}

fn update_puct_node(
    node: &mut PUCTNode,
    player_values: &PlayerValues,
    selected_action: MoveID,
    player_at_time: game::PlayerID,
    state_at_time: &NormalizedIncompleteInformation,
) {
    // First, rotate our values to be in the normalized orientation (player at that time is player 0)
    let mut rotated_player_values = [0.0; consts::MAX_PLAYERS];
    left_rotate_array(
        player_values.get(),
        &mut rotated_player_values,
        state_at_time.number_of_players,
        player_at_time.get(),
    );

    // Then update the accumulated values of the relevant node
    for player_id in game::PlayerID::all_player_ids(state_at_time.number_of_players) {
        node.accumulated_values[selected_action][player_id] +=
            rotated_player_values[player_id.get()]
    }
    node.visit_counts[selected_action.get()] += 1;
}

fn create_search_node<H: ActionPriorHeuristic>(
    incomplete_information: &game::IncompleteInformationGameState,
    normalized_information_state: &NormalizedIncompleteInformation,
    heuristic: &mut H,
) -> PUCTNode {
    let action_priors = heuristic.action_priors(normalized_information_state);
    let valid_action_mask = ActionMask::from_hand_and_top(
        &incomplete_information.player_hand,
        &incomplete_information.trick.top_set,
    );

    let mut normalized_action_priors = ActionProbabilities::zeros();
    let mut unmasked_sum = 0.0;
    for move_id in MoveID::all() {
        let is_valid = valid_action_mask[move_id];

        if !is_valid {
            continue;
        }

        let prior = action_priors[move_id];

        normalized_action_priors[move_id] = prior;
        unmasked_sum += prior;
    }

    if unmasked_sum <= 0.0 {
        let num_valid = valid_action_mask.num_valid();
        debug_assert!(num_valid > 0);

        for move_id in MoveID::all() {
            if valid_action_mask[move_id] {
                normalized_action_priors[move_id] = 1.0 / num_valid as f32;
            }
        }
    } else {
        for move_id in MoveID::all() {
            normalized_action_priors[move_id] /= unmasked_sum;
        }
    }

    return PUCTNode::new(valid_action_mask, normalized_action_priors);
}

/// Runs a single PUCT rollout evalution of the current full information state.
///
/// Returns an unrotated set of final player values, along with the MoveID
/// corresponding to the action that was taken from input
/// FullInformationGameState.
pub fn puct_rollout<H: ActionPriorHeuristic>(
    world: &game::FullInformationGameState,
    search_context: &mut SearchContext<H>,
) -> Result<(MoveID, PlayerValues), EvaluationError> {
    let mut world = world.clone();
    let mut first_action = None;
    let mut nodes_to_update_on_return = Vec::new();

    // Worst case is playing one card per turn
    // Should theoretically multiply by the number of players, because every
    // player other than the first can pass, but we will assume that these
    // players are generally more efficient than that.
    for _ in 0..(consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY * 2) {
        let current_player_information =
            game::create_incomplete_information_game_state(&world, world.current_player_number);
        let normalized_player_information =
            normalize_incomplete_information_state(&current_player_information);
        let search_node = search_context
            .nodes
            .entry(normalized_player_information.clone())
            .or_insert_with(|| {
                create_search_node(
                    &current_player_information,
                    &normalized_player_information,
                    search_context.heuristic,
                )
            });

        let selected_action = select_puct_action(
            search_node,
            game::PlayerID::new(0),
            search_context.config.exploration_factor,
        );
        nodes_to_update_on_return.push((
            selected_action,
            world.current_player_number,
            normalized_player_information,
        ));

        let action_move = selected_action.to_move(&current_player_information.player_hand)?;
        if first_action.is_none() {
            first_action = Some(selected_action);
        }
        match update_world(&mut world, action_move, search_context)? {
            Some(player_values) => {
                // First update the stats for all the nodes we visited
                for (selected_action, player_at_time, key) in nodes_to_update_on_return {
                    if let Some(search_node) = search_context.nodes.get_mut(&key) {
                        update_puct_node(
                            search_node,
                            &player_values,
                            selected_action,
                            player_at_time,
                            &key,
                        );
                    }
                }

                // Then return!
                return Ok((
                    first_action.expect("Tried to rollout a finished game!"),
                    player_values,
                ));
            }
            None => {}
        }
    }

    return Err(EvaluationError::RolloutError);
}
