use thiserror::Error;

use crate::game::actions::{Move, Play, TopSet};
use crate::game::collections::{PlayerHand, PlayerIndexed, PlayerPlacements};
use crate::game::primitives::{CardCount, CardRank, PlayerID};
use crate::game::state::{FullInformationGameState, IncompleteInformationGameState};

#[derive(Error, Debug)]
pub enum GameLogicError {
    #[error("Tried to apply invalid move for given game state: {0}")]
    InvalidMove(String),
}

pub fn all_players_have_passed(
    has_passed: &PlayerIndexed<bool>,
    player_placements: &PlayerPlacements,
    number_of_players: usize,
    top_set_player: PlayerID,
) -> bool {
    // Check if all players have passed (except the top set player)
    for (player, has_passed) in has_passed.iter_active(number_of_players) {
        if player == top_set_player {
            continue;
        }

        if player_placements.is_out(player) {
            continue;
        }

        if !has_passed {
            return false;
        }
    }

    return true;
}

pub fn update_full_information_game_state(
    game_state: &mut FullInformationGameState,
    player_move: Move,
) -> Result<bool, GameLogicError> {
    match player_move {
        Move::Play(play) => {
            // Update the player's hand
            game_state.player_hands[game_state.current_player_number][CardRank::WILD] -=
                play.num_wilds;
            game_state.player_hands[game_state.current_player_number][play.rank] -=
                play.num_non_wilds;

            // Update the top set
            game_state.trick.top_set = Some(TopSet::new(
                game_state.current_player_number,
                play.rank,
                play.total_count(),
            ));

            // Check if the player is out
            if game_state.player_hands[game_state.current_player_number].is_empty() {
                game_state
                    .player_placements
                    .mark_out(game_state.current_player_number);
            }

            // Check if there is only one player left (game over)
            if game_state
                .player_placements
                .all_out_but_one(game_state.number_of_players)
            {
                // Update the player placement
                for player_id in PlayerID::all_player_ids(game_state.number_of_players) {
                    if !game_state.player_placements.is_out(player_id) {
                        game_state.player_placements.mark_out(player_id);
                        break;
                    }
                }

                return Ok(true);
            }

            // Reset the has_passed array
            game_state.trick.has_passed = PlayerIndexed::filled(false);

            // Update the player number
            match game_state.player_placements.get_next_active_player(
                game_state.current_player_number,
                game_state.number_of_players,
            ) {
                Some(player_number) => {
                    game_state.current_player_number = player_number;
                    return Ok(false);
                }
                None => return Ok(true),
            };
        }
        Move::Pass => {
            // Check if there is only one player left (game over)
            if game_state
                .player_placements
                .all_out_but_one(game_state.number_of_players)
            {
                // Update the player placement
                for player_id in PlayerID::all_player_ids(game_state.number_of_players) {
                    if !game_state.player_placements.is_out(player_id) {
                        game_state.player_placements.mark_out(player_id);
                        break;
                    }
                }

                return Ok(true);
            }

            // Update the has_passed array
            game_state.trick.has_passed[game_state.current_player_number] = true;

            let top_set_player = match game_state.trick.top_set {
                Some(top_set) => top_set.player,
                None => {
                    return Err(GameLogicError::InvalidMove(
                        "Tried to pass on empty trick!".to_string(),
                    ))
                }
            };

            if all_players_have_passed(
                &game_state.trick.has_passed,
                &game_state.player_placements,
                game_state.number_of_players,
                top_set_player,
            ) {
                // Start a new trick

                // Reset the has_passed array
                game_state.trick.has_passed = PlayerIndexed::filled(false);

                // Reset the top set
                let trick_winner = top_set_player;
                game_state.trick.top_set = None;

                // Update the player number
                if game_state.player_placements.is_out(trick_winner) {
                    // Player still in after trick winner starts the next trick
                    match game_state
                        .player_placements
                        .get_next_active_player(trick_winner, game_state.number_of_players)
                    {
                        Some(player_number) => {
                            game_state.current_player_number = player_number;
                            return Ok(false);
                        }
                        None => return Ok(true),
                    };
                } else {
                    // Trick winner starts the next trick
                    game_state.current_player_number = trick_winner;
                    return Ok(false);
                }
            } else {
                // Update the player number
                game_state.current_player_number = game_state
                    .player_placements
                    .get_next_active_player(
                        game_state.current_player_number,
                        game_state.number_of_players,
                    )
                    .unwrap();
                return Ok(false);
            }
        }
    }
}

