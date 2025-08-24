use kdam::{tqdm, BarExt};
use rand::Rng;

use crate::ai::monte_carlo::Heuristic;

// For now, only test them playing Dalmuti
pub fn run_ai_game(heuristics: &Vec<&dyn Heuristic>) -> Vec<f32> {
    let rng = &mut rand::thread_rng();

    // Choose between 3 and const::MAX_PLAYERS
    let min_num_players = 3.max(heuristics.len());
    let num_players = rng.gen_range(min_num_players..=crate::consts::MAX_PLAYERS);

    // Now, assign a heuristic to each player, ensuring that each heuristic is used at least once
    let mut num_with_heuristic = vec![1; heuristics.len()];
    let mut player_heuristics = vec![0; num_players];
    let mut player_numbers = (0..num_players).collect::<Vec<usize>>();
    for heuristic_num in 0..heuristics.len() {
        let random_player_index = rng.gen_range(0..player_numbers.len());
        let player_number = player_numbers.remove(random_player_index);
        player_heuristics[player_number] = heuristic_num;
    }

    // Fill in the rest of the players with random heuristics
    for player_number in player_numbers {
        let random_heuristic_index = rng.gen_range(0..heuristics.len());
        player_heuristics[player_number] = random_heuristic_index;
        num_with_heuristic[random_heuristic_index] += 1;
    }

    // Calculate the best and worst score each Heuristic could have
    let mut best_score = vec![0.0; heuristics.len()];
    let mut worst_score = vec![0.0; heuristics.len()];
    for (heuristic_idx, heuristic_num) in num_with_heuristic.iter().enumerate() {
        let mut total_best_score = 0.0;
        for top_placement in 0..*heuristic_num {
            total_best_score += (num_players as f32 - top_placement as f32) / num_players as f32;
        }
        best_score[heuristic_idx] = total_best_score / *heuristic_num as f32;

        let mut total_worst_score = 0.0;
        for bottom_placement in num_players - *heuristic_num..num_players {
            total_worst_score +=
                (num_players as f32 - bottom_placement as f32) / num_players as f32;
        }
        worst_score[heuristic_idx] = total_worst_score / *heuristic_num as f32;
    }

    // Now, we play the game
    let mut game_state = crate::ai::game::generate_random_initial_game_state(
        num_players,
        crate::consts::DEFAULT_DALMUTI_DECK,
    );
    let mut out_order = Vec::with_capacity(num_players);
    while game_state.player_is_out.iter().filter(|&&x| !x).count() > 1 {
        let player_in_question = game_state.current_player_number;
        let player_perspective_state = crate::ai::game::create_incomplete_information_game_state(
            game_state,
            player_in_question,
        );
        let chosen_move = crate::ai::monte_carlo::get_best_move(
            player_perspective_state,
            heuristics[player_heuristics[player_in_question]],
            1_000,
        );
        crate::ai::game::update_full_information_game_state(&mut game_state, &chosen_move);

        if game_state.player_is_out[player_in_question] {
            out_order.push(player_in_question);
        }
    }

    // Push the remaining player to the out_order
    for (player_number, &is_out) in game_state.player_is_out.iter().enumerate() {
        if !is_out {
            out_order.push(player_number);
            break;
        }
    }

    // Calculate the scores
    let mut scores = vec![0.0; heuristics.len()];
    for (placement, player_number) in out_order.iter().enumerate() {
        scores[player_heuristics[*player_number]] +=
            (num_players as f32 - placement as f32) / num_players as f32;
    }

    // Calculate the average score
    let mut average_scores = vec![0.0; heuristics.len()];
    for (heuristic_idx, heuristic_num) in num_with_heuristic.iter().enumerate() {
        average_scores[heuristic_idx] += scores[heuristic_idx] / *heuristic_num as f32;
    }

    // Now, normalize with the best and worst scores
    let mut normalized_scores = vec![0.0; heuristics.len()];
    for heuristic_idx in 0..heuristics.len() {
        normalized_scores[heuristic_idx] = (average_scores[heuristic_idx]
            - worst_score[heuristic_idx])
            / (best_score[heuristic_idx] - worst_score[heuristic_idx]);
    }

    normalized_scores
}

pub fn run_tourney(heuristics: &Vec<&dyn Heuristic>, num_games: usize, verbose: bool) -> Vec<f32> {
    let mut progress_bar = tqdm!(
        total = num_games,
        desc = "Playing games",
        disable = !verbose
    );
    progress_bar.refresh().unwrap();
    let mut total_scores = vec![0.0; heuristics.len()];
    for _ in 0..num_games {
        let scores = run_ai_game(heuristics);
        progress_bar.update(1).unwrap();
        for (heuristic_idx, score) in scores.iter().enumerate() {
            total_scores[heuristic_idx] += score;
        }
    }

    total_scores.iter().map(|&x| x / num_games as f32).collect()
}
