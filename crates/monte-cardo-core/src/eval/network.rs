use crate::consts;
use crate::eval::normalize::NormalizedIncompleteInformation;
use crate::game::{CardRank, PlayerID};

pub const MAX_TOTAL_PLAY: usize = consts::MAX_CARD_NUMBER * 2;

pub const CARD_MATRIX_SIZE: usize = consts::MAX_CARD_ORDINALITY * consts::MAX_CARD_NUMBER;

pub const NETWORK_INPUTS_SIZE: usize =
    // 1. Player hand card matrix
    CARD_MATRIX_SIZE
    // 2. Opponent card matrix
    + CARD_MATRIX_SIZE
    // 3. Player-count gates
    + consts::MAX_PLAYERS
    // 4. Whether there is a top set
    + 1
    // 5. One-hot rank of top set
    + consts::MAX_CARD_ORDINALITY
    // 6. One-hot number of cards in top set
    + MAX_TOTAL_PLAY
    // 7. One-hot trick leader
    + consts::MAX_PLAYERS
    // 8. Has-passed encoding
    + consts::MAX_PLAYERS
    // 9. Absolute normalized hand sizes
    + consts::MAX_PLAYERS
    // 10. Proportional normalized hand sizes
    + consts::MAX_PLAYERS;

pub type NetworkInputs = [f32; NETWORK_INPUTS_SIZE];

fn write_slice<const N: usize>(output: &mut [f32; N], cursor: &mut usize, values: &[f32]) {
    let end = *cursor + values.len();
    output[*cursor..end].copy_from_slice(values);
    *cursor = end;
}

fn binary_feature(value: bool) -> f32 {
    if value {
        1.0
    } else {
        -1.0
    }
}

fn make_negative_one_hot(length: usize, active_index: Option<usize>) -> Vec<f32> {
    let mut values = vec![-1.0; length];

    if let Some(index) = active_index {
        debug_assert!(index < length);
        values[index] = 1.0;
    }

    values
}

fn make_player_count_gates(number_of_players: usize) -> Vec<f32> {
    debug_assert!(number_of_players > 0);
    debug_assert!(number_of_players <= consts::MAX_PLAYERS);

    let mut values = vec![-1.0; consts::MAX_PLAYERS];

    for player in 0..number_of_players {
        values[player] = 1.0;
    }

    values
}

fn hand_to_card_matrix(hand: &crate::game::PlayerHand) -> Vec<f32> {
    let mut matrix = vec![-1.0; CARD_MATRIX_SIZE];

    for rank in CardRank::all() {
        debug_assert!(
            hand[rank].get() <= consts::MAX_CARD_NUMBER,
            "A single rank has more cards than MAX_CARD_NUMBER"
        );

        for card_index in 0..hand[rank].get() {
            matrix[rank.get() * consts::MAX_CARD_NUMBER + card_index] = 1.0;
        }
    }

    matrix
}

fn absolute_hand_size_features(
    hand_sizes: &[usize; consts::MAX_PLAYERS],
    number_of_players: usize,
) -> Vec<f32> {
    debug_assert!(number_of_players > 0);
    debug_assert!(number_of_players <= consts::MAX_PLAYERS);

    let mut features = vec![-1.0; consts::MAX_PLAYERS];

    // This is the maximum possible hand size under the current compact deck shape.
    // If you later have a tighter "max initial hand size" constant, that would
    // be an even better denominator.
    let max_possible_hand_size = CARD_MATRIX_SIZE as f32;

    for player in 0..number_of_players {
        let hand_size = hand_sizes[player];

        if hand_size == 0 {
            features[player] = -1.0;
            continue;
        }

        debug_assert!(
            hand_size <= CARD_MATRIX_SIZE,
            "Hand size exceeds maximum representable card count"
        );

        let normalized = 2.0 * (hand_size as f32 / max_possible_hand_size) - 1.0;
        features[player] = normalized.clamp(-1.0, 1.0);
    }

    features
}

fn proportional_hand_size_features(
    hand_sizes: &[usize; consts::MAX_PLAYERS],
    number_of_players: usize,
) -> Vec<f32> {
    debug_assert!(number_of_players > 0);
    debug_assert!(number_of_players <= consts::MAX_PLAYERS);

    let mut features = vec![-1.0; consts::MAX_PLAYERS];

    let total_remaining_cards: usize = hand_sizes[..number_of_players].iter().sum();

    if total_remaining_cards == 0 {
        return features;
    }

    for player in 0..number_of_players {
        let hand_size = hand_sizes[player];

        if hand_size == 0 {
            features[player] = -1.0;
            continue;
        }

        let proportion = hand_size as f32 / total_remaining_cards as f32;
        let normalized = 2.0 * proportion - 1.0;

        features[player] = normalized.clamp(-1.0, 1.0);
    }

    features
}