pub fn update_incomplete_information_game_state(
    game_state: &mut IncompleteInformationGameState,
    player_move: Move,
) -> Result<(), GameLogicError> {
    match player_move {
        Move::Play(play) => {
            if game_state.current_player_number != game_state.perspective_player_number {
                // Update the opponent's hand
                game_state.opponent_cards[CardRank::WILD] -= play.num_wilds;
                game_state.opponent_cards[play.rank] -= play.num_non_wilds;
            } else {
                // Update the player's hand
                game_state.player_hand[CardRank::WILD] -= play.num_wilds;
                game_state.player_hand[play.rank] -= play.num_non_wilds;
            }

            // Update hand sizes
            game_state
                .hand_sizes
                .remove_cards(game_state.current_player_number, play.total_count());
            // Update the top set
            game_state.trick.top_set = Some(TopSet::new(
                game_state.current_player_number,
                play.rank,
                play.total_count(),
            ));

            // Check if the player is out
            if game_state
                .hand_sizes
                .is_empty(game_state.current_player_number)
            {
                game_state
                    .player_placements
                    .mark_out(game_state.current_player_number);
            }

            // Check if there is only one player left (game over)
            if game_state
                .player_placements
                .all_out_but_one(game_state.number_of_players)
            {
                // Update the player placement
                for player_id in PlayerID::all_player_ids(game_state.number_of_players) {
                    if !game_state.player_placements.is_out(player_id) {
                        game_state.player_placements.mark_out(player_id);
                        break;
                    }
                }

                return Ok(());
            }

            // Reset the has_passed array
            game_state.trick.has_passed = PlayerIndexed::filled(false);

            // Update the player number
            match game_state.player_placements.get_next_active_player(
                game_state.current_player_number,
                game_state.number_of_players,
            ) {
                Some(player_number) => {
                    game_state.current_player_number = player_number;
                }
                None => {}
            };
        }
        Move::Pass => {
            // Check if there is only one player left (game over)
            if game_state
                .player_placements
                .all_out_but_one(game_state.number_of_players)
            {
                // Update the player placement
                for player_id in PlayerID::all_player_ids(game_state.number_of_players) {
                    if !game_state.player_placements.is_out(player_id) {
                        game_state.player_placements.mark_out(player_id);
                        break;
                    }
                }

                return Ok(());
            }

            // Update the has_passed array
            game_state.trick.has_passed[game_state.current_player_number] = true;

            let top_set_player = match game_state.trick.top_set {
                Some(top_set) => top_set.player,
                None => {
                    return Err(GameLogicError::InvalidMove(
                        "Tried to pass on empty trick!".to_string(),
                    ))
                }
            };

            // Check if all players have passed (except the top set player)
            if all_players_have_passed(
                &game_state.trick.has_passed,
                &game_state.player_placements,
                game_state.number_of_players,
                top_set_player,
            ) {
                // Start a new trick

                // Reset the has_passed array
                game_state.trick.has_passed = PlayerIndexed::filled(false);

                // Reset the top set
                let trick_winner = top_set_player;
                game_state.trick.top_set = None;

                // Update the player number
                if game_state.player_placements.is_out(trick_winner) {
                    // Player still in after trick winner starts the next trick
                    match game_state
                        .player_placements
                        .get_next_active_player(trick_winner, game_state.number_of_players)
                    {
                        Some(player_number) => {
                            game_state.current_player_number = player_number;
                        }
                        None => {}
                    };
                } else {
                    // Trick winner starts the next trick
                    game_state.current_player_number = trick_winner;
                }
            } else {
                // Update the player number
                game_state.current_player_number = game_state
                    .player_placements
                    .get_next_active_player(
                        game_state.current_player_number,
                        game_state.number_of_players,
                    )
                    .unwrap();
            }
        }
    }

    return Ok(());
}

