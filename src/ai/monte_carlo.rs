use std::fmt::Debug;

use crate::ai::game;

use rand::{rngs::ThreadRng, Rng};

pub trait Heuristic: std::fmt::Debug {
    fn estimate_state_value(
        &self,
        game_state: game::IncompleteInformationGameState,
        rng: &mut ThreadRng,
    ) -> f32;
}

pub struct BasicHeuristic {}

impl Debug for BasicHeuristic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BasicHeuristic")
    }
}

impl Heuristic for BasicHeuristic {
    fn estimate_state_value(
        &self,
        game_state: game::IncompleteInformationGameState,
        rng: &mut ThreadRng,
    ) -> f32 {
        let mut max_player_set_size = [0; crate::consts::MAX_CARD_ORDINALITY];
        let mut max_beating_set_size = [0; crate::consts::MAX_CARD_ORDINALITY];
        max_player_set_size[1] = game_state.player_hand[1] + game_state.player_hand[0];
        for i in 2..crate::consts::MAX_CARD_ORDINALITY {
            max_beating_set_size[i] = (game_state.opponent_cards[i - 1]
                + game_state.opponent_cards[0])
                .max(max_beating_set_size[i - 2]);
            max_player_set_size[i] = game_state.player_hand[i] + game_state.player_hand[0];
        }

        let will_win_current_trick = match game_state.trick.top_set {
            Some(top_set) => {
                top_set.player == game_state.perspective_player_number
                    && max_beating_set_size[top_set.rank] < top_set.number
            }
            None => false,
        };

        let mut num_unbeatable_playable_sets = 0;
        let mut num_unbeatable_unplayable_sets = 0;
        let mut num_beatable_playable_sets = 0;
        let mut num_beatable_unplayable_sets = 0;
        for i in 1..crate::consts::MAX_CARD_ORDINALITY {
            if game_state.player_hand[i] == 0 {
                continue;
            }
            let playable = will_win_current_trick
                || match game_state.trick.top_set {
                    Some(top_set) => {
                        top_set.rank < i
                            && (max_player_set_size[i] == top_set.number
                                || game_state.player_hand[i] == top_set.number)
                    }
                    None => false,
                };
            let unbeatable = max_beating_set_size[i] < max_player_set_size[i];
            if unbeatable {
                if playable {
                    num_unbeatable_playable_sets += 1;
                } else {
                    num_unbeatable_unplayable_sets += 1;
                }
            } else {
                if playable {
                    num_beatable_playable_sets += 1;
                } else {
                    num_beatable_unplayable_sets += 1;
                }
            }
        }

        // We want at least 1 unbeatable playable set
        // We can only have 1 beatable unplayable set
        // We would like to minimize the number of beatable playable sets
        // relative to the number of beatable playable sets our opponents
        // may have
        let mut min_opponent_hand_size = 500;
        for hand_size in &game_state.hand_sizes {
            if *hand_size == 0 {
                continue;
            }
            if *hand_size < min_opponent_hand_size {
                min_opponent_hand_size = *hand_size;
            }
        }
        let estimated_opponent_beatable_playable_sets =
            (((min_opponent_hand_size / 2) as f32) - 1.0).max(0.5);

        let trump_factor = 0.65
            * if num_unbeatable_playable_sets > 0 {
                0.85 + 0.15 * (num_unbeatable_playable_sets as f32 / 4.0)
            } else {
                0.1 - 0.1 * (num_unbeatable_unplayable_sets as f32 / 4.0)
            };
        let beatable_playable_factor = 0.2
            * (num_beatable_playable_sets as f32
                / estimated_opponent_beatable_playable_sets as f32);
        let beatable_factor = 0.5
            * if num_beatable_unplayable_sets > 1 {
                0.2
            } else if num_beatable_unplayable_sets == 0 {
                0.8
            } else {
                0.7
            };

        let min_hand_size = game_state.hand_sizes.iter().min().unwrap();
        let max_hand_size = game_state.hand_sizes.iter().max().unwrap();
        let hand_size_factor = 0.1
            * (game_state.hand_sizes[game_state.perspective_player_number] - min_hand_size) as f32
            / (max_hand_size - min_hand_size) as f32;

        trump_factor + beatable_factor - beatable_playable_factor
            + hand_size_factor
            + rng.gen_range(0.0..0.15)
    }
}

pub struct RandomHeuristic {}

impl Debug for RandomHeuristic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RandomHeuristic")
    }
}

impl Heuristic for RandomHeuristic {
    fn estimate_state_value(
        &self,
        _game_state: game::IncompleteInformationGameState,
        rng: &mut ThreadRng,
    ) -> f32 {
        rng.gen_range(0.0..1.0)
    }
}