/// Network inputs:
///
/// 1. Card matrix of current player's hand.
/// 2. Card matrix of all opponent cards.
/// 3. Player-count gates, e.g. `[1, 1, 1, -1, -1, -1]`.
/// 4. Whether there is a top set.
/// 5. One-hot encoding of top-set rank.
/// 6. One-hot encoding of top-set card count.
/// 7. One-hot encoding of current trick leader.
/// 8. Encoding of which players have passed.
/// 9. Absolute normalized hand sizes.
/// 10. Proportional normalized hand sizes.
///
/// All fields are assumed to already be normalized so that player 0 is the
/// current/perspective player.
pub fn prepare_network_inputs_from_normalized_incomplete_information(
    state: NormalizedIncompleteInformation,
) -> NetworkInputs {
    debug_assert!(state.number_of_players > 0);
    debug_assert!(state.number_of_players <= consts::MAX_PLAYERS);

    let mut network_inputs = [0.0; NETWORK_INPUTS_SIZE];
    let mut cursor = 0;

    // 1. Card matrix of player hand.
    let player_card_matrix = hand_to_card_matrix(state.player_hand.inner());
    write_slice(&mut network_inputs, &mut cursor, &player_card_matrix);

    // 2. Card matrix of opponent cards.
    let opponent_card_matrix = hand_to_card_matrix(state.opponent_cards.inner());
    write_slice(&mut network_inputs, &mut cursor, &opponent_card_matrix);

    // 3. Player-count gates.
    let player_count_gates = make_player_count_gates(state.number_of_players);
    write_slice(&mut network_inputs, &mut cursor, &player_count_gates);

    // 4–7. Top-set features.
    match state.trick.inner().top_set {
        Some(top_set) => {
            debug_assert!(top_set.rank.get() < consts::MAX_CARD_ORDINALITY);
            debug_assert!(top_set.number.get() > 0);
            debug_assert!(top_set.number.get() <= MAX_TOTAL_PLAY);
            debug_assert!(top_set.player.get() < state.number_of_players);

            // 4. Whether there is a top set.
            write_slice(&mut network_inputs, &mut cursor, &[1.0]);

            // 5. Top-set rank.
            let top_set_rank =
                make_negative_one_hot(consts::MAX_CARD_ORDINALITY, Some(top_set.rank.get()));
            write_slice(&mut network_inputs, &mut cursor, &top_set_rank);

            // 6. Top-set number of cards.
            let top_set_number =
                make_negative_one_hot(MAX_TOTAL_PLAY, Some(top_set.number.get() - 1));
            write_slice(&mut network_inputs, &mut cursor, &top_set_number);

            // 7. Top-set player / trick leader.
            let top_set_player =
                make_negative_one_hot(consts::MAX_PLAYERS, Some(top_set.player.get()));
            write_slice(&mut network_inputs, &mut cursor, &top_set_player);
        }
        None => {
            // 4. Whether there is a top set.
            write_slice(&mut network_inputs, &mut cursor, &[-1.0]);

            // 5. No top-set rank.
            let top_set_rank = make_negative_one_hot(consts::MAX_CARD_ORDINALITY, None);
            write_slice(&mut network_inputs, &mut cursor, &top_set_rank);

            // 6. No top-set number.
            let top_set_number = make_negative_one_hot(MAX_TOTAL_PLAY, None);
            write_slice(&mut network_inputs, &mut cursor, &top_set_number);

            // 7. No top-set player.
            let top_set_player = make_negative_one_hot(consts::MAX_PLAYERS, None);
            write_slice(&mut network_inputs, &mut cursor, &top_set_player);
        }
    }

    // 8. Which players have passed.
    //
    //  1.0 means this player has passed.
    // -1.0 means this player has not passed or the player slot is unused.
    let mut has_passed_features = vec![-1.0; consts::MAX_PLAYERS];

    for player in PlayerID::all_player_ids(state.number_of_players) {
        has_passed_features[player.get()] = binary_feature(state.trick.inner().has_passed[player]);
    }

    write_slice(&mut network_inputs, &mut cursor, &has_passed_features);

    // 9. Absolute hand sizes, mapped to [-1, 1].
    //
    // -1.0 means the player is out or the player slot is unused.
    let absolute_hand_sizes =
        absolute_hand_size_features(&state.hand_sizes.get(), state.number_of_players);
    write_slice(&mut network_inputs, &mut cursor, &absolute_hand_sizes);

    // 10. Proportional hand sizes, mapped to [-1, 1].
    //
    // -1.0 means the player is out or the player slot is unused.
    let proportional_hand_sizes =
        proportional_hand_size_features(&state.hand_sizes.get(), state.number_of_players);
    write_slice(&mut network_inputs, &mut cursor, &proportional_hand_sizes);

    debug_assert_eq!(cursor, NETWORK_INPUTS_SIZE);

    network_inputs
}
