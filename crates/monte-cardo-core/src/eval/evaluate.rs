use std::ops::Index;
use std::ops::IndexMut;

use rand_distr::weighted::WeightedAliasIndex;
use rand_distr::Distribution;
use thiserror::Error;

use crate::consts;
use crate::eval::normalize::NormalizationError;
use crate::game;

use crate::eval::actions::{ActionMask, MoveID, MoveIDError, NUM_ACTIONS};
use crate::eval::config::{ActionPriorHeuristic, SearchContext};
use crate::eval::puct::{puct_rollout, ActionProbabilities};
use crate::game::{CardCount, CardRank, GameLogicError, PlayerHand, PlayerID, PlayerIndexed};

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct PlayerValues(PlayerIndexed<f32>);

impl PlayerValues {
    #[inline]
    pub fn new(values: PlayerIndexed<f32>) -> Self {
        return Self(values);
    }

    #[inline]
    pub fn zeros() -> Self {
        return Self(PlayerIndexed::filled(0.0));
    }

    #[inline]
    pub fn get(&self) -> &[f32; consts::MAX_PLAYERS] {
        return self.0.get();
    }
}

impl Index<PlayerID> for PlayerValues {
    type Output = f32;

    #[inline]
    fn index(&self, index: PlayerID) -> &Self::Output {
        return &self.0[index];
    }
}

impl IndexMut<PlayerID> for PlayerValues {
    #[inline]
    fn index_mut(&mut self, index: PlayerID) -> &mut Self::Output {
        return &mut self.0[index];
    }
}

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ActionValueMatrix([PlayerValues; NUM_ACTIONS]);

impl ActionValueMatrix {
    #[inline]
    pub fn zeros() -> Self {
        return Self(std::array::from_fn(|_| PlayerValues::zeros()));
    }
}

impl Index<MoveID> for ActionValueMatrix {
    type Output = PlayerValues;

    #[inline]
    fn index(&self, index: MoveID) -> &Self::Output {
        return &self.0[index.get()];
    }
}

impl IndexMut<MoveID> for ActionValueMatrix {
    #[inline]
    fn index_mut(&mut self, index: MoveID) -> &mut Self::Output {
        return &mut self.0[index.get()];
    }
}

