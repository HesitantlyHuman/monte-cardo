use rand_distr::weighted::WeightedAliasIndex;
use rand_distr::Distribution;
use thiserror::Error;

use crate::consts;
use crate::game;

use crate::eval::actions::{ActionMask, MoveID, MoveIDError, NUM_ACTIONS};
use crate::eval::config::{ActionPriorHeuristic, SearchConfig, SearchContext};
use crate::eval::puct::{puct_rollout, ActionProbabilities};

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct PlayerValues([f32; consts::MAX_PLAYERS]);

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ActionValueMatrix([[f32; consts::MAX_PLAYERS]; NUM_ACTIONS]);

#[derive(Error, Debug)]
pub enum EvaluationError {
    #[error("There are no valid actions from the given game state")]
    NoValidActions,
    #[error("Failed to convert to or from a MoveID: {0}")]
    MoveIDError(#[from] MoveIDError),
}

fn choose_best_action<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    heuristic: &mut H,
) -> Result<game::Move, EvaluationError> {
    debug_assert!(
        incomplete_information_state.current_player_number
            == incomplete_information_state.perspective_player_number
    );

    let search_config = SearchConfig::inference();
    let mut search_context = SearchContext::new(heuristic, search_config);
    let (action_values, action_mask) =
        full_tree_evaluation(incomplete_information_state, &mut search_context, 0)?;

    return Ok(best_action_from_values(
        action_values,
        action_mask,
        incomplete_information_state.current_player_number,
    )?
    .to_move(incomplete_information_state.player_hand)?);
}

fn best_action_from_values(
    action_values: ActionValueMatrix,
    action_mask: ActionMask,
    player: game::PlayerNumber,
) -> Result<MoveID, EvaluationError> {
    let mut best_action = None;
    let mut best_action_value = f32::NEG_INFINITY;

    for action_id in 0..NUM_ACTIONS {
        if !action_mask[action_id] {
            continue;
        }

        let value = action_values[action_id][player];

        if value > best_action_value {
            best_action = Some(action_id);
            best_action_value = value;
        }
    }

    match best_action {
        None => return Err(EvaluationError::NoValidActions),
        Some(id) => return Ok(MoveID::new(id)),
    }
}

