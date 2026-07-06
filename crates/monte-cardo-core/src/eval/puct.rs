use std::ops::Index;
use std::ops::IndexMut;

use smallvec::SmallVec;

use crate::consts;
use crate::eval::actions::{MoveID, NUM_ACTIONS};
use crate::eval::config::{ActionPriorHeuristic, SearchContext};
use crate::eval::evaluate::EvaluationError;
use crate::eval::evaluate::{update_world, PlayerValues};
use crate::eval::normalize::{
    normalize_incomplete_information_state, NormalizedIncompleteInformation, RankCompressible,
    RankCompressionMap,
};
use crate::eval::zobrist::ZobristHash;
use crate::eval::NormalizationError;
use crate::eval::RankCompressed;
use crate::game;
use crate::game::PlayerID;

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

impl RankCompressible for ActionProbabilities {
    fn rank_compress(
        &self,
        rank_compression_map: &RankCompressionMap,
    ) -> Result<RankCompressed<Self>, super::NormalizationError> {
        let mut compressed_action_probabilities = ActionProbabilities::zeros();

        for uncompressed_move_id in MoveID::all() {
            match uncompressed_move_id.rank_compress(rank_compression_map) {
                Ok(compressed_id) => {
                    compressed_action_probabilities[*compressed_id.inner()] =
                        self[uncompressed_move_id]
                }
                Err(_) => {}
            }
        }

        return Ok(RankCompressed::new_unchecked(
            compressed_action_probabilities,
        ));
    }

    fn rank_decompress(
        compressed: &RankCompressed<Self>,
        rank_compression_map: &RankCompressionMap,
    ) -> Result<Self, super::NormalizationError> {
        let mut decompressed_action_probabilities = ActionProbabilities::zeros();

        for compressed_move_id in MoveID::all() {
            let compressed_move_id = RankCompressed::new_unchecked(compressed_move_id);
            match MoveID::rank_decompress(&compressed_move_id, rank_compression_map) {
                Ok(decompressed_id) => {
                    decompressed_action_probabilities[decompressed_id] =
                        compressed.inner()[*compressed_move_id.inner()];
                }
                Err(_) => {}
            }
        }

        return Ok(decompressed_action_probabilities);
    }
}

pub struct PUCTAction {
    action: RankCompressed<MoveID>,
    prior: f32,
    visit_count: usize,
    accumulated_value: f32,
}

const PUCT_STACK_SIZE: usize = 4;

pub struct PUCTNode {
    actions: SmallVec<[PUCTAction; PUCT_STACK_SIZE]>,
    times_at_node: usize,
}

impl PUCTNode {
    fn new(actions: SmallVec<[PUCTAction; PUCT_STACK_SIZE]>) -> Self {
        PUCTNode {
            actions: actions,
            times_at_node: 0,
        }
    }
}

impl Index<&RankCompressed<MoveID>> for RankCompressed<ActionProbabilities> {
    type Output = f32;

    fn index(&self, index: &RankCompressed<MoveID>) -> &Self::Output {
        return &self.inner()[*index.inner()];
    }
}

impl IndexMut<&RankCompressed<MoveID>> for RankCompressed<ActionProbabilities> {
    fn index_mut(&mut self, index: &RankCompressed<MoveID>) -> &mut Self::Output {
        return &mut self.inner_mut()[*index.inner()];
    }
}

fn puct_score(puct_action: &PUCTAction, times_at_node: usize, exploration_factor: f32) -> f32 {
    let q_term = if puct_action.visit_count == 0 {
        0.0
    } else {
        puct_action.accumulated_value / puct_action.visit_count as f32
    };

    let exploration_term = exploration_factor
        * puct_action.prior
        * ((times_at_node as f32).sqrt() / (1.0 + puct_action.visit_count as f32));

    return q_term + exploration_term;
}
fn select_puct_action(
    node: &PUCTNode,
    rank_compression_map: &RankCompressionMap,
    exploration_factor: f32,
) -> Result<(MoveID, usize), EvaluationError> {
    debug_assert!(node.actions.len() > 0);

    let mut best_action = None;
    let mut best_score = f32::NEG_INFINITY;

    for (puct_action_index, puct_action) in node.actions.iter().enumerate() {
        let score = puct_score(puct_action, node.times_at_node, exploration_factor);

        debug_assert!(score.is_finite());

        if score > best_score {
            best_action = Some((puct_action.action.clone(), puct_action_index));
            best_score = score;
        }
    }

    let (compressed_action, action_index) = best_action.expect("PUCTNode has no valid actions!");

    let concrete_action = MoveID::rank_decompress(&compressed_action, rank_compression_map)
        .map_err(|err| {
            EvaluationError::NormalizationError(NormalizationError::RankDecompressionError(
                format!("PUCT selected an invalid compressed action: {}", err,),
            ))
        })?;

    return Ok((concrete_action, action_index));
}

fn update_puct_node(
    node: &mut PUCTNode,
    player_values: &PlayerValues,
    action_index: usize,
    player_at_time: game::PlayerID,
) -> Result<(), EvaluationError> {
    debug_assert!(
        player_values.get().iter().all(|value| value.is_finite()),
        "Non-finite PlayerValues before rotation: {:?}",
        player_values.get(),
    );

    node.actions[action_index].accumulated_value += player_values[player_at_time];

    node.actions[action_index].visit_count += 1;
    node.times_at_node += 1;

    return Ok(());
}

