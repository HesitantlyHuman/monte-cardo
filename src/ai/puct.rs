use core::num;
use std::collections::HashMap;

use crate::ai::game;
use crate::consts;

const NUM_ACTIONS: usize = 1 + consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY * 2; // The assumption is that we only consider playing the minimum number of wilds. Using all the wilds and all of the cards the max you could play in one go is consts::MAX_CARD_NUMBER * 2

const SWAP_DEPTH: usize = 4;
const TEMPERATURE: f32 = 2.0;

type PlayerValues = [f32; consts::MAX_PLAYERS];
type ActionValueMatrix = [PlayerValues; NUM_ACTIONS];
type ActionProbabilties = [f32; NUM_ACTIONS];
type ActionMask = [bool; NUM_ACTIONS];
type MoveID = usize;

struct SearchConfig {
    full_tree_depth: usize,
    num_worlds: usize,
    puct_rollouts_per_leaf: usize,
    exploration_factor: f32,
    temperature: f32,
}

struct SearchContext<H: ActionPriorHeuristic> {
    heuristic: H,
    nodes: HashMap<NormalizedIncompleteInformation, PUCTNode>,
    config: SearchConfig,
}

trait ActionPriorHeuristic {
    fn action_priors(&mut self, state: NormalizedIncompleteInformation) -> ActionProbabilties;
}

fn value_to_probabilities(
    action_value_matrix: ActionValueMatrix,
    player: game::PlayerNumber,
) -> ActionProbabilties {
    [0.0; NUM_ACTIONS]
}

fn move_to_id(game_move: game::Move) -> MoveID {
    match game_move {
        game::Move::Pass => 0,
        game::Move::Play(game::Play {
            rank,
            num_non_wilds,
            num_wilds,
        }) => consts::MAX_CARD_ORDINALITY * (num_non_wilds + num_wilds - 1) as usize + rank + 1,
    }
}

fn id_to_move(id: MoveID, hand: game::Hand) -> game::Move {
    if id == 0 {
        return game::Move::Pass;
    }

    let play_id = id - 1;
    let (num_to_play, rank_to_play) = (
        play_id / (consts::MAX_CARD_ORDINALITY) + 1,
        play_id % (consts::MAX_CARD_ORDINALITY),
    );

    let available_to_play = hand[rank_to_play as usize];
    let wilds = hand[0];
    debug_assert!(available_to_play + wilds >= num_to_play as u8);
    let wilds_to_use = (num_to_play as u8).saturating_sub(available_to_play);
    let non_wilds_to_use = num_to_play as u8 - wilds_to_use;

    return game::Move::Play(game::Play {
        rank: rank_to_play,
        num_non_wilds: non_wilds_to_use,
        num_wilds: wilds_to_use,
    });
}

// TODO: This probably needs to know the valid actions
fn get_best_action(action_value_matrix: ActionValueMatrix) -> usize {
    // let mut best_action = None;
    let mut best_value = f32::NEG_INFINITY;

    for action_id in 0..NUM_ACTIONS {}
    0
}

fn choose_action<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    heuristic: &mut H,
) -> game::Move {
    game::Move::Pass
}

fn full_tree_evaluation<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    search_context: SearchContext<H>,
    current_depth: usize,
) -> ActionValueMatrix {
    debug_assert_eq!(
        incomplete_information_state.current_player_number,
        incomplete_information_state.perspective_player_number
    );
    [[0.0; consts::MAX_PLAYERS]; NUM_ACTIONS]
}

#[derive(Debug, Clone, Copy, Hash)]
struct NormalizedIncompleteInformation {
    player_hand: [u8; consts::MAX_CARD_ORDINALITY],
    opponent_cards: [u8; consts::MAX_CARD_ORDINALITY],
    hand_sizes: [u16; consts::MAX_PLAYERS],
    trick: game::Trick,
}

#[derive(Debug, Clone)]
struct TrainingExample {
    state: NormalizedIncompleteInformation,
    action_probabilities: ActionProbabilties,
    action_mask: ActionMask,
}

// TODO: this needs to rotate the incomplete information state so that we have a canonical representation (0 is the current player)
// We also should rename this, because that output will be the input for the heuristic
fn get_puct_key(
    incomplete_information_state: game::IncompleteInformationGameState,
) -> NormalizedIncompleteInformation {
    NormalizedIncompleteInformation {
        player_hand: incomplete_information_state.player_hand,
        opponent_cards: incomplete_information_state.opponent_cards,
        hand_sizes: incomplete_information_state.hand_sizes,
        trick: incomplete_information_state.trick,
    }
}

struct PUCTNode {
    action_mask: ActionMask,
    action_priors: ActionProbabilties,
    visit_weights: [f32; NUM_ACTIONS],
    accumulated_values: ActionValueMatrix,
}

fn puct_score(node: &PUCTNode, action_id: usize, exploration_factor: f32) -> f32 {
    // Considered from the perspective of 0 being the active player
    let n_action_taken = node.visit_weights[action_id];
    let n_times_at_node: f32 = node.visit_weights.iter().sum();

    let q_term = if n_action_taken == 0.0 {
        0.0
    } else {
        node.accumulated_values[action_id][0] / n_times_at_node
    };

    let prior = node.action_priors[action_id];
    let exploration_term =
        exploration_factor * prior * (n_times_at_node.sqrt() / (1.0 + n_action_taken));

    return q_term + exploration_term;
}

fn select_puct_action(node: &PUCTNode, exploration_factor: f32) -> usize {
    let mut best_action = None;
    let mut best_score = f32::NEG_INFINITY;

    for action_id in 0..NUM_ACTIONS {
        if !node.action_mask[action_id] {
            continue;
        }

        let score = puct_score(node, action_id, exploration_factor);

        if score > best_score {
            best_action = Some(action_id);
            best_score = score;
        }
    }

    best_action.expect("PUCTNode has no valid actions!")
}

fn puct_rollout<H: ActionPriorHeuristic>(
    world: game::FullInformationGameState,
    world_probability: f32,
    search_context: &mut SearchContext<H>,
) -> (MoveID, PlayerValues) {
    (0, [0.0; consts::MAX_PLAYERS])
}

fn puct_evalution<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    search_context: &mut SearchContext<H>,
) -> ActionValueMatrix {
    let possible_worlds_and_scores = get_possible_worlds(incomplete_information_state);

    let mut action_value_matrix = [[0.0; consts::MAX_PLAYERS]; NUM_ACTIONS];
    let mut total_probability_mass = [0.0; NUM_ACTIONS];
    for (possible_world, probability) in possible_worlds_and_scores {
        let (first_move, final_values) = puct_rollout(possible_world, probability, search_context);
        total_probability_mass[first_move] += probability;
        for player_index in 0..consts::MAX_PLAYERS {
            action_value_matrix[first_move][player_index] +=
                probability * final_values[player_index];
        }
    }

    for action_id in 0..NUM_ACTIONS {
        for player_index in 0..consts::MAX_PLAYERS {
            action_value_matrix[action_id][player_index] /= total_probability_mass[action_id]
        }
    }

    return action_value_matrix;
}

fn get_possible_worlds(
    incomplete_information_state: game::IncompleteInformationGameState,
) -> Vec<(game::FullInformationGameState, f32)> {
    // Should return normalized probability scores
    Vec::new()
}

fn update_world(world: game::FullInformationGameState) {}