// Takes the information we have currently about the game and uses a heuristic function to generate
// a best move. Searches using a markov tree to a given number of nodes before using the heuristic.
// Uses the markov_tree_state_value function to find the best move.
pub fn get_best_move(
    game_state: game::IncompleteInformationGameState,
    heuristic: &dyn Heuristic,
    num_rollouts: usize,
) -> game::Move {
    let available_moves =
        game::get_available_moves(game_state.player_hand, game_state.trick.top_set);
    let mut best_move = available_moves[0];
    let mut best_move_value = 0.0;

    for hypothetical_move in &available_moves {
        let mut hypothetical_game_state = game_state.clone();
        game::update_incomplete_information_game_state(
            &mut hypothetical_game_state,
            hypothetical_move,
        );
        let predicted_value = simple_markov_rollout(
            hypothetical_game_state,
            heuristic,
            num_rollouts / available_moves.len(),
        );
        if predicted_value > best_move_value {
            best_move = *hypothetical_move;
            best_move_value = predicted_value;
        }
    }

    best_move
}

// Also, the player who finishes a trick gets to start the next one.
// To do that, we need to keep track of which player's cards are currently in play.
pub fn simple_markov_rollout(
    game_state: game::IncompleteInformationGameState,
    heuristic: &dyn Heuristic,
    num_rollouts: usize,
) -> f64 {
    let perspective_player_number = game_state.perspective_player_number;
    if game_state.player_is_out[perspective_player_number] {
        return 1.0;
    }

    // Calculate the number of remaining players
    let mut num_players_before_rollout = 0;
    for hand_size in &game_state.hand_sizes {
        if *hand_size > 0 {
            num_players_before_rollout += 1;
        }
    }
    if num_players_before_rollout < 2 {
        return 0.0;
    }

    // Value accumulated from the rollouts
    let mut value = 0.0;

    // rng for random number generation
    let mut rng = rand::thread_rng();

    // Now we want to generate num_rollouts hypothetical scenarios, we will use these to play out the game
    // and update the value probabilities of the current game state for the current player.
    for _ in 0..num_rollouts {
        // Generate a random full information game state based on our current knowledge
        let mut hypothetical_game_state =
        game::generate_random_full_information_game_state_from_incomplete_information_game_state(
                &game_state,
            );

        // Now we want to play out the game until our player in question has no cards left
        // or until all other players are out
        while !hypothetical_game_state.player_is_out[perspective_player_number]
            || hypothetical_game_state
                .player_is_out
                .iter()
                .filter(|&&x| !x)
                .count()
                > 1
        {
            // Get the available moves for the current player
            let available_moves = game::get_available_moves(
                hypothetical_game_state.player_hands[hypothetical_game_state.current_player_number],
                hypothetical_game_state.trick.top_set,
            );

            // Use our heuristic to estimate the value of each of these moves
            let mut best_move = None;
            let mut best_move_value = -1000.0;

            for potential_move in &available_moves {
                let mut resulting_hypothetical_game_state = hypothetical_game_state.clone();
                game::update_full_information_game_state(
                    &mut resulting_hypothetical_game_state,
                    potential_move,
                );
                let rollout_player_perspective_game_state =
                    game::create_incomplete_information_game_state(
                        resulting_hypothetical_game_state,
                        perspective_player_number,
                    );

                let estimated_value =
                    heuristic.estimate_state_value(rollout_player_perspective_game_state, &mut rng);

                match best_move {
                    None => {
                        best_move = Some(potential_move);
                        best_move_value = estimated_value;
                    }
                    Some(_) => {
                        if estimated_value > best_move_value {
                            best_move = Some(potential_move);
                            best_move_value = estimated_value;
                        }
                    }
                }
            }

            // Update the game state
            game::update_full_information_game_state(
                &mut hypothetical_game_state,
                &best_move.unwrap(),
            );
        }

        // Now we want to update the value sum based on the final state of the game
        // Our player will receive a score between 0 and 1 based on their position in the rankings

        // Calculate the number of remaining players
        let mut num_players_after_rollout = 0;
        for player_hand in hypothetical_game_state.player_hands {
            let mut hand_size = 0;
            for card_count in player_hand {
                hand_size += card_count as u16;
            }
            if hand_size > 0 {
                num_players_after_rollout += 1;
            }
        }

        let rollout_value =
            num_players_after_rollout as f64 / ((num_players_before_rollout as f64) - 1.0);
        value += rollout_value / num_rollouts as f64;
    }

    value
}