#[derive(Error, Debug)]
pub enum EvaluationError {
    #[error("There are no valid actions from the given game state")]
    NoValidActions,
    #[error("Failed to convert to or from a MoveID: {0}")]
    MoveIDError(#[from] MoveIDError),
    #[error("Error in the game logic: {0}")]
    GameLogicError(#[from] GameLogicError),
    #[error("PUCT rollout failed to finish")]
    RolloutError,
    #[error("Normalization failed: {0}")]
    NormalizationError(#[from] NormalizationError),
}

pub fn get_action_values<H: ActionPriorHeuristic>(
    incomplete_information_state: &game::IncompleteInformationGameState,
    search_context: &mut SearchContext<H>,
) -> Result<Vec<(game::Move, f32)>, EvaluationError> {
    debug_assert!(
        incomplete_information_state.current_player_number
            == incomplete_information_state.perspective_player_number
    );

    let (action_values, action_mask) =
        full_tree_evaluation(incomplete_information_state, search_context, 0)?;

    ordered_player_action_values(
        &action_values,
        &action_mask,
        incomplete_information_state.current_player_number,
        &incomplete_information_state.player_hand,
    )
}

// fn best_action_from_values(
//     action_values: ActionValueMatrix,
//     action_mask: ActionMask,
//     player: game::PlayerID,
// ) -> Result<MoveID, EvaluationError> {
//     let mut best_action = None;
//     let mut best_action_value = f32::NEG_INFINITY;

//     for move_id in MoveID::all() {
//         if !action_mask[move_id] {
//             continue;
//         }

//         let value = action_values[move_id][player];

//         if value > best_action_value {
//             best_action = Some(move_id);
//             best_action_value = value;
//         }
//     }

//     return best_action.ok_or_else(|| EvaluationError::NoValidActions);
// }

/// Returns a full tree evaluation of the current incomplete information state.
///
/// The full tree evalution will try every single move, and then re-evaluate for that new state, to determine as exact of an action value matrix as possible.
///
/// Returns a full action value matrix, along with the valid action mask for the current player from the given state.
pub fn full_tree_evaluation<H: ActionPriorHeuristic>(
    incomplete_information_state: &game::IncompleteInformationGameState,
    search_context: &mut SearchContext<H>,
    current_depth: usize,
) -> Result<(ActionValueMatrix, ActionMask), EvaluationError> {
    let possible_worlds_and_probs = get_possible_worlds(
        &incomplete_information_state,
        search_context.config.num_worlds,
        search_context,
    );

    let mut action_value_matrix = ActionValueMatrix::zeros();
    let valid_action_matrix = ActionMask::from_hand_and_top(
        &incomplete_information_state.player_hand,
        &incomplete_information_state.trick.top_set,
    );

    // Update search statistics
    search_context.stats.full_tree_nodes_visited += 1;
    search_context.stats.total_sampled_worlds += possible_worlds_and_probs.len();
    search_context.stats.full_tree_edges_evaluated += NUM_ACTIONS * possible_worlds_and_probs.len();

    for (world, world_probability) in possible_worlds_and_probs {
        for action_id in MoveID::all() {
            // Ignore actions that the current player cannot take.
            if !valid_action_matrix[action_id] {
                continue;
            }

            let mut hypothetical_world = world.clone();
            let player_move = action_id.to_move(
                &hypothetical_world.player_hands[hypothetical_world.current_player_number],
            )?;
            match update_world(&mut hypothetical_world, player_move, search_context)? {
                Some(player_values) => {
                    // If the game has finished and we have true player values,
                    // we can just directly update the action value matrix
                    for player_id in
                        PlayerID::all_player_ids(incomplete_information_state.number_of_players)
                    {
                        action_value_matrix[action_id][player_id] +=
                            world_probability * player_values[player_id];
                    }
                }
                None => {
                    // If the game is not finished, we calculate the value
                    // matrix from the new postion, after updating the world.
                    let current_player_information = game::create_incomplete_information_game_state(
                        &hypothetical_world,
                        hypothetical_world.current_player_number,
                    );
                    // Select our evaluation method depending on depth
                    let (next_value_matrix, valid_next_actions) =
                        if current_depth + 1 >= search_context.config.full_tree_depth {
                            search_context.stats.full_tree_puct_calls += 1;
                            puct_evaluation(current_player_information, search_context)?
                        } else {
                            full_tree_evaluation(
                                &current_player_information,
                                search_context,
                                current_depth + 1,
                            )?
                        };
                    // Update the value matrix according to the likelihood of
                    // the player making various decisions, as estimated by the
                    // temperature.
                    let action_probabilities = value_to_probabilities(
                        &next_value_matrix,
                        &valid_next_actions,
                        hypothetical_world.current_player_number,
                        search_context.config.temperature,
                    );
                    for next_action_id in MoveID::all() {
                        for player_id in
                            PlayerID::all_player_ids(incomplete_information_state.number_of_players)
                        {
                            action_value_matrix[action_id][player_id] += next_value_matrix
                                [next_action_id][player_id]
                                * action_probabilities[next_action_id]
                                * world_probability;
                        }
                    }
                }
            }
        }
    }

    return Ok((action_value_matrix, valid_action_matrix));
}

/// Runs a full PUCT rollout evalution of the current incomplete information state.
///
/// The PUCT evaluation will perform a number of Monte Carlo rollouts, rather than evaluating every single action, averaging the player values of those potential game results.s
///
/// Returns a full action value matrix, along with the valid action mask for the current player from the given state.
fn puct_evaluation<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    search_context: &mut SearchContext<H>,
) -> Result<(ActionValueMatrix, ActionMask), EvaluationError> {
    let (possible_worlds, world_probabilities): (Vec<game::FullInformationGameState>, Vec<f32>) =
        get_possible_worlds(
            &incomplete_information_state,
            search_context.config.num_worlds,
            search_context,
        )
        .into_iter()
        .unzip();

    let mut action_value_matrix = ActionValueMatrix::zeros();
    let mut action_visits = [0; NUM_ACTIONS];

    // Update search stats
    search_context.stats.total_sampled_worlds += possible_worlds.len();

    // TODO: Make this an EvaluationError
    let dist =
        WeightedAliasIndex::new(world_probabilities).expect("Failed to build random world index");

    for _ in 0..search_context.config.puct_rollouts_per_leaf {
        let possible_world = &possible_worlds[dist.sample(&mut search_context.rng)];
        let (first_move, final_values) = puct_rollout(possible_world, search_context)?;
        action_visits[first_move.get()] += 1;
        for player_id in PlayerID::all_player_ids(incomplete_information_state.number_of_players) {
            action_value_matrix[first_move][player_id] += final_values[player_id];
        }
    }

    for action_id in MoveID::all() {
        if action_visits[action_id.get()] == 0 {
            continue;
        }

        for player_id in PlayerID::all_player_ids(incomplete_information_state.number_of_players) {
            action_value_matrix[action_id][player_id] /= action_visits[action_id.get()] as f32
        }
    }

    return Ok((
        action_value_matrix,
        ActionMask::from_hand_and_top(
            &incomplete_information_state.player_hand,
            &incomplete_information_state.trick.top_set,
        ),
    ));
}

fn get_possible_worlds<H: ActionPriorHeuristic>(
    incomplete_information_state: &game::IncompleteInformationGameState,
    num_worlds: usize,
    search_context: &mut SearchContext<H>,
) -> Vec<(game::FullInformationGameState, f32)> {
    debug_assert!(num_worlds > 0);

    // Needs to return normalized prob scores, currently just assuming uniform,
    // even though the distribution should be different for strong players
    let probability_score = 1.0 / num_worlds as f32;

    let mut outputs = Vec::new();

    let mut player_hands = PlayerIndexed::new(std::array::from_fn(|_| PlayerHand::empty()));
    player_hands[incomplete_information_state.perspective_player_number] =
        incomplete_information_state.player_hand.clone();

    let mut player_constraints = [0; consts::MAX_PLAYERS - 1];
    let mut compressed_index = 0;
    for original_player_id in
        PlayerID::all_player_ids(incomplete_information_state.number_of_players)
    {
        if original_player_id == incomplete_information_state.perspective_player_number {
            continue;
        }
        player_constraints[compressed_index] =
            incomplete_information_state.hand_sizes[original_player_id];
        compressed_index += 1;
    }

    let rank_constraints = incomplete_information_state
        .opponent_cards
        .to_usize_counts();

    for _ in 0..num_worlds {
        let mut world_hands = player_hands.clone();

        let generated_state = crate::world::greedy_stars_and_bars::<
            { consts::MAX_PLAYERS - 1 },
            { consts::MAX_CARD_ORDINALITY },
        >(
            player_constraints,
            rank_constraints,
            &mut search_context.rng,
        );

        let mut original_index = 0;
        for compressed_index in 0..(consts::MAX_PLAYERS - 1) {
            if PlayerID::new(original_index)
                == incomplete_information_state.perspective_player_number
            {
                original_index += 1
            }
            for card_rank in CardRank::all() {
                world_hands[PlayerID::new(original_index)][card_rank] =
                    CardCount::new(generated_state[compressed_index][card_rank.get()]);
            }
            original_index += 1;
        }

        let state = game::FullInformationGameState {
            current_player_number: incomplete_information_state.current_player_number,
            number_of_players: incomplete_information_state.number_of_players,
            player_hands: world_hands,
            player_placements: incomplete_information_state.player_placements.clone(),
            trick: incomplete_information_state.trick.clone(),
        };
        outputs.push((state, probability_score));
    }

    return outputs;
}

pub fn update_world<H: ActionPriorHeuristic>(
    world: &mut game::FullInformationGameState,
    player_move: game::Move,
    search_context: &mut SearchContext<H>,
) -> Result<Option<PlayerValues>, EvaluationError> {
    // We need to update the full information game state. Then, if we have reached the end of the round, we need to return the values for the players.
    match crate::game::update_full_information_game_state(world, player_move)? {
        true => {
            let player_values = placements_to_value(
                &world.player_placements,
                world.number_of_players,
                search_context.config.greediness,
            );
            return Ok(Some(player_values));
        }
        false => return Ok(None),
    }
}

fn placements_to_value(
    placements: &game::PlayerPlacements,
    num_players: usize,
    greediness: f32,
) -> PlayerValues {
    debug_assert!(num_players > 0);
    debug_assert!(num_players <= consts::MAX_PLAYERS);
    debug_assert!(greediness > 0.0 && greediness.is_finite());

    let mut values = PlayerValues::zeros();

    for player_id in PlayerID::all_player_ids(num_players) {
        let placement = placements[player_id];

        // placement == 0 means this player never went out.
        // At terminal time, that should mean they are the last remaining player.
        let final_place = if placement == 0 {
            num_players
        } else {
            placement
        };

        values[player_id] = place_to_value(final_place, num_players, greediness);
    }

    values
}

pub fn estimate_nonterminal_values_from_hand_sizes(
    placements: &game::PlayerPlacements,
    hand_sizes: &game::HandSizes,
    num_players: usize,
    greediness: f32,
) -> PlayerValues {
    debug_assert!(num_players > 0);
    debug_assert!(num_players <= consts::MAX_PLAYERS);
    debug_assert!(greediness > 0.0 && greediness.is_finite());

    let mut ordered_players: Vec<PlayerID> = PlayerID::all_player_ids(num_players).collect();

    ordered_players.sort_by_key(|&player_id| {
        let placement = placements[player_id];

        if placement != 0 {
            // Already-out players are ordered first by actual placement.
            (0usize, placement, 0usize, player_id.get())
        } else {
            // Unplaced players are ordered by fewer cards remaining.
            (1usize, usize::MAX, hand_sizes[player_id], player_id.get())
        }
    });

    let mut values = PlayerValues::zeros();

    for (place_index, player_id) in ordered_players.into_iter().enumerate() {
        let estimated_place = place_index + 1;
        values[player_id] = place_to_value(estimated_place, num_players, greediness);
    }

    values
}

pub fn value_to_probabilities(
    action_value_matrix: &ActionValueMatrix,
    valid_action_mask: &ActionMask,
    player: game::PlayerID,
    temperature: f32,
) -> ActionProbabilities {
    debug_assert!(temperature > 0.0);

    let mut max_value = f32::NEG_INFINITY;

    for action_id in MoveID::all() {
        if valid_action_mask[action_id] {
            max_value = max_value.max(action_value_matrix[action_id][player]);
        }
    }

    debug_assert!(
        max_value.is_finite(),
        "Cannot create action probabilities with no valid actions"
    );

    let mut probabilities = ActionProbabilities::zeros();
    let mut total = 0.0;

    for action_id in MoveID::all() {
        if !valid_action_mask[action_id] {
            continue;
        }

        let scaled = (action_value_matrix[action_id][player] - max_value) / temperature;
        let weight = scaled.exp();

        probabilities[action_id] = weight;
        total += weight;
    }

    debug_assert!(total > 0.0 && total.is_finite());

    for action_id in MoveID::all() {
        probabilities[action_id] /= total;
    }

    return probabilities;
}

fn place_to_value(final_place: usize, num_players: usize, greediness: f32) -> f32 {
    debug_assert!(final_place >= 1);
    debug_assert!(final_place <= num_players);
    debug_assert!(num_players > 0);
    debug_assert!(greediness > 0.0 && greediness.is_finite());

    if num_players == 1 {
        return 1.0;
    }

    // Convert:
    // place 1           -> 1.0
    // place num_players -> 0.0
    let linear_value = 1.0 - ((final_place - 1) as f32 / (num_players - 1) as f32);

    debug_assert!(linear_value >= 0.0);
    debug_assert!(linear_value <= 1.0);

    let value = linear_value.powf(greediness);

    debug_assert!(value.is_finite());

    value
}
pub fn ordered_player_action_values(
    action_value_matrix: &ActionValueMatrix,
    valid_action_mask: &ActionMask,
    player: game::PlayerID,
    current_hand: &game::PlayerHand,
) -> Result<Vec<(game::Move, f32)>, EvaluationError> {
    let mut action_values = Vec::new();

    for move_id in MoveID::all() {
        if !valid_action_mask[move_id] {
            continue;
        }

        let value = action_value_matrix[move_id][player];

        debug_assert!(
            value.is_finite(),
            "Non-finite action value for {:?}: {}",
            move_id,
            value,
        );

        let player_move = move_id.to_move(current_hand)?;

        action_values.push((player_move, value));
    }

    action_values.sort_by(|(move_a, value_a), (move_b, value_b)| {
        value_b
            .partial_cmp(value_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| format!("{:?}", move_a).cmp(&format!("{:?}", move_b)))
    });

    Ok(action_values)
}
