use rand::rngs::SmallRng;
use rand_distr::weighted::WeightedAliasIndex;
use rand_distr::Distribution;

use crate::game;

use crate::eval::actions::{ActionMask, MoveID, MoveIDError, NUM_ACTIONS};
use crate::eval::config::{ActionPriorHeuristic, SearchConfig, SearchContext};
use crate::eval::evaluate::{full_tree_evaluation, value_to_probabilities, EvaluationError};
use crate::eval::normalize::{
    normalize_incomplete_information_state, NormalizedIncompleteInformation,
};
use crate::eval::puct::ActionProbabilities;

#[derive(Debug, Clone)]
struct TrainingExample {
    state: NormalizedIncompleteInformation,
    action_probabilities: ActionProbabilities,
    action_mask: ActionMask,
}

fn generate_training_example<H: ActionPriorHeuristic>(
    incomplete_information_state: game::IncompleteInformationGameState,
    heuristic: &mut H,
    temperature_schedule: f32,
) -> Result<(game::Move, TrainingExample), EvaluationError> {
    let search_config = SearchConfig::training(temperature_schedule);
    let mut search_context = SearchContext::new(heuristic, search_config);
    let (action_values, action_mask) =
        full_tree_evaluation(&incomplete_information_state, &mut search_context, 0)?;

    let action_probabilities = value_to_probabilities(
        &action_values,
        &action_mask,
        incomplete_information_state.current_player_number,
        temperature_schedule,
    );
    let selected_move = choose_best_action_from_probabilities(
        &action_probabilities,
        &action_mask,
        &incomplete_information_state.player_hand,
        &mut search_context.rng,
    )?;

    return Ok((
        selected_move,
        TrainingExample {
            state: normalize_incomplete_information_state(&incomplete_information_state),
            action_probabilities: action_probabilities,
            action_mask: action_mask,
        },
    ));
}

fn choose_best_action_from_probabilities(
    action_probabilities: &ActionProbabilities,
    action_mask: &ActionMask,
    hand: &game::PlayerHand,
    rng: &mut SmallRng,
) -> Result<game::Move, MoveIDError> {
    let mut action_ids = Vec::new();
    let mut weights = Vec::new();

    for action_id in MoveID::all() {
        if !action_mask[action_id] {
            continue;
        }

        let probability = action_probabilities[action_id];

        debug_assert!(
            probability.is_finite() && probability >= 0.0,
            "Action probability must be finite and nonnegative for valid actions"
        );

        if probability > 0.0 {
            action_ids.push(action_id);
            weights.push(probability);
        }
    }

    if action_ids.is_empty() {
        // Fallback to uniform sampling over valid actions.
        for action_id in MoveID::all() {
            if action_mask[action_id] {
                action_ids.push(action_id);
                weights.push(1.0);
            }
        }
    }

    let dist =
        WeightedAliasIndex::new(weights).expect("Failed to build action probability distribution");

    let sampled_index = dist.sample(rng);
    let action_id = action_ids[sampled_index];

    return action_id.to_move(hand);
}