/// Returns a full tree evaluation of the current incomplete information state.
///
/// The full tree evalution will try every single move, and then re-evaluate for that new state, to determine as exact of an action value matrix as possible.
///
/// Returns a full action value matrix, along with the valid action mask for the current player from the given state.
pub fn full_tree_evaluation<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    search_context: &mut SearchContext<H>,
    current_depth: usize,
) -> Result<(ActionValueMatrix, ActionMask), EvaluationError> {
    let possible_worlds_and_probs = get_possible_worlds(
        incomplete_information_state,
        search_context.config.num_worlds,
        search_context,
    );

    let mut action_value_matrix = [[0.0; consts::MAX_PLAYERS]; NUM_ACTIONS];
    let valid_action_matrix = ActionMask::from_incomplete_information(incomplete_information_state);

    for (world, world_probability) in possible_worlds_and_probs {
        for action_id in 0..NUM_ACTIONS {
            let action_id = MoveID::new(action_id);

            // Ignore actions that the current player cannot take.
            if !valid_action_matrix[action_id.get()] {
                continue;
            }

            let mut hypothetical_world = world.clone();
            let player_hand =
                hypothetical_world.player_hands[hypothetical_world.current_player_number];
            match update_world(
                &mut hypothetical_world,
                action_id.to_move(player_hand)?,
                search_context,
            ) {
                Some(player_values) => {
                    // If the game has finished and we have true player values,
                    // we can just directly update the action value matrix
                    for player_id in 0..consts::MAX_PLAYERS {
                        action_value_matrix[action_id.get()][player_id] +=
                            world_probability * player_values[player_id];
                    }
                }
                None => {
                    // If the game is not finished, we calculate the value
                    // matrix from the new postion, after updating the world.
                    let current_player_information = game::create_incomplete_information_game_state(
                        hypothetical_world,
                        hypothetical_world.current_player_number,
                    );
                    // Select our evaluation method depending on depth
                    let (next_value_matrix, valid_next_actions) =
                        if current_depth + 1 >= search_context.config.full_tree_depth {
                            puct_evaluation(current_player_information, search_context)?
                        } else {
                            full_tree_evaluation(
                                current_player_information,
                                search_context,
                                current_depth + 1,
                            )?
                        };
                    // Update the value matrix according to the likelihood of
                    // the player making various decisions, as estimated by the
                    // temperature.
                    let action_probabilities = value_to_probabilities(
                        next_value_matrix,
                        valid_next_actions,
                        hypothetical_world.current_player_number,
                        search_context.config.temperature,
                    );
                    for next_action_id in 0..NUM_ACTIONS {
                        for player_id in 0..consts::MAX_PLAYERS {
                            action_value_matrix[action_id.get()][player_id] += next_value_matrix
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
            incomplete_information_state,
            search_context.config.num_worlds,
            search_context,
        )
        .into_iter()
        .unzip();

    let mut action_value_matrix = [[0.0; consts::MAX_PLAYERS]; NUM_ACTIONS];
    let mut action_visits = [0; NUM_ACTIONS];

    // TODO: would it be a good idea to put the rng in the search context?
    let dist =
        WeightedAliasIndex::new(world_probabilities).expect("Failed to build random world index");

    for _ in 0..search_context.config.puct_rollouts_per_leaf {
        let possible_world = possible_worlds[dist.sample(&mut search_context.rng)];
        let (first_move, final_values) = puct_rollout(possible_world, search_context);
        action_visits[first_move.get()] += 1;
        for player_index in 0..consts::MAX_PLAYERS {
            action_value_matrix[first_move.get()][player_index] += final_values[player_index];
        }
    }

    for action_id in 0..NUM_ACTIONS {
        if action_visits[action_id] == 0 {
            continue;
        }

        for player_index in 0..consts::MAX_PLAYERS {
            action_value_matrix[action_id][player_index] /= action_visits[action_id] as f32
        }
    }

    return Ok((
        action_value_matrix,
        ActionMask::from_incomplete_information(incomplete_information_state),
    ));
}

fn get_possible_worlds<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    num_worlds: usize,
    search_context: &mut SearchContext<H>,
) -> Vec<(game::FullInformationGameState, f32)> {
    debug_assert!(num_worlds > 0);

    // Needs to return normalized prob scores, currently just assuming uniform,
    // even though the distribution should be different for strong players
    let probability_score = 1.0 / num_worlds as f32;

    let mut outputs = Vec::new();

    let mut player_hands = [[0; consts::MAX_CARD_ORDINALITY]; consts::MAX_PLAYERS];
    player_hands[incomplete_information_state.perspective_player_number] =
        incomplete_information_state.player_hand;

    let mut player_constraints = [0; consts::MAX_PLAYERS - 1];
    let mut compressed_index = 0;
    for original_index in 0..incomplete_information_state.number_of_players {
        if original_index == incomplete_information_state.perspective_player_number {
            continue;
        }
        player_constraints[compressed_index] =
            incomplete_information_state.hand_sizes[original_index];
        compressed_index += 1;
    }

    let rank_constraints = incomplete_information_state.opponent_cards;

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
            if original_index == incomplete_information_state.perspective_player_number {
                original_index += 1
            }
            for card_rank in 0..consts::MAX_CARD_ORDINALITY {
                world_hands[original_index][card_rank] =
                    generated_state[compressed_index][card_rank];
            }
            original_index += 1;
        }

        let state = game::FullInformationGameState {
            current_player_number: incomplete_information_state.current_player_number,
            number_of_players: incomplete_information_state.number_of_players,
            player_hands: world_hands,
            player_finish_order: incomplete_information_state.player_finish_order,
            trick: incomplete_information_state.trick,
        };
        outputs.push((state, probability_score));
    }

    return outputs;
}

pub fn update_world<H: ActionPriorHeuristic>(
    world: &mut game::FullInformationGameState,
    player_move: game::Move,
    search_context: &mut SearchContext<H>,
) -> Option<PlayerValues> {
    // We need to update the full information game state. Then, if we have reached the end of the round, we need to return the values for the players.
    match crate::game::update_full_information_game_state(world, &player_move) {
        true => {
            let player_values = placements_to_value(
                world.player_finish_order,
                world.number_of_players,
                search_context.config.greediness,
            );
            return Some(player_values);
        }
        false => return None,
    }
}

fn placements_to_value(
    placements: [usize; consts::MAX_PLAYERS],
    num_players: usize,
    greediness: f32,
) -> PlayerValues {
    let mut values = [0.0; consts::MAX_PLAYERS];
    for active_player in 0..num_players {
        values[active_player] =
            (1.0 - (placements[active_player] as f32 / (num_players - 1) as f32)).powf(greediness);
    }
    return values;
}

pub fn value_to_probabilities(
    action_value_matrix: ActionValueMatrix,
    valid_action_mask: ActionMask,
    player: game::PlayerNumber,
    temperature: f32,
) -> ActionProbabilities {
    debug_assert!(temperature > 0.0);

    let mut max_value = f32::NEG_INFINITY;

    for action_id in 0..NUM_ACTIONS {
        if valid_action_mask[action_id] {
            max_value = max_value.max(action_value_matrix[action_id][player]);
        }
    }

    debug_assert!(
        max_value.is_finite(),
        "Cannot create action probabilities with no valid actions"
    );

    let mut probabilities = [0.0; NUM_ACTIONS];
    let mut total = 0.0;

    for action_id in 0..NUM_ACTIONS {
        if !valid_action_mask[action_id] {
            continue;
        }

        let scaled = (action_value_matrix[action_id][player] - max_value) / temperature;
        let weight = scaled.exp();

        probabilities[action_id] = weight;
        total += weight;
    }

    debug_assert!(total > 0.0 && total.is_finite());

    for action_id in 0..NUM_ACTIONS {
        probabilities[action_id] /= total;
    }

    probabilities
}
