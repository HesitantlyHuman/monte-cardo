use rand::rngs::SmallRng;
use rand::SeedableRng;

mod ai;
mod consts;
mod ui;

pub fn stars_and_bars<const N: usize>(element_sum: usize) -> [usize; N] {
    [0; N]
}

// For now, we will leave this to be naive sampling. Just need to set it up to receive the rng.
pub fn sample_world(
    incomplete_information_game_state: &ai::game::IncompleteInformationGameState,
    rng: &mut SmallRng,
) -> ai::game::FullInformationGameState {
    // TODO: Remove this duplication (also found in generate_random_initial_game_state)
    let mut shuffled = Vec::new();
    for i in 0..consts::MAX_CARD_ORDINALITY {
        let num_cards = incomplete_information_game_state.opponent_cards[i];
        for _ in 0..num_cards {
            match shuffled.len() == 0 {
                false => {
                    let random_index = rng.gen_range(0..shuffled.len());
                    if random_index != shuffled.len() {
                        shuffled.push(shuffled[random_index]);
                        shuffled[random_index] = i;
                    } else {
                        shuffled.push(i);
                    }
                }
                true => {
                    shuffled.push(i);
                }
            };
        }
    }

    let mut player_hands = [([0; consts::MAX_CARD_ORDINALITY]); consts::MAX_PLAYERS];
    player_hands[incomplete_information_game_state.perspective_player_number] =
        incomplete_information_game_state.player_hand;
    for (hand_size_player_number, hand_size) in incomplete_information_game_state
        .hand_sizes
        .iter()
        .enumerate()
    {
        if *hand_size == 0 {
            continue;
        }
        if hand_size_player_number == incomplete_information_game_state.perspective_player_number {
            continue;
        }
        let mut random_hand = [0; consts::MAX_CARD_ORDINALITY];
        for _ in 0..*hand_size {
            let card = shuffled
                .pop()
                .expect("Hand sizes and opponent cards do not match, not enough cards to shuffle");
            random_hand[card] += 1;
        }
        player_hands[hand_size_player_number] = random_hand;
    }

    FullInformationGameState::new(
        incomplete_information_game_state.current_player_number,
        player_hands,
        incomplete_information_game_state.player_is_out,
        incomplete_information_game_state.trick,
    )
}

fn main() {}
