use crate::game;

use rand::Rng;

pub trait Heuristic {
    fn estimate_state_value(&self, game_state: game::IncompleteInformationGameState) -> f32;
}

pub struct BasicHeuristic {}

impl Heuristic for BasicHeuristic {
    fn estimate_state_value(&self, game_state: game::IncompleteInformationGameState) -> f32 {
        0.5
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
            let mut move_values = Vec::new();
            for _ in &available_moves {
                move_values.push(0.5);
            }

            // Normalize the move values
            let move_values_sum = move_values.iter().sum::<f32>();
            for move_value in &mut move_values {
                *move_value /= move_values_sum;
            }

            // Now use the heuristic values to randomly select a move
            let random_value = rng.gen_range(0.0..1.0);
            let mut cumulative_value = 0.0;
            let mut player_move = available_moves[0];
            for (i, move_value) in move_values.iter().enumerate() {
                cumulative_value += move_value;
                if random_value <= cumulative_value {
                    player_move = available_moves[i];
                    break;
                }
            }

            // Update the game state
            game::update_full_information_game_state(&mut hypothetical_game_state, &player_move);
        }

        // Now we want to update the value sum based on the final state of the game
        // Our player will receive a score between 0 and 1 based on their position in the rankings

        // Calculate the number of remaining players
        let mut num_players_after_rollout = 0;
        for hand_size in hypothetical_game_state.player_hands {
            if hand_size.iter().sum::<u16>() > 0 {
                num_players_after_rollout += 1;
            }
        }

        let rollout_value =
            num_players_after_rollout as f64 / ((num_players_before_rollout as f64) - 1.0);
        value += rollout_value / num_rollouts as f64;
    }

    value
}
