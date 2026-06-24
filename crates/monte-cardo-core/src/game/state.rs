use rand::{rngs::SmallRng, RngExt};

use crate::consts;
use crate::game::actions::Trick;
use crate::game::collections::{HandSizes, PlayerHand, PlayerIndexed, PlayerPlacements};
use crate::game::primitives::{CardCount, CardRank, PlayerID};

#[derive(Debug, Clone)]
pub struct IncompleteInformationGameState {
    pub current_player_number: PlayerID,
    pub perspective_player_number: PlayerID,
    pub number_of_players: usize,
    pub player_hand: PlayerHand,
    pub opponent_cards: PlayerHand,
    pub player_placements: PlayerPlacements,
    pub hand_sizes: HandSizes,
    pub trick: Trick,
}

impl IncompleteInformationGameState {
    fn new(
        current_player_number: PlayerID,
        perspective_player_number: PlayerID,
        number_of_players: usize,
        player_hand: PlayerHand,
        opponent_cards: PlayerHand,
        player_placements: PlayerPlacements,
        hand_sizes: HandSizes,
        trick: Trick,
    ) -> IncompleteInformationGameState {
        return IncompleteInformationGameState {
            current_player_number,
            perspective_player_number,
            number_of_players,
            player_hand,
            opponent_cards,
            player_placements,
            hand_sizes,
            trick,
        };
    }
}

#[derive(Debug, Clone)]
pub struct FullInformationGameState {
    pub current_player_number: PlayerID,
    pub number_of_players: usize,
    pub player_hands: PlayerIndexed<PlayerHand>,
    pub player_placements: PlayerPlacements,
    pub trick: Trick,
}

impl FullInformationGameState {
    fn new(
        current_player_number: PlayerID,
        number_of_players: usize,
        player_hands: PlayerIndexed<PlayerHand>,
        player_placements: PlayerPlacements,
        trick: Trick,
    ) -> FullInformationGameState {
        return FullInformationGameState {
            current_player_number,
            number_of_players,
            player_hands,
            player_placements,
            trick,
        };
    }
}

pub fn create_incomplete_information_game_state(
    full_information_game_state: &FullInformationGameState,
    perspective_player_number: PlayerID,
) -> IncompleteInformationGameState {
    let mut opponent_cards = PlayerHand::empty();
    let mut hand_sizes = HandSizes::empty();

    for player_id in PlayerID::all_player_ids(full_information_game_state.number_of_players) {
        let hand = &full_information_game_state.player_hands[player_id];
        for rank in CardRank::all() {
            hand_sizes.add_cards(player_id, hand[rank]);

            if player_id != perspective_player_number {
                opponent_cards[rank] += hand[rank]
            }
        }
    }

    return IncompleteInformationGameState::new(
        full_information_game_state.current_player_number,
        perspective_player_number,
        full_information_game_state.number_of_players,
        full_information_game_state.player_hands[perspective_player_number].clone(),
        opponent_cards,
        full_information_game_state.player_placements.clone(),
        hand_sizes,
        full_information_game_state.trick.clone(),
    );
}

pub fn generate_random_initial_game_state(
    num_players: usize,
    deck: &[usize; consts::MAX_CARD_ORDINALITY],
    rng: &mut SmallRng,
) -> FullInformationGameState {
    debug_assert!(num_players > 0);
    debug_assert!(num_players <= consts::MAX_PLAYERS);

    // Split the deck into equal parts for each player
    let mut shuffled = Vec::new();
    for i in 0..consts::MAX_CARD_ORDINALITY {
        let num_cards = deck[i];
        for _ in 0..num_cards {
            match shuffled.len() == 0 {
                false => {
                    let random_index = rng.random_range(0..=shuffled.len());
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

    let mut hands = Vec::new();
    for _ in 0..num_players {
        hands.push([0; consts::MAX_CARD_ORDINALITY]);
    }

    for i in 0..shuffled.len() {
        let player_number = i % num_players;
        let card = shuffled[i];
        hands[player_number][card] += 1;
    }

    let mut has_passed = [true; consts::MAX_PLAYERS];
    // Reset the has_passed array
    for (player_number, _) in hands.iter().enumerate() {
        has_passed[player_number] = false;
    }

    let mut trick = Trick::new();
    trick.has_passed = PlayerIndexed::new(has_passed);

    let mut array_hands = std::array::from_fn(|_| PlayerHand::empty());
    for (i, hand) in hands.iter().enumerate() {
        array_hands[i] = PlayerHand::new((*hand).map(CardCount::new));
    }

    return FullInformationGameState::new(
        PlayerID::new(0),
        num_players,
        PlayerIndexed::new(array_hands),
        PlayerPlacements::new(),
        trick,
    );
}
