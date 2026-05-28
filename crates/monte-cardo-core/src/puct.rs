use rand::rngs::SmallRng;
use rand::SeedableRng;
use rand_distr::weighted::WeightedAliasIndex;
use rand_distr::Distribution;
use std::collections::HashMap;
use std::hash::Hash;

use crate::consts;
use crate::game::{self, FullInformationGameState, IncompleteInformationGameState};

const NUM_ACTIONS: usize = 1 + consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY * 2; // The assumption is that we only consider playing the minimum number of wilds. Using all the wilds and all of the cards the max you could play in one go is consts::MAX_CARD_NUMBER * 2

type PlayerValues = [f32; consts::MAX_PLAYERS];
type ActionValueMatrix = [PlayerValues; NUM_ACTIONS];
type ActionProbabilities = [f32; NUM_ACTIONS];
type ActionMask = [bool; NUM_ACTIONS];
type MoveID = usize;

struct SearchConfig {
    full_tree_depth: usize,
    num_worlds: usize,
    puct_rollouts_per_leaf: usize,
    exploration_factor: f32,
    temperature: f32,
}

impl SearchConfig {
    fn inference() -> Self {
        Self {
            full_tree_depth: 4,
            num_worlds: 100,
            puct_rollouts_per_leaf: 200,
            exploration_factor: 2.0,
            temperature: 0.1,
        }
    }

    fn training(temperature_schedule: f32) -> Self {
        Self {
            full_tree_depth: 4,
            num_worlds: 100,
            puct_rollouts_per_leaf: 200,
            exploration_factor: 2.0,
            temperature: temperature_schedule,
        }
    }
}

struct SearchContext<'a, H: ActionPriorHeuristic> {
    heuristic: &'a mut H,
    nodes: HashMap<NormalizedIncompleteInformation, PUCTNode>,
    config: SearchConfig,
    rng: SmallRng,
}

impl<'a, H: ActionPriorHeuristic> SearchContext<'a, H> {
    fn new(heuristic: &'a mut H, config: SearchConfig) -> Self {
        Self {
            heuristic: heuristic,
            nodes: HashMap::new(),
            config: config,
            rng: SmallRng::seed_from_u64(42),
        }
    }
}

trait ActionPriorHeuristic {
    fn action_priors(&mut self, state: NormalizedIncompleteInformation) -> ActionProbabilities;
}