pub fn get_available_moves(hand: &PlayerHand, top_set: &Option<TopSet>) -> Vec<Move> {
    match top_set {
        Some(top_set) => {
            // We must play something from our hand with the same card number
            // and a lower card type
            let mut moves = Vec::new();
            let num_wilds = hand[CardRank::WILD];

            for rank in CardRank::non_wilds_below(top_set.rank) {
                let available_non_wilds = hand[rank];

                if available_non_wilds + num_wilds < top_set.number {
                    continue;
                }

                let max_non_wilds_playable = available_non_wilds.min(top_set.number);

                for num_non_wilds_played in CardCount::choices_largest_first(max_non_wilds_playable)
                {
                    let num_wilds_needed = top_set.number - num_non_wilds_played;
                    if num_wilds_needed > num_wilds {
                        break;
                    }

                    moves.push(Move::Play(Play::new(
                        rank,
                        num_non_wilds_played,
                        num_wilds_needed,
                    )));
                }
            }
            moves.push(Move::Pass);
            moves
        }
        None => {
            let mut moves = Vec::new();
            let num_wilds_available = hand[CardRank::WILD];

            for rank in CardRank::non_wilds() {
                let num_non_wilds_available = hand[rank];

                for num_non_wilds_played in CardCount::choices(num_non_wilds_available) {
                    for num_wilds_played in CardCount::choices(num_wilds_available) {
                        if num_non_wilds_played.is_zero() && num_wilds_played.is_zero() {
                            continue;
                        }

                        moves.push(Move::Play(Play::new(
                            rank,
                            num_non_wilds_played,
                            num_wilds_played,
                        )));
                    }
                }
            }
            moves
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::actions::Trick;
    use crate::game::collections::HandSizes;

    fn player(index: usize) -> PlayerID {
        PlayerID::new(index)
    }

    fn rank(index: usize) -> CardRank {
        CardRank::new(index)
    }

    fn count(value: usize) -> CardCount {
        CardCount::new(value)
    }

    fn play(rank_index: usize, non_wilds: usize, wilds: usize) -> Move {
        Move::Play(Play::new(rank(rank_index), count(non_wilds), count(wilds)))
    }

    fn top_set(player_index: usize, rank_index: usize, number: usize) -> TopSet {
        TopSet::new(player(player_index), rank(rank_index), count(number))
    }

    fn hand_from_pairs(pairs: &[(usize, usize)]) -> PlayerHand {
        let mut values = [CardCount::new(0); crate::consts::MAX_CARD_ORDINALITY];

        for &(rank_index, card_count) in pairs {
            values[rank_index] = CardCount::new(card_count);
        }

        PlayerHand::new(values)
    }

    fn player_hands_from_pairs(pairs: &[(usize, PlayerHand)]) -> PlayerIndexed<PlayerHand> {
        let mut hands = std::array::from_fn(|_| PlayerHand::empty());

        for &(player_index, ref hand) in pairs {
            hands[player_index] = hand.clone();
        }

        PlayerIndexed::new(hands)
    }

    fn hand_sizes_from_hands(
        player_hands: &PlayerIndexed<PlayerHand>,
        number_of_players: usize,
    ) -> HandSizes {
        let mut hand_sizes = HandSizes::empty();

        for player_id in PlayerID::all_player_ids(number_of_players) {
            for card_rank in CardRank::all() {
                hand_sizes.add_cards(player_id, player_hands[player_id][card_rank]);
            }
        }

        hand_sizes
    }

    fn full_state(
        current_player_number: usize,
        number_of_players: usize,
        player_hands: PlayerIndexed<PlayerHand>,
        player_placements: PlayerPlacements,
        trick: Trick,
    ) -> FullInformationGameState {
        FullInformationGameState {
            current_player_number: player(current_player_number),
            number_of_players,
            player_hands,
            player_placements,
            trick,
        }
    }

    fn incomplete_state(
        current_player_number: usize,
        perspective_player_number: usize,
        number_of_players: usize,
        player_hand: PlayerHand,
        opponent_cards: PlayerHand,
        player_placements: PlayerPlacements,
        hand_sizes: HandSizes,
        trick: Trick,
    ) -> IncompleteInformationGameState {
        IncompleteInformationGameState {
            current_player_number: player(current_player_number),
            perspective_player_number: player(perspective_player_number),
            number_of_players,
            player_hand,
            opponent_cards,
            player_placements,
            hand_sizes,
            trick,
        }
    }

    fn assert_has_move(moves: &[Move], expected: Move) {
        assert!(
            moves.contains(&expected),
            "expected move {expected:?} in {moves:?}"
        );
    }

    fn assert_does_not_have_move(moves: &[Move], unexpected: Move) {
        assert!(
            !moves.contains(&unexpected),
            "did not expect move {unexpected:?} in {moves:?}"
        );
    }

    fn assert_no_players_passed(trick: &Trick, number_of_players: usize) {
        for player_id in PlayerID::all_player_ids(number_of_players) {
            assert!(!trick.has_passed[player_id]);
        }
    }

    fn assert_enough_players(number_of_players: usize) {
        assert!(
            crate::consts::MAX_PLAYERS >= number_of_players,
            "test requires at least {number_of_players} players"
        );
    }

    fn assert_enough_ranks(number_of_ranks: usize) {
        assert!(
            crate::consts::MAX_CARD_ORDINALITY >= number_of_ranks,
            "test requires at least {number_of_ranks} ranks"
        );
    }

    #[test]
    fn game_logic_error_formats_invalid_move_message() {
        let error = GameLogicError::InvalidMove("bad move".to_string());

        assert_eq!(
            error.to_string(),
            "Tried to apply invalid move for given game state: bad move"
        );
    }

    #[test]
    fn all_players_have_passed_returns_true_when_all_other_active_players_passed() {
        assert_enough_players(4);

        let mut has_passed = PlayerIndexed::filled(false);
        has_passed[player(1)] = true;
        has_passed[player(2)] = true;
        has_passed[player(3)] = true;

        let placements = PlayerPlacements::new();

        assert!(all_players_have_passed(
            &has_passed,
            &placements,
            4,
            player(0),
        ));
    }

    #[test]
    fn all_players_have_passed_ignores_top_set_player() {
        assert_enough_players(3);

        let mut has_passed = PlayerIndexed::filled(false);
        has_passed[player(1)] = true;
        has_passed[player(2)] = true;

        let placements = PlayerPlacements::new();

        assert!(all_players_have_passed(
            &has_passed,
            &placements,
            3,
            player(0),
        ));
    }

    #[test]
    fn all_players_have_passed_returns_false_when_active_non_top_player_has_not_passed() {
        assert_enough_players(4);

        let mut has_passed = PlayerIndexed::filled(false);
        has_passed[player(1)] = true;
        has_passed[player(3)] = true;

        let placements = PlayerPlacements::new();

        assert!(!all_players_have_passed(
            &has_passed,
            &placements,
            4,
            player(0),
        ));
    }

    #[test]
    fn all_players_have_passed_ignores_out_players() {
        assert_enough_players(4);

        let mut has_passed = PlayerIndexed::filled(false);
        has_passed[player(1)] = true;
        has_passed[player(3)] = true;

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(2));

        assert!(all_players_have_passed(
            &has_passed,
            &placements,
            4,
            player(0),
        ));
    }

    #[test]
    fn all_players_have_passed_ignores_inactive_slots() {
        assert_enough_players(4);

        let mut has_passed = PlayerIndexed::filled(false);
        has_passed[player(1)] = true;
        has_passed[player(2)] = true;
        has_passed[player(3)] = false;

        let placements = PlayerPlacements::new();

        assert!(all_players_have_passed(
            &has_passed,
            &placements,
            3,
            player(0),
        ));
    }

    #[test]
    fn update_full_play_removes_cards_from_current_player_hand() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(0, 1), (1, 3)])),
            (1, hand_from_pairs(&[(2, 2)])),
            (2, hand_from_pairs(&[(1, 1)])),
        ]);

        let mut state = full_state(0, 3, player_hands, PlayerPlacements::new(), Trick::new());

        let finished = update_full_information_game_state(&mut state, play(1, 2, 1)).unwrap();

        assert!(!finished);
        assert_eq!(state.player_hands[player(0)][CardRank::WILD], count(0));
        assert_eq!(state.player_hands[player(0)][rank(1)], count(1));
    }

    #[test]
    fn update_full_play_sets_top_set_to_current_player_rank_and_total_count() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(0, 1), (1, 3)])),
            (1, hand_from_pairs(&[(2, 2)])),
            (2, hand_from_pairs(&[(1, 1)])),
        ]);

        let mut state = full_state(0, 3, player_hands, PlayerPlacements::new(), Trick::new());

        update_full_information_game_state(&mut state, play(1, 2, 1)).unwrap();

        assert_eq!(state.trick.top_set, Some(top_set(0, 1, 3)));
    }

    #[test]
    fn update_full_play_resets_has_passed() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.has_passed[player(1)] = true;
        trick.has_passed[player(2)] = true;

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 2)])),
            (1, hand_from_pairs(&[(2, 2)])),
            (2, hand_from_pairs(&[(1, 1)])),
        ]);

        let mut state = full_state(0, 3, player_hands, PlayerPlacements::new(), trick);

        update_full_information_game_state(&mut state, play(1, 1, 0)).unwrap();

        assert_no_players_passed(&state.trick, 3);
    }

    #[test]
    fn update_full_play_advances_to_next_active_player() {
        assert_enough_players(4);
        assert_enough_ranks(3);

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 2)])),
            (1, hand_from_pairs(&[(2, 2)])),
            (2, hand_from_pairs(&[(1, 1)])),
            (3, hand_from_pairs(&[(1, 1)])),
        ]);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));

        let mut state = full_state(0, 4, player_hands, placements, Trick::new());

        let finished = update_full_information_game_state(&mut state, play(1, 1, 0)).unwrap();

        assert!(!finished);
        assert_eq!(state.current_player_number, player(2));
    }

    #[test]
    fn update_full_play_marks_current_player_out_when_hand_becomes_empty() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, hand_from_pairs(&[(2, 2)])),
            (2, hand_from_pairs(&[(1, 1)])),
        ]);

        let mut state = full_state(0, 3, player_hands, PlayerPlacements::new(), Trick::new());

        let finished = update_full_information_game_state(&mut state, play(1, 1, 0)).unwrap();

        assert!(!finished);
        assert!(state.player_placements.is_out(player(0)));
        assert_eq!(state.player_placements[player(0)], 1);
        assert_eq!(state.current_player_number, player(1));
    }

    #[test]
    fn update_full_play_returns_finished_when_no_next_active_player_remains() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, PlayerHand::empty()),
            (2, PlayerHand::empty()),
        ]);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));
        placements.mark_out(player(2));

        let mut state = full_state(0, 3, player_hands, placements, Trick::new());

        let finished = update_full_information_game_state(&mut state, play(1, 1, 0)).unwrap();

        assert!(finished);
        assert!(state.player_placements.is_out(player(0)));
    }

    #[test]
    fn update_full_pass_on_empty_trick_returns_invalid_move_error() {
        assert_enough_players(3);

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, hand_from_pairs(&[(1, 1)])),
            (2, hand_from_pairs(&[(1, 1)])),
        ]);

        let mut state = full_state(0, 3, player_hands, PlayerPlacements::new(), Trick::new());

        let error = update_full_information_game_state(&mut state, Move::Pass).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Tried to apply invalid move for given game state: Tried to pass on empty trick!"
        );
    }

    #[test]
    fn update_full_pass_sets_current_player_has_passed_when_trick_continues() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, hand_from_pairs(&[(2, 1)])),
            (2, hand_from_pairs(&[(2, 1)])),
        ]);

        let mut state = full_state(1, 3, player_hands, PlayerPlacements::new(), trick);

        let finished = update_full_information_game_state(&mut state, Move::Pass).unwrap();

        assert!(!finished);
        assert!(state.trick.has_passed[player(1)]);
        assert_eq!(state.current_player_number, player(2));
        assert_eq!(state.trick.top_set, Some(top_set(0, 1, 1)));
    }

    #[test]
    fn update_full_pass_when_not_all_have_passed_advances_to_next_active_player() {
        assert_enough_players(4);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, hand_from_pairs(&[(2, 1)])),
            (2, hand_from_pairs(&[(2, 1)])),
            (3, hand_from_pairs(&[(2, 1)])),
        ]);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(2));

        let mut state = full_state(1, 4, player_hands, placements, trick);

        update_full_information_game_state(&mut state, Move::Pass).unwrap();

        assert_eq!(state.current_player_number, player(3));
    }

    #[test]
    fn update_full_pass_when_all_have_passed_clears_trick_and_winner_starts_if_still_active() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));
        trick.has_passed[player(1)] = true;

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, hand_from_pairs(&[(2, 1)])),
            (2, hand_from_pairs(&[(2, 1)])),
        ]);

        let mut state = full_state(2, 3, player_hands, PlayerPlacements::new(), trick);

        let finished = update_full_information_game_state(&mut state, Move::Pass).unwrap();

        assert!(!finished);
        assert_eq!(state.trick.top_set, None);
        assert_no_players_passed(&state.trick, 3);
        assert_eq!(state.current_player_number, player(0));
    }

    #[test]
    fn update_full_pass_when_winner_is_out_next_active_after_winner_starts() {
        assert_enough_players(4);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));
        trick.has_passed[player(1)] = true;
        trick.has_passed[player(2)] = true;

        let player_hands = player_hands_from_pairs(&[
            (0, PlayerHand::empty()),
            (1, hand_from_pairs(&[(2, 1)])),
            (2, hand_from_pairs(&[(2, 1)])),
            (3, hand_from_pairs(&[(2, 1)])),
        ]);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(0));

        let mut state = full_state(3, 4, player_hands, placements, trick);

        let finished = update_full_information_game_state(&mut state, Move::Pass).unwrap();

        assert!(!finished);
        assert_eq!(state.trick.top_set, None);
        assert_eq!(state.current_player_number, player(1));
    }

    #[test]
    fn update_full_pass_when_winner_is_out_and_no_active_player_remains_returns_finished() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));

        let player_hands = player_hands_from_pairs(&[
            (0, PlayerHand::empty()),
            (1, PlayerHand::empty()),
            (2, hand_from_pairs(&[(2, 1)])),
        ]);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(0));
        placements.mark_out(player(1));

        let mut state = full_state(2, 3, player_hands, placements, trick);

        let finished = update_full_information_game_state(&mut state, Move::Pass).unwrap();

        assert!(finished);
    }

    #[test]
    fn update_incomplete_play_by_perspective_player_removes_from_player_hand_not_opponents() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut hand_sizes = HandSizes::empty();
        hand_sizes.add_cards(player(0), count(3));
        hand_sizes.add_cards(player(1), count(2));
        hand_sizes.add_cards(player(2), count(2));

        let mut state = incomplete_state(
            0,
            0,
            3,
            hand_from_pairs(&[(0, 1), (1, 2)]),
            hand_from_pairs(&[(2, 4)]),
            PlayerPlacements::new(),
            hand_sizes,
            Trick::new(),
        );

        update_incomplete_information_game_state(&mut state, play(1, 1, 1)).unwrap();

        assert_eq!(state.player_hand[CardRank::WILD], count(0));
        assert_eq!(state.player_hand[rank(1)], count(1));
        assert_eq!(state.opponent_cards[rank(2)], count(4));
    }

    #[test]
    fn update_incomplete_play_by_opponent_removes_from_opponent_cards_not_player_hand() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut hand_sizes = HandSizes::empty();
        hand_sizes.add_cards(player(0), count(3));
        hand_sizes.add_cards(player(1), count(3));
        hand_sizes.add_cards(player(2), count(2));

        let mut state = incomplete_state(
            1,
            0,
            3,
            hand_from_pairs(&[(1, 3)]),
            hand_from_pairs(&[(0, 1), (2, 4)]),
            PlayerPlacements::new(),
            hand_sizes,
            Trick::new(),
        );

        update_incomplete_information_game_state(&mut state, play(2, 2, 1)).unwrap();

        assert_eq!(state.player_hand[rank(1)], count(3));
        assert_eq!(state.opponent_cards[CardRank::WILD], count(0));
        assert_eq!(state.opponent_cards[rank(2)], count(2));
    }

    #[test]
    fn update_incomplete_play_updates_hand_size_top_set_passes_and_current_player() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut hand_sizes = HandSizes::empty();
        hand_sizes.add_cards(player(0), count(3));
        hand_sizes.add_cards(player(1), count(3));
        hand_sizes.add_cards(player(2), count(2));

        let mut trick = Trick::new();
        trick.has_passed[player(2)] = true;

        let mut state = incomplete_state(
            1,
            0,
            3,
            hand_from_pairs(&[(1, 3)]),
            hand_from_pairs(&[(0, 1), (2, 4)]),
            PlayerPlacements::new(),
            hand_sizes,
            trick,
        );

        update_incomplete_information_game_state(&mut state, play(2, 2, 1)).unwrap();

        assert_eq!(state.hand_sizes[player(1)], 0);
        assert_eq!(state.trick.top_set, Some(top_set(1, 2, 3)));
        assert_no_players_passed(&state.trick, 3);
        assert_eq!(state.current_player_number, player(2));
        assert!(state.player_placements.is_out(player(1)));
    }

    #[test]
    fn update_incomplete_play_does_not_change_current_player_when_no_next_active_exists() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));
        placements.mark_out(player(2));

        let mut hand_sizes = HandSizes::empty();
        hand_sizes.add_cards(player(0), count(1));

        let mut state = incomplete_state(
            0,
            0,
            3,
            hand_from_pairs(&[(1, 1)]),
            PlayerHand::empty(),
            placements,
            hand_sizes,
            Trick::new(),
        );

        update_incomplete_information_game_state(&mut state, play(1, 1, 0)).unwrap();

        assert_eq!(state.current_player_number, player(0));
        assert!(state.player_placements.is_out(player(0)));
    }

    #[test]
    fn update_incomplete_pass_on_empty_trick_returns_invalid_move_error() {
        assert_enough_players(3);

        let mut state = incomplete_state(
            0,
            0,
            3,
            PlayerHand::empty(),
            PlayerHand::empty(),
            PlayerPlacements::new(),
            HandSizes::empty(),
            Trick::new(),
        );

        let error = update_incomplete_information_game_state(&mut state, Move::Pass).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Tried to apply invalid move for given game state: Tried to pass on empty trick!"
        );
    }

    #[test]
    fn update_incomplete_pass_when_trick_continues_sets_passed_and_advances() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));

        let mut state = incomplete_state(
            1,
            0,
            3,
            PlayerHand::empty(),
            PlayerHand::empty(),
            PlayerPlacements::new(),
            HandSizes::empty(),
            trick,
        );

        update_incomplete_information_game_state(&mut state, Move::Pass).unwrap();

        assert!(state.trick.has_passed[player(1)]);
        assert_eq!(state.current_player_number, player(2));
        assert_eq!(state.trick.top_set, Some(top_set(0, 1, 1)));
    }

    #[test]
    fn update_incomplete_pass_when_all_have_passed_clears_trick_and_winner_starts() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));
        trick.has_passed[player(1)] = true;

        let mut state = incomplete_state(
            2,
            0,
            3,
            PlayerHand::empty(),
            PlayerHand::empty(),
            PlayerPlacements::new(),
            HandSizes::empty(),
            trick,
        );

        update_incomplete_information_game_state(&mut state, Move::Pass).unwrap();

        assert_eq!(state.trick.top_set, None);
        assert_no_players_passed(&state.trick, 3);
        assert_eq!(state.current_player_number, player(0));
    }

    #[test]
    fn update_incomplete_pass_when_winner_is_out_next_active_after_winner_starts() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut trick = Trick::new();
        trick.top_set = Some(top_set(0, 1, 1));
        trick.has_passed[player(1)] = true;

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(0));

        let mut state = incomplete_state(
            2,
            0,
            3,
            PlayerHand::empty(),
            PlayerHand::empty(),
            placements,
            HandSizes::empty(),
            trick,
        );

        update_incomplete_information_game_state(&mut state, Move::Pass).unwrap();

        assert_eq!(state.trick.top_set, None);
        assert_eq!(state.current_player_number, player(1));
    }

    #[test]
    fn get_available_moves_on_empty_trick_returns_no_moves_for_empty_hand() {
        assert_enough_ranks(3);

        let moves = get_available_moves(&PlayerHand::empty(), &None);

        assert!(moves.is_empty());
    }

    #[test]
    fn get_available_moves_on_empty_trick_does_not_include_pass() {
        assert_enough_ranks(3);

        let hand = hand_from_pairs(&[(1, 1)]);

        let moves = get_available_moves(&hand, &None);

        assert!(!moves.contains(&Move::Pass));
    }

    #[test]
    fn get_available_moves_on_empty_trick_includes_non_wild_only_plays() {
        assert_enough_ranks(3);

        let hand = hand_from_pairs(&[(1, 2)]);

        let moves = get_available_moves(&hand, &None);

        assert_has_move(&moves, play(1, 1, 0));
        assert_has_move(&moves, play(1, 2, 0));
    }

    #[test]
    fn get_available_moves_on_empty_trick_includes_wild_assisted_and_all_wild_declared_rank_plays()
    {
        assert_enough_ranks(3);

        let hand = hand_from_pairs(&[(0, 2), (1, 1)]);

        let moves = get_available_moves(&hand, &None);

        assert_has_move(&moves, play(1, 1, 1));
        assert_has_move(&moves, play(1, 1, 2));
        assert_has_move(&moves, play(1, 0, 1));
        assert_has_move(&moves, play(1, 0, 2));
        assert_has_move(&moves, play(2, 0, 1));
        assert_has_move(&moves, play(2, 0, 2));
    }

    #[test]
    fn get_available_moves_on_empty_trick_excludes_empty_play() {
        assert_enough_ranks(3);

        let hand = hand_from_pairs(&[(0, 1), (1, 1)]);

        let moves = get_available_moves(&hand, &None);

        for game_move in moves {
            match game_move {
                Move::Play(play) => {
                    assert!(
                        !play.num_non_wilds.is_zero() || !play.num_wilds.is_zero(),
                        "get_available_moves returned an empty play: {play:?}"
                    );
                }
                Move::Pass => {
                    panic!("get_available_moves should not return Pass on an empty trick");
                }
            }
        }
    }

    #[test]
    fn get_available_moves_on_empty_trick_generates_expected_count_for_small_hand() {
        assert_enough_ranks(4);

        let hand = hand_from_pairs(&[(0, 1), (1, 2)]);

        let moves = get_available_moves(&hand, &None);

        // For rank 1: non_wild choices 0..=2, wild choices 0..=1, minus 0/0 => 5.
        // For every other non-wild rank: non_wild choices only 0, wild choices 0..=1,
        // minus 0/0 => 1 all-wild move per rank.
        let expected = 5 + (crate::consts::MAX_CARD_ORDINALITY - 2);

        assert_eq!(moves.len(), expected);
    }

    #[test]
    fn get_available_moves_on_top_set_always_includes_pass() {
        assert_enough_ranks(4);

        let hand = PlayerHand::empty();

        let moves = get_available_moves(&hand, &Some(top_set(0, 3, 1)));

        assert_eq!(moves, vec![Move::Pass]);
    }

    #[test]
    fn get_available_moves_on_top_set_only_uses_lower_non_wild_ranks() {
        assert_enough_ranks(5);

        let hand = hand_from_pairs(&[(1, 2), (2, 2), (3, 2), (4, 2)]);

        let moves = get_available_moves(&hand, &Some(top_set(0, 3, 1)));

        assert_has_move(&moves, play(1, 1, 0));
        assert_has_move(&moves, play(2, 1, 0));
        assert_does_not_have_move(&moves, play(3, 1, 0));
        assert_does_not_have_move(&moves, play(4, 1, 0));
        assert_has_move(&moves, Move::Pass);
    }

    #[test]
    fn get_available_moves_on_top_set_requires_same_total_count() {
        assert_enough_ranks(4);

        let hand = hand_from_pairs(&[(1, 3)]);

        let moves = get_available_moves(&hand, &Some(top_set(0, 3, 2)));

        assert_has_move(&moves, play(1, 2, 0));
        assert_does_not_have_move(&moves, play(1, 1, 0));
        assert_does_not_have_move(&moves, play(1, 3, 0));
    }

    #[test]
    fn get_available_moves_on_top_set_uses_wilds_to_complete_required_count() {
        assert_enough_ranks(4);

        let hand = hand_from_pairs(&[(0, 2), (1, 1)]);

        let moves = get_available_moves(&hand, &Some(top_set(0, 3, 2)));

        assert_has_move(&moves, play(1, 1, 1));
        assert_has_move(&moves, play(1, 0, 2));
        assert_has_move(&moves, Move::Pass);
    }

    #[test]
    fn get_available_moves_on_top_set_generates_largest_non_wild_usage_first() {
        assert_enough_ranks(4);

        let hand = hand_from_pairs(&[(0, 2), (1, 2)]);

        let moves = get_available_moves(&hand, &Some(top_set(0, 3, 2)));

        assert_eq!(moves[0], play(1, 2, 0));
        assert_eq!(moves[1], play(1, 1, 1));
        assert_eq!(moves[2], play(1, 0, 2));
    }

    #[test]
    fn get_available_moves_on_top_set_excludes_rank_when_cards_plus_wilds_are_insufficient() {
        assert_enough_ranks(4);

        let hand = hand_from_pairs(&[(0, 1), (1, 1)]);

        let moves = get_available_moves(&hand, &Some(top_set(0, 3, 3)));

        assert_eq!(moves, vec![Move::Pass]);
    }

    #[test]
    fn get_available_moves_on_top_set_with_rank_one_has_only_pass() {
        assert_enough_ranks(3);

        let hand = hand_from_pairs(&[(0, 5), (1, 5)]);

        let moves = get_available_moves(&hand, &Some(top_set(0, 1, 1)));

        assert_eq!(moves, vec![Move::Pass]);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn update_full_play_panics_when_non_wild_cards_are_not_available() {
        assert_enough_players(2);
        assert_enough_ranks(3);

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, hand_from_pairs(&[(2, 1)])),
        ]);

        let mut state = full_state(0, 2, player_hands, PlayerPlacements::new(), Trick::new());

        let _ = update_full_information_game_state(&mut state, play(1, 2, 0));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn update_incomplete_play_panics_when_hand_size_underflows() {
        assert_enough_players(2);
        assert_enough_ranks(3);

        let mut hand_sizes = HandSizes::empty();
        hand_sizes.add_cards(player(0), count(1));

        let mut state = incomplete_state(
            0,
            0,
            2,
            hand_from_pairs(&[(1, 1)]),
            PlayerHand::empty(),
            PlayerPlacements::new(),
            hand_sizes,
            Trick::new(),
        );

        let _ = update_incomplete_information_game_state(&mut state, play(1, 2, 0));
    }
}