fn create_search_node<H: ActionPriorHeuristic>(
    incomplete_information: &game::IncompleteInformationGameState,
    normalized_information_state: &NormalizedIncompleteInformation,
    rank_compression_map: &RankCompressionMap,
    heuristic: &mut H,
) -> Result<PUCTNode, EvaluationError> {
    let action_priors = heuristic.action_priors(normalized_information_state);

    let mut valid_actions = Vec::new();
    for available_move in game::get_available_moves(
        &incomplete_information.player_hand,
        &incomplete_information.trick.top_set,
    ) {
        let move_id = MoveID::from_move(&available_move)?;
        let compressed_move_id = match move_id.rank_compress(rank_compression_map) {
            Ok(compressed) => compressed,
            Err(_) => {
                continue;
            } // This can happen if the player has wilds. We may have compressed the rank away, but it is technically still possible to play. However, we will assume that it would be functionally equivalent to simply play something higher that is still in the allowed ranks.
        };
        valid_actions.push(compressed_move_id);
    }

    let mut normalized_action_priors = RankCompressed::new_unchecked(ActionProbabilities::zeros());
    let mut unmasked_sum = 0.0;
    for compressed_move_id in &valid_actions {
        let prior = action_priors[&compressed_move_id];

        normalized_action_priors[&compressed_move_id] = prior;
        unmasked_sum += prior;
    }

    if unmasked_sum <= 0.0 {
        let num_valid = valid_actions.len();
        debug_assert!(num_valid > 0);

        for compressed_move_id in &valid_actions {
            if valid_actions.contains(&compressed_move_id) {
                normalized_action_priors[&compressed_move_id] = 1.0 / num_valid as f32;
            }
        }
    } else {
        for compressed_move_id in &valid_actions {
            normalized_action_priors[&compressed_move_id] /= unmasked_sum;
        }
    }

    let mut actions = SmallVec::new();

    for compressed_move_id in valid_actions {
        actions.push(PUCTAction {
            action: compressed_move_id.clone(),
            prior: action_priors[&compressed_move_id],
            visit_count: 0,
            accumulated_value: 0.0,
        });
    }

    return Ok(PUCTNode::new(actions));
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
    // Update search statistics
    search_context.stats.puct_num_rollouts += 1;

    let mut world = world.clone();
    let mut first_action = None;
    let mut nodes_to_update_on_return =
        Vec::with_capacity(consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY * 2);

    // Worst case is playing one card per turn
    // Should theoretically multiply by the number of players, because every
    // player other than the first can pass, but we will assume that these
    // players are generally more efficient than that.
    for _ in 0..(consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY * 2) {
        match puct_rollout_step(
            &mut world,
            search_context,
            &mut nodes_to_update_on_return,
            &mut first_action,
        )? {
            Some(output) => return Ok(output),
            None => {}
        };
    }

    return Err(EvaluationError::RolloutError);
}

pub fn puct_rollout_step<H: ActionPriorHeuristic>(
    world: &mut game::FullInformationGameState,
    search_context: &mut SearchContext<H>,
    nodes_to_update: &mut Vec<(usize, PlayerID, ZobristHash)>,
    first_action: &mut Option<MoveID>,
) -> Result<Option<(MoveID, PlayerValues)>, EvaluationError> {
    // Update node stat
    search_context.stats.puct_nodes_visited += 1;

    let current_player_information =
        game::create_incomplete_information_game_state(&world, world.current_player_number);
    let (normalized_player_information, rank_compression_map): (
        NormalizedIncompleteInformation,
        RankCompressionMap,
    ) = normalize_incomplete_information_state(&current_player_information)?;
    let zobrist_hash = search_context
        .zobrist_hash
        .hash(&normalized_player_information);

    let (selected_action, selected_action_index) = {
        let exploration_factor = search_context.config.exploration_factor;
        let heuristic = &mut *search_context.heuristic;
        let nodes = &mut search_context.puct_nodes;

        let search_node = nodes
            .get_or_insert_with(&zobrist_hash, || {
                search_context.stats.puct_nodes_created += 1;
                Ok::<Box<PUCTNode>, EvaluationError>(Box::new(create_search_node(
                    &current_player_information,
                    &normalized_player_information,
                    &rank_compression_map,
                    heuristic,
                )?))
            })?
            .expect("PUCT node was not admitted into cache");

        search_context.stats.puct_valid_actions_seen += search_node.actions.len();

        select_puct_action(
            search_node.as_ref(),
            &rank_compression_map,
            exploration_factor,
        )?
    };

    nodes_to_update.push((
        selected_action_index,
        world.current_player_number,
        zobrist_hash,
    ));

    let action_move = selected_action.to_move(&current_player_information.player_hand)?;
    if first_action.is_none() {
        *first_action = Some(selected_action);
    }
    match update_world(world, action_move, search_context)? {
        Some(player_values) => {
            puct_backprop(&player_values, &nodes_to_update, search_context)?;

            return Ok(Some((
                first_action.expect("Tried to rollout a finished game!"),
                player_values,
            )));
        }
        None => return Ok(None),
    }
}

pub fn puct_backprop<H: ActionPriorHeuristic>(
    player_values: &PlayerValues,
    nodes_to_update: &[(usize, PlayerID, ZobristHash)],
    search_context: &mut SearchContext<H>,
) -> Result<(), EvaluationError> {
    for (selected_action, player_at_time, key) in nodes_to_update {
        if let Some(mut search_node) = search_context.puct_nodes.get_mut(key) {
            update_puct_node(
                search_node.as_mut(),
                &player_values,
                *selected_action,
                *player_at_time,
            )?;
        }
    }
    return Ok(());
}