fn value_to_probabilities(
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

fn move_to_id(game_move: game::Move) -> MoveID {
    match game_move {
        game::Move::Pass => 0,
        game::Move::Play(game::Play {
            rank,
            num_non_wilds,
            num_wilds,
        }) => {
            let total_num_cards = num_non_wilds + num_wilds;
            debug_assert!(total_num_cards > 0);
            consts::MAX_CARD_ORDINALITY * (total_num_cards - 1) as usize + rank + 1
        }
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

fn best_action_from_values(
    action_values: ActionValueMatrix,
    action_mask: ActionMask,
    player: game::PlayerNumber,
) -> MoveID {
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

    best_action.expect("No valid actions found!")
}

// TODO: Maybe validate that the current player and perspective player are the same
fn choose_best_action<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    heuristic: &mut H,
) -> game::Move {
    let search_config = SearchConfig::inference();
    let mut search_context = SearchContext::new(heuristic, search_config);
    let (action_values, action_mask) =
        full_tree_evaluation(incomplete_information_state, &mut search_context, 0);

    return id_to_move(
        best_action_from_values(
            action_values,
            action_mask,
            incomplete_information_state.current_player_number,
        ),
        incomplete_information_state.player_hand,
    );
}

fn generate_training_example<H: ActionPriorHeuristic>(
    incomplete_information_state: IncompleteInformationGameState,
    heuristic: &mut H,
    temperature_schedule: f32,
) -> (game::Move, TrainingExample) {
    let search_config = SearchConfig::training(temperature_schedule);
    let mut search_context = SearchContext::new(heuristic, search_config);
    let (action_values, action_mask) =
        full_tree_evaluation(incomplete_information_state, &mut search_context, 0);

    let selected_move = id_to_move(
        best_action_from_values(
            action_values,
            action_mask,
            incomplete_information_state.current_player_number,
        ),
        incomplete_information_state.player_hand,
    );
    let action_probabilities = value_to_probabilities(
        action_values,
        action_mask,
        incomplete_information_state.current_player_number,
        temperature_schedule,
    );
    return (
        selected_move,
        TrainingExample {
            state: normalize_incomplete_information_state(incomplete_information_state),
            action_probabilities: action_probabilities,
            action_mask: action_mask,
        },
    );
}

fn full_tree_evaluation<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    search_context: &mut SearchContext<H>,
    current_depth: usize,
) -> (ActionValueMatrix, ActionMask) {
    let possible_worlds_and_probs = get_possible_worlds(
        incomplete_information_state,
        search_context.config.num_worlds,
        search_context,
    );

    let mut action_value_matrix = [[0.0; consts::MAX_PLAYERS]; NUM_ACTIONS];
    let valid_action_matrix = create_valid_action_mask(incomplete_information_state);

    for (world, probability) in possible_worlds_and_probs {
        for action_id in 0..NUM_ACTIONS {
            if !valid_action_matrix[action_id] {
                continue;
            }

            let mut hypothetical_world = world.clone();
            let player_hand =
                hypothetical_world.player_hands[hypothetical_world.current_player_number];
            match update_world(&mut hypothetical_world, id_to_move(action_id, player_hand)) {
                Some(player_values) => {
                    // Just directly update the action value matrix
                    for player_id in 0..consts::MAX_PLAYERS {
                        action_value_matrix[action_id][player_id] +=
                            probability * player_values[player_id];
                    }
                }
                None => {
                    // Calculate the value matrix from this position
                    let current_player_information = game::create_incomplete_information_game_state(
                        hypothetical_world,
                        hypothetical_world.current_player_number,
                    );
                    let (next_value_matrix, valid_next_actions) =
                        if current_depth >= search_context.config.full_tree_depth {
                            puct_evalution(current_player_information, search_context)
                        } else {
                            full_tree_evaluation(
                                current_player_information,
                                search_context,
                                current_depth + 1,
                            )
                        };
                    // Update the value matrix according to the likelihood of
                    // the player making various decisions, as estimated by the
                    // temperature.
                    let probabilities = value_to_probabilities(
                        next_value_matrix,
                        valid_next_actions,
                        hypothetical_world.current_player_number,
                        search_context.config.temperature,
                    );
                    for next_action_id in 0..NUM_ACTIONS {
                        for player_id in 0..consts::MAX_PLAYERS {
                            action_value_matrix[action_id][player_id] += next_value_matrix
                                [next_action_id][player_id]
                                * probabilities[next_action_id]
                                * probability;
                        }
                    }
                }
            }
        }
    }

    return (action_value_matrix, valid_action_matrix);
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct NormalizedIncompleteInformation {
    player_hand: [u8; consts::MAX_CARD_ORDINALITY],
    opponent_cards: [u8; consts::MAX_CARD_ORDINALITY],
    hand_sizes: [u16; consts::MAX_PLAYERS],
    trick: game::Trick,
}

#[derive(Debug, Clone)]
struct TrainingExample {
    state: NormalizedIncompleteInformation,
    action_probabilities: ActionProbabilities,
    action_mask: ActionMask,
}

// TODO: this needs to rotate the incomplete information state so that we have a canonical representation (0 is the current player)
fn normalize_incomplete_information_state(
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
            accumulated_values: [[0.0; consts::MAX_PLAYERS]; NUM_ACTIONS],
        }
    }
}

