use crate::consts;
use crate::game;

// TODO: We can also perform an additional simplification in our normalization by eliminating ordinalities of cards that have already all been played.
// If all of the 1s have been played, for example, then 2s are the new ones, 3s are the new twos, and so on and so forth.
// Not only will this reduce the number of entries in our PUCT lookup, it will also simplify the learning for any network that we train.

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct NormalizedIncompleteInformation {
    pub number_of_players: usize,
    pub player_hand: game::Hand,
    pub opponent_cards: game::Hand,
    pub hand_sizes: [usize; consts::MAX_PLAYERS],
    pub trick: game::Trick,
}

fn normalize_player_index(
    absolute_player: usize,
    number_of_players: usize,
    perspective_player: usize,
) -> usize {
    debug_assert!(number_of_players > 0);
    debug_assert!(absolute_player < number_of_players);
    debug_assert!(perspective_player < number_of_players);

    (absolute_player + number_of_players - perspective_player) % number_of_players
}

fn left_rotate_index(index: usize, rotation_length: usize, zero: usize) -> usize {
    debug_assert!(rotation_length > 0);
    debug_assert!(index < rotation_length);
    debug_assert!(zero < rotation_length);
    return (index + zero) % rotation_length;
}

pub fn left_rotate_array<T: Copy>(
    array: &[T],
    target: &mut [T],
    rotation_length: usize,
    zero: usize,
) {
    debug_assert!(rotation_length <= array.len());
    debug_assert!(rotation_length <= target.len());
    debug_assert!(zero < rotation_length);
    for index in 0..rotation_length {
        let rotated_index = left_rotate_index(index, rotation_length, zero);
        target[index] = array[rotated_index];
    }
}

fn normalize_trick(
    trick: game::Trick,
    current_player_number: usize,
    number_of_players: usize,
) -> game::Trick {
    let top_set = match trick.top_set {
        Some(set) => {
            let normalized_player_number =
                normalize_player_index(set.player, number_of_players, current_player_number);
            Some(game::TopSet::new(
                normalized_player_number,
                set.rank,
                set.number,
            ))
        }
        None => None,
    };

    let mut rotated_has_passed = [false; consts::MAX_PLAYERS];
    left_rotate_array(
        &trick.has_passed,
        &mut rotated_has_passed,
        number_of_players,
        current_player_number,
    );

    game::Trick {
        top_set: top_set,
        has_passed: rotated_has_passed,
    }
}

pub fn normalize_incomplete_information_state(
    incomplete_information_state: game::IncompleteInformationGameState,
) -> NormalizedIncompleteInformation {
    let mut rotated_hand_sizes = [0; consts::MAX_PLAYERS];
    left_rotate_array(
        &incomplete_information_state.hand_sizes,
        &mut rotated_hand_sizes,
        incomplete_information_state.number_of_players,
        incomplete_information_state.current_player_number,
    );

    let normalized_trick = normalize_trick(
        incomplete_information_state.trick,
        incomplete_information_state.current_player_number,
        incomplete_information_state.number_of_players,
    );

    NormalizedIncompleteInformation {
        number_of_players: incomplete_information_state.number_of_players,
        player_hand: incomplete_information_state.player_hand,
        opponent_cards: incomplete_information_state.opponent_cards,
        hand_sizes: rotated_hand_sizes,
        trick: normalized_trick,
    }
}
