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
    debug_assert!(perspective_player_number.get() < full_information_game_state.number_of_players);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::actions::TopSet;
    use rand::SeedableRng;

    fn player(index: usize) -> PlayerID {
        PlayerID::new(index)
    }

    fn rank(index: usize) -> CardRank {
        CardRank::new(index)
    }

    fn count(value: usize) -> CardCount {
        CardCount::new(value)
    }

    fn assert_enough_players(number_of_players: usize) {
        assert!(
            consts::MAX_PLAYERS >= number_of_players,
            "test requires at least {number_of_players} max players"
        );
    }

    fn assert_enough_ranks(number_of_ranks: usize) {
        assert!(
            consts::MAX_CARD_ORDINALITY >= number_of_ranks,
            "test requires at least {number_of_ranks} card ranks"
        );
    }

    fn hand_from_pairs(pairs: &[(usize, usize)]) -> PlayerHand {
        let mut values = [CardCount::new(0); consts::MAX_CARD_ORDINALITY];

        for &(rank_index, card_count) in pairs {
            values[rank_index] = CardCount::new(card_count);
        }

        PlayerHand::new(values)
    }

    fn player_hands_from_pairs(pairs: &[(usize, PlayerHand)]) -> PlayerIndexed<PlayerHand> {
        let mut hands = std::array::from_fn(|_| PlayerHand::empty());

        for (player_index, hand) in pairs {
            hands[*player_index] = hand.clone();
        }

        PlayerIndexed::new(hands)
    }

    fn sample_deck() -> [usize; consts::MAX_CARD_ORDINALITY] {
        assert_enough_ranks(4);

        let mut deck = [0; consts::MAX_CARD_ORDINALITY];
        deck[0] = 2;
        deck[1] = 3;
        deck[2] = 4;
        deck[3] = 1;
        deck
    }

    fn rank_totals_from_game_state(
        game_state: &FullInformationGameState,
    ) -> [usize; consts::MAX_CARD_ORDINALITY] {
        let mut totals = [0; consts::MAX_CARD_ORDINALITY];

        for player_id in PlayerID::all_player_ids(game_state.number_of_players) {
            let hand = &game_state.player_hands[player_id];

            for card_rank in CardRank::all() {
                totals[card_rank.get()] += hand[card_rank].get();
            }
        }

        totals
    }

    fn active_hand_totals(game_state: &FullInformationGameState) -> Vec<usize> {
        PlayerID::all_player_ids(game_state.number_of_players)
            .map(|player_id| game_state.player_hands[player_id].total_cards())
            .collect()
    }

    #[test]
    fn incomplete_information_game_state_new_stores_all_fields() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let current_player_number = player(1);
        let perspective_player_number = player(2);
        let number_of_players = 3;
        let player_hand = hand_from_pairs(&[(1, 2), (2, 1)]);
        let opponent_cards = hand_from_pairs(&[(0, 1), (1, 3)]);
        let mut player_placements = PlayerPlacements::new();
        player_placements.mark_out(player(0));
        let mut hand_sizes = HandSizes::empty();
        hand_sizes.add_cards(player(0), count(0));
        hand_sizes.add_cards(player(1), count(4));
        hand_sizes.add_cards(player(2), count(3));

        let mut trick = Trick::new();
        let top_set = TopSet::new(player(1), rank(1), count(2));
        trick.top_set = Some(top_set);
        trick.has_passed[player(0)] = true;

        let state = IncompleteInformationGameState::new(
            current_player_number,
            perspective_player_number,
            number_of_players,
            player_hand.clone(),
            opponent_cards.clone(),
            player_placements.clone(),
            hand_sizes.clone(),
            trick.clone(),
        );

        assert_eq!(state.current_player_number, current_player_number);
        assert_eq!(state.perspective_player_number, perspective_player_number);
        assert_eq!(state.number_of_players, number_of_players);
        assert_eq!(state.player_hand, player_hand);
        assert_eq!(state.opponent_cards, opponent_cards);
        assert_eq!(state.player_placements, player_placements);
        assert_eq!(state.hand_sizes, hand_sizes);
        assert_eq!(state.trick, trick);
    }

    #[test]
    fn incomplete_information_game_state_clone_preserves_all_fields() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let state = IncompleteInformationGameState::new(
            player(1),
            player(2),
            3,
            hand_from_pairs(&[(1, 2)]),
            hand_from_pairs(&[(2, 3)]),
            PlayerPlacements::new(),
            HandSizes::new([0; consts::MAX_PLAYERS]),
            Trick::new(),
        );

        let cloned = state.clone();

        assert_eq!(cloned.current_player_number, state.current_player_number);
        assert_eq!(
            cloned.perspective_player_number,
            state.perspective_player_number
        );
        assert_eq!(cloned.number_of_players, state.number_of_players);
        assert_eq!(cloned.player_hand, state.player_hand);
        assert_eq!(cloned.opponent_cards, state.opponent_cards);
        assert_eq!(cloned.player_placements, state.player_placements);
        assert_eq!(cloned.hand_sizes, state.hand_sizes);
        assert_eq!(cloned.trick, state.trick);
    }

    #[test]
    fn full_information_game_state_new_stores_all_fields() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let current_player_number = player(1);
        let number_of_players = 3;

        let player_hands = player_hands_from_pairs(&[
            (0, hand_from_pairs(&[(1, 1)])),
            (1, hand_from_pairs(&[(2, 2)])),
            (2, hand_from_pairs(&[(0, 1), (1, 1)])),
        ]);

        let mut player_placements = PlayerPlacements::new();
        player_placements.mark_out(player(2));

        let mut trick = Trick::new();
        trick.top_set = Some(TopSet::new(player(1), rank(1), count(2)));
        trick.has_passed[player(0)] = true;

        let state = FullInformationGameState::new(
            current_player_number,
            number_of_players,
            player_hands.clone(),
            player_placements.clone(),
            trick.clone(),
        );

        assert_eq!(state.current_player_number, current_player_number);
        assert_eq!(state.number_of_players, number_of_players);
        assert_eq!(state.player_hands, player_hands);
        assert_eq!(state.player_placements, player_placements);
        assert_eq!(state.trick, trick);
    }

    #[test]
    fn full_information_game_state_clone_preserves_all_fields() {
        assert_enough_players(2);
        assert_enough_ranks(3);

        let state = FullInformationGameState::new(
            player(0),
            2,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 2)])),
                (1, hand_from_pairs(&[(2, 3)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let cloned = state.clone();

        assert_eq!(cloned.current_player_number, state.current_player_number);
        assert_eq!(cloned.number_of_players, state.number_of_players);
        assert_eq!(cloned.player_hands, state.player_hands);
        assert_eq!(cloned.player_placements, state.player_placements);
        assert_eq!(cloned.trick, state.trick);
    }

    #[test]
    fn create_incomplete_information_state_preserves_current_player_perspective_and_player_count() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let full_state = FullInformationGameState::new(
            player(1),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 1)])),
                (1, hand_from_pairs(&[(2, 2)])),
                (2, hand_from_pairs(&[(1, 3)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(2));

        assert_eq!(incomplete.current_player_number, player(1));
        assert_eq!(incomplete.perspective_player_number, player(2));
        assert_eq!(incomplete.number_of_players, 3);
    }

    #[test]
    fn create_incomplete_information_state_uses_perspective_players_hand() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let perspective_hand = hand_from_pairs(&[(1, 3), (2, 1)]);

        let full_state = FullInformationGameState::new(
            player(0),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 1)])),
                (1, hand_from_pairs(&[(2, 2)])),
                (2, perspective_hand.clone()),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(2));

        assert_eq!(incomplete.player_hand, perspective_hand);
    }

    #[test]
    fn create_incomplete_information_state_sums_all_non_perspective_hands_as_opponent_cards() {
        assert_enough_players(4);
        assert_enough_ranks(5);

        let full_state = FullInformationGameState::new(
            player(0),
            4,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(0, 1), (1, 2)])),
                (1, hand_from_pairs(&[(1, 1), (3, 2)])),
                (2, hand_from_pairs(&[(2, 4)])),
                (3, hand_from_pairs(&[(0, 1), (3, 1), (4, 2)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(2));
        let opponent_counts = incomplete.opponent_cards.to_usize_counts();

        assert_eq!(opponent_counts[0], 2);
        assert_eq!(opponent_counts[1], 3);
        assert_eq!(opponent_counts[2], 0);
        assert_eq!(opponent_counts[3], 3);
        assert_eq!(opponent_counts[4], 2);
    }

    #[test]
    fn create_incomplete_information_state_excludes_perspective_hand_from_opponent_cards() {
        assert_enough_players(3);
        assert_enough_ranks(4);

        let full_state = FullInformationGameState::new(
            player(0),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 1)])),
                (1, hand_from_pairs(&[(2, 5)])),
                (2, hand_from_pairs(&[(3, 2)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(1));
        let opponent_counts = incomplete.opponent_cards.to_usize_counts();

        assert_eq!(opponent_counts[1], 1);
        assert_eq!(opponent_counts[2], 0);
        assert_eq!(opponent_counts[3], 2);
    }

    #[test]
    fn create_incomplete_information_state_ignores_inactive_player_hands() {
        assert_enough_players(4);
        assert_enough_ranks(4);

        let full_state = FullInformationGameState::new(
            player(0),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 1)])),
                (1, hand_from_pairs(&[(2, 2)])),
                (2, hand_from_pairs(&[(3, 3)])),
                // This hand is outside number_of_players and should be ignored.
                (3, hand_from_pairs(&[(1, 9), (2, 9), (3, 9)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(0));
        let opponent_counts = incomplete.opponent_cards.to_usize_counts();

        assert_eq!(opponent_counts[1], 0);
        assert_eq!(opponent_counts[2], 2);
        assert_eq!(opponent_counts[3], 3);
    }

    #[test]
    fn create_incomplete_information_state_computes_hand_sizes_for_active_players() {
        assert_enough_players(4);
        assert_enough_ranks(4);

        let full_state = FullInformationGameState::new(
            player(0),
            4,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 1), (2, 1)])),
                (1, hand_from_pairs(&[(2, 3)])),
                (2, hand_from_pairs(&[(0, 1), (3, 2)])),
                (3, PlayerHand::empty()),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(1));

        assert_eq!(incomplete.hand_sizes[player(0)], 2);
        assert_eq!(incomplete.hand_sizes[player(1)], 3);
        assert_eq!(incomplete.hand_sizes[player(2)], 3);
        assert_eq!(incomplete.hand_sizes[player(3)], 0);
    }

    #[test]
    fn create_incomplete_information_state_leaves_inactive_hand_sizes_zero() {
        assert_enough_players(4);
        assert_enough_ranks(3);

        let full_state = FullInformationGameState::new(
            player(0),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 1)])),
                (1, hand_from_pairs(&[(2, 2)])),
                (2, hand_from_pairs(&[(1, 3)])),
                // Inactive slot with cards should not contribute to hand_sizes.
                (3, hand_from_pairs(&[(1, 5)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(0));

        assert_eq!(incomplete.hand_sizes[player(0)], 1);
        assert_eq!(incomplete.hand_sizes[player(1)], 2);
        assert_eq!(incomplete.hand_sizes[player(2)], 3);
        assert_eq!(incomplete.hand_sizes[player(3)], 0);
    }

    #[test]
    fn create_incomplete_information_state_clones_placements_and_trick() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));

        let mut trick = Trick::new();
        let top_set = TopSet::new(player(0), rank(1), count(2));
        trick.top_set = Some(top_set);
        trick.has_passed[player(2)] = true;

        let full_state = FullInformationGameState::new(
            player(0),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 2)])),
                (1, PlayerHand::empty()),
                (2, hand_from_pairs(&[(2, 1)])),
            ]),
            placements.clone(),
            trick.clone(),
        );

        let incomplete = create_incomplete_information_game_state(&full_state, player(0));

        assert_eq!(incomplete.player_placements, placements);
        assert_eq!(incomplete.trick, trick);
    }

    #[test]
    fn create_incomplete_information_state_does_not_mutate_full_state() {
        assert_enough_players(3);
        assert_enough_ranks(3);

        let full_state = FullInformationGameState::new(
            player(0),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 2)])),
                (1, hand_from_pairs(&[(2, 1)])),
                (2, hand_from_pairs(&[(1, 1), (2, 1)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let before_hands = full_state.player_hands.clone();
        let before_placements = full_state.player_placements.clone();
        let before_trick = full_state.trick.clone();

        let _ = create_incomplete_information_game_state(&full_state, player(1));

        assert_eq!(full_state.player_hands, before_hands);
        assert_eq!(full_state.player_placements, before_placements);
        assert_eq!(full_state.trick, before_trick);
    }

    #[test]
    fn generate_random_initial_game_state_sets_basic_state_fields() {
        assert_enough_players(3);

        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(1);

        let state = generate_random_initial_game_state(3, &deck, &mut rng);

        assert_eq!(state.current_player_number, player(0));
        assert_eq!(state.number_of_players, 3);
        assert_eq!(state.trick.top_set, None);
    }

    #[test]
    fn generate_random_initial_game_state_starts_with_no_player_out() {
        assert_enough_players(3);

        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(2);

        let state = generate_random_initial_game_state(3, &deck, &mut rng);

        for player_id in PlayerID::all_player_ids(3) {
            assert!(!state.player_placements.is_out(player_id));
            assert_eq!(state.player_placements[player_id], 0);
        }
    }

    #[test]
    fn generate_random_initial_game_state_marks_active_players_not_passed() {
        assert_enough_players(4);

        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(3);

        let state = generate_random_initial_game_state(4, &deck, &mut rng);

        for player_id in PlayerID::all_player_ids(4) {
            assert!(!state.trick.has_passed[player_id]);
        }
    }

    #[test]
    fn generate_random_initial_game_state_marks_inactive_player_slots_passed() {
        assert_enough_players(4);

        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(4);

        let state = generate_random_initial_game_state(3, &deck, &mut rng);

        assert!(!state.trick.has_passed[player(0)]);
        assert!(!state.trick.has_passed[player(1)]);
        assert!(!state.trick.has_passed[player(2)]);
        assert!(state.trick.has_passed[player(3)]);
    }

    #[test]
    fn generate_random_initial_game_state_preserves_total_cards_by_rank() {
        assert_enough_players(4);

        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(5);

        let state = generate_random_initial_game_state(4, &deck, &mut rng);
        let totals = rank_totals_from_game_state(&state);

        assert_eq!(totals, deck);
    }

    #[test]
    fn generate_random_initial_game_state_leaves_inactive_hands_empty() {
        assert_enough_players(4);

        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(6);

        let state = generate_random_initial_game_state(3, &deck, &mut rng);

        assert!(state.player_hands[player(3)].is_empty());
    }

    #[test]
    fn generate_random_initial_game_state_deals_cards_as_evenly_as_possible() {
        assert_enough_players(4);

        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(7);

        let state = generate_random_initial_game_state(4, &deck, &mut rng);
        let totals = active_hand_totals(&state);

        let min = totals.iter().min().copied().unwrap();
        let max = totals.iter().max().copied().unwrap();

        assert!(max - min <= 1);
        assert_eq!(totals.iter().sum::<usize>(), deck.iter().sum::<usize>());
    }

    #[test]
    fn generate_random_initial_game_state_handles_empty_deck() {
        assert_enough_players(3);

        let deck = [0; consts::MAX_CARD_ORDINALITY];
        let mut rng = SmallRng::seed_from_u64(8);

        let state = generate_random_initial_game_state(3, &deck, &mut rng);

        for player_id in PlayerID::all_player_ids(3) {
            assert!(state.player_hands[player_id].is_empty());
        }

        assert_eq!(rank_totals_from_game_state(&state), deck);
        assert_eq!(state.current_player_number, player(0));
        assert_eq!(state.trick.top_set, None);
    }

    #[test]
    fn generate_random_initial_game_state_is_deterministic_for_same_seed() {
        assert_enough_players(4);

        let deck = sample_deck();

        let mut first_rng = SmallRng::seed_from_u64(9);
        let mut second_rng = SmallRng::seed_from_u64(9);

        let first = generate_random_initial_game_state(4, &deck, &mut first_rng);
        let second = generate_random_initial_game_state(4, &deck, &mut second_rng);

        assert_eq!(first.current_player_number, second.current_player_number);
        assert_eq!(first.number_of_players, second.number_of_players);
        assert_eq!(first.player_hands, second.player_hands);
        assert_eq!(first.player_placements, second.player_placements);
        assert_eq!(first.trick, second.trick);
    }

    #[test]
    fn generate_random_initial_game_state_supports_one_player() {
        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(10);

        let state = generate_random_initial_game_state(1, &deck, &mut rng);

        assert_eq!(state.current_player_number, player(0));
        assert_eq!(state.number_of_players, 1);
        assert_eq!(rank_totals_from_game_state(&state), deck);
        assert!(!state.trick.has_passed[player(0)]);
    }

    #[test]
    fn generate_random_initial_game_state_supports_max_players() {
        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(11);

        let state = generate_random_initial_game_state(consts::MAX_PLAYERS, &deck, &mut rng);

        assert_eq!(state.current_player_number, player(0));
        assert_eq!(state.number_of_players, consts::MAX_PLAYERS);
        assert_eq!(rank_totals_from_game_state(&state), deck);

        for player_id in PlayerID::all_player_ids(consts::MAX_PLAYERS) {
            assert!(!state.trick.has_passed[player_id]);
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn generate_random_initial_game_state_rejects_zero_players() {
        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(12);

        let _ = generate_random_initial_game_state(0, &deck, &mut rng);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn generate_random_initial_game_state_rejects_too_many_players() {
        let deck = sample_deck();
        let mut rng = SmallRng::seed_from_u64(13);

        let _ = generate_random_initial_game_state(consts::MAX_PLAYERS + 1, &deck, &mut rng);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn create_incomplete_information_state_rejects_inactive_perspective_player() {
        assert_enough_players(4);
        assert_enough_ranks(3);

        let full_state = FullInformationGameState::new(
            player(0),
            3,
            player_hands_from_pairs(&[
                (0, hand_from_pairs(&[(1, 1)])),
                (1, hand_from_pairs(&[(2, 2)])),
                (2, hand_from_pairs(&[(1, 3)])),
            ]),
            PlayerPlacements::new(),
            Trick::new(),
        );

        let _ = create_incomplete_information_game_state(&full_state, player(3));
    }
}
