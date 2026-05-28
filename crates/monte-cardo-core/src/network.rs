use crate::consts;
use crate::game::IncompleteInformationGameState;

pub const NETWORK_INPUTS_SIZE: usize = 2 * (consts::MAX_CARD_ORDINALITY * consts::MAX_CARD_NUMBER)
    + 1
    + consts::MAX_CARD_ORDINALITY
    + (consts::MAX_CARD_NUMBER - 1)
    + consts::MAX_PLAYERS * 3
    + 8;

pub type NetworkInputs = [u8; NETWORK_INPUTS_SIZE];

// Network inputs:
// 1. Card matrix of player hand
// 2. Card matrix of opponent hands
// 3. Whether there is a set in play
// 4. One-hot encoding of card rank of the set in play
// 5. One-hot encoding of number of cards in play
// 6. One-hot encoding of the current trick leader
// 7. Encoding of which players have passed on the current top set
// 8. The proportion of the deck each player has.
// All rotated such that the current player is 0 in the encodings.
pub fn prepare_network_inputs_from_incomplete_information_state(
    game_state: IncompleteInformationGameState,
    value: f64,
) -> NetworkInputs {
    // First, we need to create a mapping of the card ranks to eliminate cards which are no
    // longer in play. This does not apply to the wild cards, since they have a special function.
    let mut remaining_card_ranks = Vec::new();
    for i in 0..consts::MAX_CARD_ORDINALITY {
        if game_state.opponent_cards[i] > 0 || game_state.player_hand[i] > 0 {
            remaining_card_ranks.push(i);
        }
    }

    let mut player_card_matrix = vec![0; consts::MAX_CARD_ORDINALITY * consts::MAX_CARD_NUMBER];
    for (new_rank, old_rank) in remaining_card_ranks.iter().enumerate() {
        for j in 0..game_state.player_hand[*old_rank] {
            player_card_matrix[new_rank * consts::MAX_CARD_NUMBER + j as usize] = 1;
        }
    }

    let mut opponent_card_matrix = vec![0; consts::MAX_CARD_ORDINALITY * consts::MAX_CARD_NUMBER];
    for (new_rank, old_rank) in remaining_card_ranks.iter().enumerate() {
        for j in 0..game_state.opponent_cards[*old_rank] {
            opponent_card_matrix[new_rank * consts::MAX_CARD_NUMBER + j as usize] = 1;
        }
    }

    // Now, a mapping of remaining players
    let mut remaining_players = Vec::new();
    for i in 0..consts::MAX_PLAYERS {
        let player_number = (game_state.perspective_player_number + i) % consts::MAX_PLAYERS;
        if !game_state.player_is_out[player_number] {
            remaining_players.push(player_number);
        }
    }

    let is_top_set: u8;
    let mut top_set_rank_matrix = vec![0; consts::MAX_CARD_ORDINALITY];
    let mut top_set_number_matrix = vec![0; consts::MAX_CARD_NUMBER - 1];
    let mut top_set_player_matrix = vec![0; consts::MAX_PLAYERS];

    match game_state.trick.top_set {
        Some(top_set) => {
            is_top_set = 1;
            top_set_rank_matrix[top_set.rank] = 1;
            top_set_number_matrix[(top_set.number - 1) as usize] = 1;
            // If the top set player is still in the game, find their new index
            for (new_player, old_player) in remaining_players.iter().enumerate() {
                if *old_player == top_set.player {
                    top_set_player_matrix[new_player] = 1;
                    break;
                }
            }
        }
        None => {
            is_top_set = 0;
        }
    }

    let mut has_passed_matrix = vec![1; consts::MAX_PLAYERS];
    for (new_player, old_player) in remaining_players.iter().enumerate() {
        if !game_state.trick.has_passed[*old_player] {
            has_passed_matrix[new_player] = 0;
        }
    }

    let mut hand_sizes_matrix = vec![0; consts::MAX_PLAYERS];
    for (new_player, old_player) in remaining_players.iter().enumerate() {
        hand_sizes_matrix[new_player] = game_state.hand_sizes[*old_player] as u8;
    }

    let mut network_inputs = [0; NETWORK_INPUTS_SIZE];
    let mut write_position_start = 0;
    let mut write_position_end = consts::MAX_CARD_ORDINALITY * consts::MAX_CARD_NUMBER;
    // 1. Card matrix of player hand
    network_inputs[write_position_start..write_position_end].copy_from_slice(&player_card_matrix);
    write_position_start = write_position_end;
    write_position_end += consts::MAX_CARD_ORDINALITY * consts::MAX_CARD_NUMBER;
    // 2. Card matrix of opponent hands
    network_inputs[write_position_start..write_position_end].copy_from_slice(&opponent_card_matrix);
    write_position_start = write_position_end;
    write_position_end += 1;
    // 3. Whether there is a set in play
    network_inputs[write_position_start] = is_top_set;
    write_position_start = write_position_end;
    write_position_end += consts::MAX_CARD_ORDINALITY;
    // 4. One-hot encoding of the rank of the top set
    network_inputs[write_position_start..write_position_end].copy_from_slice(&top_set_rank_matrix);
    write_position_start = write_position_end;
    write_position_end += consts::MAX_CARD_NUMBER - 1;
    // 5. One-hot encoding of the number of the top set
    network_inputs[write_position_start..write_position_end]
        .copy_from_slice(&top_set_number_matrix);
    write_position_start = write_position_end;
    write_position_end += consts::MAX_PLAYERS;
    // 6. One-hot encoding of the current trick leader
    network_inputs[write_position_start..write_position_end]
        .copy_from_slice(&top_set_player_matrix);
    write_position_start = write_position_end;
    write_position_end += consts::MAX_PLAYERS;
    // 7. Encoding of the players who have passed
    network_inputs[write_position_start..write_position_end].copy_from_slice(&has_passed_matrix);
    write_position_start = write_position_end;
    write_position_end += consts::MAX_PLAYERS;
    // 8. Hand sizes of the remaining players (will be normalized in python later)
    network_inputs[write_position_start..write_position_end].copy_from_slice(&hand_sizes_matrix);

    // 9. Value of the state
    let value = value.to_le_bytes();
    network_inputs[write_position_end..write_position_end + 8].copy_from_slice(&value);

    network_inputs
}