fn puct_score(node: &PUCTNode, action_id: usize, player_id: usize, exploration_factor: f32) -> f32 {
    let n_action_taken = node.visit_counts[action_id];
    let n_times_at_node: u32 = node.visit_counts.iter().sum();

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

fn select_puct_action(node: &PUCTNode, player_id: usize, exploration_factor: f32) -> usize {
    let mut best_action = None;
    let mut best_score = f32::NEG_INFINITY;

    for action_id in 0..NUM_ACTIONS {
        if !node.action_mask[action_id] {
            continue;
        }

        let score = puct_score(node, action_id, player_id, exploration_factor);

        if score > best_score {
            best_action = Some(action_id);
            best_score = score;
        }
    }

    best_action.expect("PUCTNode has no valid actions!")
}

fn create_valid_action_mask(incomplete_information: IncompleteInformationGameState) -> ActionMask {
    let mut valid_action_mask = [false; NUM_ACTIONS];
    for available_move in game::get_available_moves(
        incomplete_information.player_hand,
        incomplete_information.trick.top_set,
    ) {
        valid_action_mask[move_to_id(available_move)] = true;
    }
    return valid_action_mask;
}

fn create_search_node<H: ActionPriorHeuristic>(
    incomplete_information: IncompleteInformationGameState,
    normalized_information_state: NormalizedIncompleteInformation,
    heuristic: &mut H,
) -> PUCTNode {
    let action_priors = heuristic.action_priors(normalized_information_state);
    let valid_action_mask = create_valid_action_mask(incomplete_information);
    let mut normalized_action_priors = [0.0; NUM_ACTIONS];
    let mut unmasked_sum = 0.0;
    for (idx, (prior, is_valid)) in action_priors.iter().zip(valid_action_mask).enumerate() {
        if !is_valid {
            continue;
        }

        normalized_action_priors[idx] = *prior;
        unmasked_sum += prior;
    }

    if unmasked_sum <= 0.0 {
        let num_valid = valid_action_mask.iter().filter(|&&x| x).count();
        debug_assert!(num_valid > 0);

        for idx in 0..NUM_ACTIONS {
            if valid_action_mask[idx] {
                normalized_action_priors[idx] = 1.0 / num_valid as f32;
            }
        }
    } else {
        for idx in 0..NUM_ACTIONS {
            normalized_action_priors[idx] /= unmasked_sum;
        }
    }

    return PUCTNode::new(valid_action_mask, normalized_action_priors);
}

fn puct_rollout<H: ActionPriorHeuristic>(
    world: game::FullInformationGameState,
    search_context: &mut SearchContext<H>,
) -> (MoveID, PlayerValues) {
    // Worst case is playing one card per turn
    // Should theoretically multiply by the number of players, because every
    // player other than the first can pass, but we will assume that these
    // players are generally more efficient than that.
    let mut world = world.clone();
    let mut first_action = None;
    let mut nodes_to_update = Vec::new();
    for _ in 0..(consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY * 2) {
        let current_player_information =
            game::create_incomplete_information_game_state(world, world.current_player_number);
        let normalized_player_information =
            normalize_incomplete_information_state(current_player_information);
        let search_node = search_context
            .nodes
            .entry(normalized_player_information)
            .or_insert_with(|| {
                create_search_node(
                    current_player_information,
                    normalized_player_information,
                    search_context.heuristic,
                )
            });
        let selected_action = select_puct_action(
            search_node,
            current_player_information.current_player_number,
            search_context.config.exploration_factor,
        );

        nodes_to_update.push((selected_action, normalized_player_information));
        let action_move = id_to_move(selected_action, current_player_information.player_hand);
        if first_action.is_none() {
            first_action = Some(selected_action);
        }
        match update_world(&mut world, action_move) {
            Some(player_values) => {
                // First update the stats for all the nodes we visited
                for (selected_action, key) in nodes_to_update {
                    if let Some(search_node) = search_context.nodes.get_mut(&key) {
                        for player_id in 0..consts::MAX_PLAYERS {
                            search_node.accumulated_values[selected_action][player_id] +=
                                player_values[player_id]
                        }
                        search_node.visit_counts[selected_action] += 1;
                    }
                }
                // Then return!
                return (
                    first_action.expect("Tried to rollout a finished game!"),
                    player_values,
                );
            }
            None => {}
        }
    }

    panic!("Rollout failed to finish!");
}

fn puct_evalution<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    search_context: &mut SearchContext<H>,
) -> (ActionValueMatrix, ActionMask) {
    let (possible_worlds, world_probabilities): (Vec<FullInformationGameState>, Vec<f32>) =
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
        action_visits[first_move] += 1;
        for player_index in 0..consts::MAX_PLAYERS {
            action_value_matrix[first_move][player_index] += final_values[player_index];
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

    return (
        action_value_matrix,
        create_valid_action_mask(incomplete_information_state),
    );
}

fn get_possible_worlds<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    num_worlds: usize,
    search_context: &mut SearchContext<H>,
) -> Vec<(game::FullInformationGameState, f32)> {
    // Needs to return normalized prob scores
    Vec::new()
}

fn update_world(
    world: &mut game::FullInformationGameState,
    player_move: game::Move,
) -> Option<PlayerValues> {
    None
}
