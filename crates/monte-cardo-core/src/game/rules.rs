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

pub fn get_available_moves(hand: &PlayerHand, top_set: Option<TopSet>) -> Vec<Move> {
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
