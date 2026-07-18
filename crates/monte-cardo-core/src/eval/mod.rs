mod actions;
mod config;
mod evaluate;
mod naive;
mod normalize;
mod puct;
mod zobrist;

pub use actions::{ActionMask, MoveID, MoveIDError};
pub use config::{ActionPriorHeuristic, SearchConfig, SearchContext};
pub use evaluate::{
    full_tree_evaluation, get_action_values, value_to_probabilities, EvaluationError,
};
pub use naive::{NaiveHeuristic, SimpleHeuristic};
pub use normalize::{
    normalize_incomplete_information_state, NormalizationError, NormalizedIncompleteInformation,
    RankCompressed, RankCompressible, RankCompressionMap,
};
pub use puct::ActionProbabilities;

mod tests {
    use crate::game;

    #[test]
    fn solver_does_not_return_compressed_move_as_uncompressed_after_rank_compression() {
        use crate::{consts, eval, game};

        let mut heuristic = eval::NaiveHeuristic::new();

        let config = eval::SearchConfig::inference();

        let mut search_context = eval::SearchContext::with_seed(&mut heuristic, config, 42);

        let mut game_state = game::generate_random_initial_game_state(
            5,
            &consts::DEFAULT_DALMUTI_DECK,
            &mut search_context.rng,
        );

        // P0 plays four 10s, no wilds.
        game::update_full_information_game_state(
            &mut game_state,
            game::Move::Play(game::Play::new(
                game::CardRank::new(10),
                game::CardCount::new(4),
                game::CardCount::new(0),
            )),
        )
        .unwrap();

        // P1-P4 pass.
        for _ in 0..4 {
            game::update_full_information_game_state(&mut game_state, game::Move::Pass).unwrap();
        }

        // P0 starts new trick and plays three 12s, no wilds.
        game::update_full_information_game_state(
            &mut game_state,
            game::Move::Play(game::Play::new(
                game::CardRank::new(12),
                game::CardCount::new(3),
                game::CardCount::new(0),
            )),
        )
        .unwrap();

        // Now P1 is to act.
        let perspective_player = game_state.current_player_number;

        let incomplete_state =
            game::create_incomplete_information_game_state(&game_state, perspective_player);

        let action_values = eval::get_action_values(&incomplete_state, &mut search_context);

        assert!(
            action_values.is_ok(),
            "get_action_values failed: {:?}",
            action_values.err(),
        );

        let action_values = action_values.unwrap();

        for (player_move, _value) in action_values {
            let legal_moves = game::get_available_moves(
                &incomplete_state.player_hand,
                &incomplete_state.trick.top_set,
            );

            assert!(
                legal_moves
                    .iter()
                    .any(|candidate| moves_are_equal(*candidate, player_move)),
                "Solver returned illegal move {:?} for hand {:?} and top set {:?}",
                player_move,
                incomplete_state.player_hand,
                incomplete_state.trick.top_set,
            );
        }
    }

    #[allow(unused)]
    fn moves_are_equal(a: game::Move, b: game::Move) -> bool {
        match (a, b) {
            (game::Move::Pass, game::Move::Pass) => true,
            (game::Move::Play(a), game::Move::Play(b)) => {
                a.rank == b.rank && a.num_non_wilds == b.num_non_wilds && a.num_wilds == b.num_wilds
            }
            _ => false,
        }
    }
}
