use core::panic;
use kdam::{tqdm, BarExt};
use rand::Rng;
use std::io::Write;

use crate::{
    ai::network::{self, NetworkInputs},
    consts,
};

struct BatchWriter {
    folder: std::path::PathBuf,
    file_chunk_size: usize,
    batch_number: usize,
    current_batch: Vec<network::NetworkInputs>,
}

impl BatchWriter {
    fn new(folder: std::path::PathBuf, file_chunk_size: usize) -> Self {
        // Create the folder if it doesn't exist
        if !folder.exists() {
            std::fs::create_dir_all(&folder).expect("Failed to create folder");
        }

        // Get the number of files in the folder
        let batch_number = std::fs::read_dir(&folder)
            .expect("Failed to read folder")
            .count();

        Self {
            folder,
            file_chunk_size,
            batch_number,
            current_batch: Vec::new(),
        }
    }

    fn write(&mut self, network_inputs: network::NetworkInputs) {
        self.current_batch.push(network_inputs);

        if self.current_batch.len() >= self.file_chunk_size {
            self.write_batch();
        }
    }

    fn write_batch(&mut self) {
        let file_path = self.folder.join(format!("batch_{}.bin", self.batch_number));
        let mut file = std::fs::File::create(file_path).expect("Failed to create file");
        for network_inputs in &self.current_batch {
            file.write_all(network_inputs)
                .expect("Failed to write to file");
        }

        self.current_batch.clear();
        self.batch_number += 1;
    }

    fn finish(&mut self) {
        if !self.current_batch.is_empty() {
            self.write_batch();
        }
    }
}

fn play_and_write_game(
    batch_writer: &mut BatchWriter,
    batch_writer_function: &mut dyn FnMut(&mut BatchWriter, NetworkInputs),
    deck: [u16; consts::MAX_CARD_ORDINALITY],
    num_players: usize,
) {
    let rng = &mut rand::thread_rng();
    let mut current_game_state =
        crate::ai::game::generate_random_initial_game_state(num_players, deck);

    // Iterate through the game until there is only one player left
    while current_game_state
        .player_is_out
        .iter()
        .filter(|&&x| !x)
        .count()
        > 1
    {
        // Figure out what move the current player should make
        let incomplete_information_game_state =
            crate::ai::game::create_incomplete_information_game_state(
                current_game_state,
                current_game_state.current_player_number,
            );
        let available_moves = crate::ai::game::get_available_moves(
            incomplete_information_game_state.player_hand,
            incomplete_information_game_state.trick.top_set,
        );
        let mut move_values = Vec::new();

        for hypothetical_move in &available_moves {
            let mut hypothetical_game_state = incomplete_information_game_state.clone();
            crate::ai::game::update_incomplete_information_game_state(
                &mut hypothetical_game_state,
                &hypothetical_move,
            );

            let predicted_value = crate::ai::monte_carlo::simple_markov_rollout(
                hypothetical_game_state,
                &crate::ai::monte_carlo::BasicHeuristic {},
                10_000,
            );
            if predicted_value.is_nan() {
                panic!("Predicted value is NaN");
            }
            if predicted_value.is_infinite() {
                panic!("Predicted value is infinite");
            }

            let network_inputs =
                crate::ai::network::prepare_network_inputs_from_incomplete_information_state(
                    hypothetical_game_state,
                    predicted_value,
                );
            batch_writer_function(batch_writer, network_inputs);

            move_values.push(predicted_value);
        }

        // Normalize the move values
        let move_values_sum = move_values.iter().sum::<f64>();
        for move_value in &mut move_values {
            *move_value /= move_values_sum;
        }

        // Randomly choose based on how good the moves are
        let random_value = rng.gen_range(0.0..1.0);
        let mut cumulative_value = 0.0;
        let mut selected_move = available_moves[0];
        for (i, &value) in move_values.iter().enumerate() {
            cumulative_value += value;
            if cumulative_value >= random_value {
                selected_move = available_moves[i];
                break;
            }
        }

        crate::ai::game::update_full_information_game_state(
            &mut current_game_state,
            &selected_move,
        );
    }
}

pub fn generate_data(
    folder: std::path::PathBuf,
    num_examples: usize,
    batch_size: usize,
    verbose: bool,
) {
    let rng = &mut rand::thread_rng();
    let mut batch_writer = BatchWriter::new(folder, batch_size);
    let mut progress_bar = tqdm!(
        total = num_examples,
        desc = "Generating data",
        animation = "fillup",
        disable = !verbose
    );
    let mut batch_writer_function =
        |batch_writer: &mut BatchWriter, network_inputs: NetworkInputs| {
            batch_writer.write(network_inputs);
            progress_bar.update(1).expect("Progress bar error");
        };
    let num_existing_batches = batch_writer.batch_number;

    // 10% chance of using Dalmuti deck
    // 10% chance of using Scum deck
    // 80% chance of using a random deck
    let num_batches = num_examples / batch_size + 1;
    let num_dalmuti_batches = num_batches / 10;
    let num_scum_batches = num_batches / 10;

    let mut num_dalmuti_games = 0;
    while batch_writer.batch_number < num_dalmuti_batches {
        let num_players = rng.gen_range(3..=consts::MAX_PLAYERS);
        play_and_write_game(
            &mut batch_writer,
            &mut batch_writer_function,
            consts::DEFAULT_DALMUTI_DECK,
            num_players,
        );
        num_dalmuti_games += 1;
    }
    let num_dalmuti_batches = batch_writer.batch_number;

    let mut num_scum_games = 0;
    while batch_writer.batch_number < num_dalmuti_batches + num_scum_batches {
        let num_players = rng.gen_range(3..=consts::MAX_PLAYERS);
        play_and_write_game(
            &mut batch_writer,
            &mut batch_writer_function,
            consts::DEFAULT_SCUM_DECK,
            num_players,
        );
        num_scum_games += 1;
    }

    let mut num_random_games = 0;
    while batch_writer.batch_number < num_batches {
        let mut deck = [0; consts::MAX_CARD_ORDINALITY];
        for i in 0..consts::MAX_CARD_ORDINALITY {
            deck[i] = rng.gen_range(0..=consts::MAX_CARD_NUMBER) as u16;
        }
        let num_players = rng.gen_range(3..=consts::MAX_PLAYERS);
        play_and_write_game(
            &mut batch_writer,
            &mut batch_writer_function,
            deck,
            num_players,
        );
        num_random_games += 1;
    }

    batch_writer.finish();
    if verbose {
        println!(
            "Finished generating data. Wrote {} batches. Played {} Dalmuti games, {} Scum games, and {} games with random decks.",
            batch_writer.batch_number - num_existing_batches,
            num_dalmuti_games,
            num_scum_games,
            num_random_games
        )
    }
}
