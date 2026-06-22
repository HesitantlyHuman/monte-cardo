use crate::consts;
use crate::game;

// TODO: Create feature branch for card rank compression. The card rank
// compression would improve the performance and reliability of PUCT, and reduce
// the learning load of the model by skipping eliminated ranks of cards. Then,
// the model will not have to learn that once all of the 1s are gone, 2s would
// function as the new 1s, and so on and so forth.

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

// pub fn normalize_hands(
//     player_hand: game::Hand,
//     opponent_cards: game::Hand,
// ) -> (game::Hand, game::Hand) {
//     // We need to compress all of the card ordinalities upwards by ignoring
//     // ordinalities with 0 count. We will ignore wilds because those have a
//     // special function, and so would not result in a comparable state.
//     let mut normalized_player_hand = [0; consts::MAX_CARD_ORDINALITY];
//     let mut normalized_opponent_cards = [0; consts::MAX_CARD_ORDINALITY];
//     normalized_player_hand[0] = player_hand[0];
//     normalized_opponent_cards[0] = opponent_cards[0];

//     let mut writing_index = 1;
//     for reading_index in 1..consts::MAX_CARD_ORDINALITY {
//         if player_hand[reading_index] + opponent_cards[reading_index] == 0 {
//             continue;
//         }

//         normalized_player_hand[writing_index] = player_hand[reading_index];
//         normalized_opponent_cards[writing_index] = opponent_cards[reading_index];

//         writing_index += 1;
//     }

//     (normalized_player_hand, normalized_opponent_cards)
// }

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

    // let (normalized_player_hand, normalized_opponent_cards) = normalize_hands(
    //     incomplete_information_state.player_hand,
    //     incomplete_information_state.opponent_cards,
    // );

    NormalizedIncompleteInformation {
        number_of_players: incomplete_information_state.number_of_players,
        player_hand: incomplete_information_state.player_hand,
        opponent_cards: incomplete_information_state.opponent_cards,
        hand_sizes: rotated_hand_sizes,
        trick: normalized_trick,
    }
}
