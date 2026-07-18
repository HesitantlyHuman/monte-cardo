use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

use crate::consts;
use crate::eval;
use crate::game::primitives::{CardCount, CardRank, PlayerID};

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerHand([CardCount; consts::MAX_CARD_ORDINALITY]);

impl PlayerHand {
    #[inline]
    pub fn new(values: [CardCount; consts::MAX_CARD_ORDINALITY]) -> Self {
        return Self(values);
    }

    #[inline]
    pub fn empty() -> Self {
        return Self([CardCount::new(0); consts::MAX_CARD_ORDINALITY]);
    }

    #[inline]
    pub fn to_usize_counts(&self) -> [usize; consts::MAX_CARD_ORDINALITY] {
        std::array::from_fn(|index| self.0[index].get())
    }

    pub fn iter(&self) -> impl Iterator<Item = &CardCount> {
        return self.0.iter();
    }

    pub fn total_cards(&self) -> usize {
        return self.0.iter().map(|count| count.get()).sum();
    }

    pub fn is_empty(&self) -> bool {
        return self.0.iter().all(|count| count.get() == 0);
    }
}

impl Index<CardRank> for PlayerHand {
    type Output = CardCount;

    #[inline]
    fn index(&self, rank: CardRank) -> &Self::Output {
        return &self.0[rank.get()];
    }
}

impl IndexMut<CardRank> for PlayerHand {
    #[inline]
    fn index_mut(&mut self, rank: CardRank) -> &mut Self::Output {
        return &mut self.0[rank.get()];
    }
}

impl eval::RankCompressible for PlayerHand {
    fn rank_compress(
        &self,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<eval::RankCompressed<Self>, eval::NormalizationError> {
        let mut compressed_hand = PlayerHand::empty();
        for card_rank in CardRank::all() {
            let card_count = self[card_rank];

            if card_count > CardCount::new(0) {
                let compressed_card_rank = match card_rank.rank_compress(&rank_compression_map) {
                    Ok(compressed_card_rank) => compressed_card_rank,
                    Err(_) => return Err(eval::NormalizationError::RankCompressionError(
                        format!("While compressing player hand: {:?}, encountered error compressing card rank: {:?}", self, card_rank)
                    ))
                };
                compressed_hand[*compressed_card_rank.inner()] = card_count;
            }
        }
        return Ok(eval::RankCompressed::new_unchecked(compressed_hand));
    }

    fn rank_decompress(
        compressed: &eval::RankCompressed<Self>,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<Self, eval::NormalizationError> {
        let mut uncompressed_hand = PlayerHand::empty();

        for card_rank in CardRank::all() {
            let card_count = compressed.inner()[card_rank];

            if card_count > CardCount::new(0) {
                let uncompressed_card_rank = CardRank::rank_decompress(
                    &eval::RankCompressed::new_unchecked(card_rank),
                    &rank_compression_map,
                )?;
                uncompressed_hand[uncompressed_card_rank] = card_count;
            }
        }
        return Ok(uncompressed_hand);
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerIndexed<T>([T; consts::MAX_PLAYERS]);

impl<T> PlayerIndexed<T> {
    #[inline]
    pub fn new(values: [T; consts::MAX_PLAYERS]) -> Self {
        return Self(values);
    }

    #[inline]
    pub fn get(&self) -> &[T; consts::MAX_PLAYERS] {
        return &self.0;
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        return self.0.iter();
    }

    pub fn iter_active(&self, number_of_players: usize) -> impl Iterator<Item = (PlayerID, &T)> {
        debug_assert!(number_of_players <= consts::MAX_PLAYERS);

        return PlayerID::all_player_ids(number_of_players)
            .map(move |player| (player, &self[player]));
    }
}

impl<T: Copy> PlayerIndexed<T> {
    #[inline]
    pub fn filled(value: T) -> Self {
        return Self([value; consts::MAX_PLAYERS]);
    }
}

impl<T> Index<PlayerID> for PlayerIndexed<T> {
    type Output = T;

    #[inline]
    fn index(&self, player: PlayerID) -> &Self::Output {
        return &self.0[player.get()];
    }
}

impl<T> IndexMut<PlayerID> for PlayerIndexed<T> {
    #[inline]
    fn index_mut(&mut self, player: PlayerID) -> &mut Self::Output {
        return &mut self.0[player.get()];
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerPlacements(PlayerIndexed<usize>);

impl PlayerPlacements {
    #[inline]
    pub fn new() -> Self {
        return Self(PlayerIndexed::filled(0));
    }

    #[inline]
    pub fn is_out(&self, player: PlayerID) -> bool {
        return self.0[player] != 0;
    }

    #[inline]
    pub fn all_out_but_one(&self, number_of_players: usize) -> bool {
        let mut num_not_out = 0;
        for player in PlayerID::all_player_ids(number_of_players) {
            if !self.is_out(player) {
                num_not_out += 1;
            }

            if num_not_out > 1 {
                return false;
            }
        }
        return true;
    }

    pub fn mark_out(&mut self, player: PlayerID) {
        debug_assert!(!self.is_out(player));

        let next_placement = self.0.iter().max().unwrap() + 1;

        self.0[player] = next_placement;
    }

    pub fn get_next_active_player(
        &self,
        current_player_number: PlayerID,
        number_of_players: usize,
    ) -> Option<PlayerID> {
        debug_assert!(number_of_players > 0);
        debug_assert!(number_of_players <= consts::MAX_PLAYERS);
        debug_assert!(current_player_number.get() < number_of_players);

        for i in 1..number_of_players {
            let next_player_number = (current_player_number.get() + i) % number_of_players;
            let next_player_number = PlayerID::new(next_player_number);
            if self.0[next_player_number] == 0 {
                return Some(next_player_number);
            }
        }
        return None;
    }
}

impl Index<PlayerID> for PlayerPlacements {
    type Output = usize;

    fn index(&self, index: PlayerID) -> &Self::Output {
        return &self.0[index];
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandSizes(PlayerIndexed<usize>);

impl HandSizes {
    #[inline]
    pub fn new(values: [usize; consts::MAX_PLAYERS]) -> Self {
        return Self(PlayerIndexed::new(values));
    }

    #[inline]
    pub fn empty() -> Self {
        return Self(PlayerIndexed::filled(0));
    }

    #[inline]
    pub fn is_empty(&self, player: PlayerID) -> bool {
        return self.0[player] == 0;
    }

    #[inline]
    pub fn get(&self) -> &[usize; consts::MAX_PLAYERS] {
        return self.0.get();
    }

    #[inline]
    pub fn add_cards(&mut self, player: PlayerID, count: CardCount) {
        self.0[player] += count.get();
    }

    #[inline]
    pub fn remove_cards(&mut self, player: PlayerID, count: CardCount) {
        debug_assert!(self.0[player] >= count.get());
        self.0[player] -= count.get();
    }
}

impl Index<PlayerID> for HandSizes {
    type Output = usize;

    fn index(&self, index: PlayerID) -> &Self::Output {
        return &self.0[index];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    fn player(index: usize) -> PlayerID {
        PlayerID::new(index)
    }

    fn rank(index: usize) -> CardRank {
        CardRank::new(index)
    }

    fn count(value: usize) -> CardCount {
        CardCount::new(value)
    }

    fn hand_from_pairs(pairs: &[(usize, usize)]) -> PlayerHand {
        let mut values = [CardCount::new(0); consts::MAX_CARD_ORDINALITY];

        for &(rank_index, card_count) in pairs {
            values[rank_index] = CardCount::new(card_count);
        }

        PlayerHand::new(values)
    }

    fn player_values() -> [usize; consts::MAX_PLAYERS] {
        std::array::from_fn(|index| index * 10 + 1)
    }

    #[test]
    fn player_hand_new_stores_values() {
        let values = std::array::from_fn(|index| CardCount::new(index % 3));
        let hand = PlayerHand::new(values);

        assert_eq!(hand.to_usize_counts(), values.map(CardCount::get));
    }

    #[test]
    fn player_hand_empty_has_all_zero_counts() {
        let hand = PlayerHand::empty();

        assert_eq!(hand.to_usize_counts(), [0; consts::MAX_CARD_ORDINALITY]);
    }

    #[test]
    fn player_hand_empty_is_empty() {
        let hand = PlayerHand::empty();

        assert!(hand.is_empty());
        assert_eq!(hand.total_cards(), 0);
    }

    #[test]
    fn player_hand_non_empty_is_not_empty() {
        let hand = hand_from_pairs(&[(1, 2)]);

        assert!(!hand.is_empty());
    }

    #[test]
    fn player_hand_total_cards_sums_all_ranks() {
        let hand = hand_from_pairs(&[(0, 1), (1, 2), (3, 4)]);

        assert_eq!(hand.total_cards(), 7);
    }

    #[test]
    fn player_hand_to_usize_counts_returns_all_counts_in_rank_order() {
        let hand = hand_from_pairs(&[(0, 1), (2, 3), (5, 2)]);

        let counts = hand.to_usize_counts();

        assert_eq!(counts[0], 1);
        assert_eq!(counts[1], 0);
        assert_eq!(counts[2], 3);
        assert_eq!(counts[5], 2);
    }

    #[test]
    fn player_hand_iter_returns_counts_in_rank_order() {
        let hand = hand_from_pairs(&[(0, 1), (2, 3), (5, 2)]);

        let counts: Vec<usize> = hand.iter().map(|count| count.get()).collect();

        assert_eq!(counts, hand.to_usize_counts().to_vec());
    }

    #[test]
    fn player_hand_index_reads_count_for_rank() {
        let hand = hand_from_pairs(&[(0, 1), (2, 3)]);

        assert_eq!(hand[rank(0)].get(), 1);
        assert_eq!(hand[rank(1)].get(), 0);
        assert_eq!(hand[rank(2)].get(), 3);
    }

    #[test]
    fn player_hand_index_mut_updates_count_for_rank() {
        let mut hand = PlayerHand::empty();

        hand[rank(2)] += count(3);

        assert_eq!(hand[rank(2)].get(), 3);
        assert_eq!(hand.total_cards(), 3);
    }

    #[test]
    fn player_hand_clone_preserves_counts() {
        let hand = hand_from_pairs(&[(1, 2), (4, 1)]);
        let cloned = hand.clone();

        assert_eq!(hand, cloned);
        assert_eq!(cloned.to_usize_counts(), hand.to_usize_counts());
    }

    #[test]
    fn player_hand_is_transparent_card_count_array_newtype() {
        assert_eq!(
            size_of::<PlayerHand>(),
            size_of::<[CardCount; consts::MAX_CARD_ORDINALITY]>()
        );
        assert_eq!(
            align_of::<PlayerHand>(),
            align_of::<[CardCount; consts::MAX_CARD_ORDINALITY]>()
        );
    }

    #[test]
    fn player_indexed_new_stores_values() {
        let values = player_values();
        let indexed = PlayerIndexed::new(values);

        assert_eq!(indexed.get(), &values);
    }

    #[test]
    fn player_indexed_filled_fills_all_slots() {
        let indexed = PlayerIndexed::filled(7usize);

        assert!(indexed.iter().all(|value| *value == 7));
    }

    #[test]
    fn player_indexed_get_returns_all_slots() {
        let values = player_values();
        let indexed = PlayerIndexed::new(values);

        assert_eq!(indexed.get().len(), consts::MAX_PLAYERS);
        assert_eq!(indexed.get(), &values);
    }

    #[test]
    fn player_indexed_iter_returns_all_slots_in_order() {
        let values = player_values();
        let indexed = PlayerIndexed::new(values);

        let iterated: Vec<usize> = indexed.iter().copied().collect();

        assert_eq!(iterated, values.to_vec());
    }

    #[test]
    fn player_indexed_iter_active_returns_only_active_players_in_order() {
        assert!(consts::MAX_PLAYERS >= 4);

        let values = player_values();
        let indexed = PlayerIndexed::new(values);

        let active: Vec<(usize, usize)> = indexed
            .iter_active(4)
            .map(|(player, value)| (player.get(), *value))
            .collect();

        assert_eq!(
            active,
            vec![
                (0, values[0]),
                (1, values[1]),
                (2, values[2]),
                (3, values[3]),
            ]
        );
    }

    #[test]
    fn player_indexed_iter_active_with_all_players_returns_all_slots() {
        let values = player_values();
        let indexed = PlayerIndexed::new(values);

        let active: Vec<(usize, usize)> = indexed
            .iter_active(consts::MAX_PLAYERS)
            .map(|(player, value)| (player.get(), *value))
            .collect();

        let expected: Vec<(usize, usize)> = values
            .iter()
            .enumerate()
            .map(|(player, value)| (player, *value))
            .collect();

        assert_eq!(active, expected);
    }

    #[test]
    fn player_indexed_index_reads_value_for_player() {
        let values = player_values();
        let indexed = PlayerIndexed::new(values);

        assert_eq!(indexed[player(0)], values[0]);
        assert_eq!(indexed[player(1)], values[1]);
    }

    #[test]
    fn player_indexed_index_mut_updates_value_for_player() {
        let mut indexed = PlayerIndexed::filled(0usize);

        indexed[player(2)] = 99;

        assert_eq!(indexed[player(2)], 99);
        assert_eq!(indexed[player(0)], 0);
    }

    #[test]
    fn player_indexed_clone_preserves_values() {
        let indexed = PlayerIndexed::new(player_values());
        let cloned = indexed.clone();

        assert_eq!(indexed, cloned);
    }

    #[test]
    fn player_indexed_is_transparent_array_newtype() {
        assert_eq!(
            size_of::<PlayerIndexed<usize>>(),
            size_of::<[usize; consts::MAX_PLAYERS]>()
        );
        assert_eq!(
            align_of::<PlayerIndexed<usize>>(),
            align_of::<[usize; consts::MAX_PLAYERS]>()
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_indexed_iter_active_rejects_too_many_players() {
        let indexed = PlayerIndexed::filled(0usize);

        let _ = indexed
            .iter_active(consts::MAX_PLAYERS + 1)
            .collect::<Vec<_>>();
    }

    #[test]
    fn player_placements_new_marks_all_players_in() {
        let placements = PlayerPlacements::new();

        for player_id in PlayerID::all_player_ids(consts::MAX_PLAYERS) {
            assert!(!placements.is_out(player_id));
            assert_eq!(placements[player_id], 0);
        }
    }

    #[test]
    fn player_placements_mark_out_marks_player_out() {
        let mut placements = PlayerPlacements::new();

        placements.mark_out(player(1));

        assert!(placements.is_out(player(1)));
        assert_eq!(placements[player(1)], 1);
        assert!(!placements.is_out(player(0)));
    }

    #[test]
    fn player_placements_mark_out_assigns_increasing_placements() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();

        placements.mark_out(player(2));
        placements.mark_out(player(0));
        placements.mark_out(player(3));

        assert_eq!(placements[player(2)], 1);
        assert_eq!(placements[player(0)], 2);
        assert_eq!(placements[player(3)], 3);
        assert_eq!(placements[player(1)], 0);
    }

    #[test]
    fn player_placements_is_out_returns_false_for_zero_placement() {
        let placements = PlayerPlacements::new();

        assert!(!placements.is_out(player(0)));
    }

    #[test]
    fn player_placements_is_out_returns_true_for_nonzero_placement() {
        let mut placements = PlayerPlacements::new();

        placements.mark_out(player(0));

        assert!(placements.is_out(player(0)));
    }

    #[test]
    fn player_placements_index_reads_player_placement() {
        let mut placements = PlayerPlacements::new();

        placements.mark_out(player(1));

        assert_eq!(placements[player(1)], 1);
        assert_eq!(placements[player(0)], 0);
    }

    #[test]
    fn player_placements_get_next_active_player_returns_next_player_when_active() {
        assert!(consts::MAX_PLAYERS >= 4);

        let placements = PlayerPlacements::new();

        assert_eq!(
            placements.get_next_active_player(player(0), 4),
            Some(player(1))
        );
    }

    #[test]
    fn player_placements_get_next_active_player_wraps_around() {
        assert!(consts::MAX_PLAYERS >= 4);

        let placements = PlayerPlacements::new();

        assert_eq!(
            placements.get_next_active_player(player(3), 4),
            Some(player(0))
        );
    }

    #[test]
    fn player_placements_get_next_active_player_skips_out_players() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));
        placements.mark_out(player(2));

        assert_eq!(
            placements.get_next_active_player(player(0), 4),
            Some(player(3))
        );
    }

    #[test]
    fn player_placements_get_next_active_player_skips_out_players_across_wraparound() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(0));
        placements.mark_out(player(1));

        assert_eq!(
            placements.get_next_active_player(player(3), 4),
            Some(player(2))
        );
    }

    #[test]
    fn player_placements_get_next_active_player_returns_none_when_no_other_active_players() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));
        placements.mark_out(player(2));
        placements.mark_out(player(3));

        assert_eq!(placements.get_next_active_player(player(0), 4), None);
    }

    #[test]
    fn player_placements_get_next_active_player_does_not_return_current_player() {
        let placements = PlayerPlacements::new();

        assert_eq!(placements.get_next_active_player(player(0), 1), None);
    }

    #[test]
    fn player_placements_clone_preserves_state() {
        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));

        let cloned = placements.clone();

        assert_eq!(placements, cloned);
        assert_eq!(cloned[player(1)], 1);
    }

    #[test]
    fn player_placements_is_transparent_player_indexed_usize_newtype() {
        assert_eq!(
            size_of::<PlayerPlacements>(),
            size_of::<PlayerIndexed<usize>>()
        );
        assert_eq!(
            align_of::<PlayerPlacements>(),
            align_of::<PlayerIndexed<usize>>()
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_placements_mark_out_rejects_player_who_is_already_out() {
        let mut placements = PlayerPlacements::new();

        placements.mark_out(player(0));
        placements.mark_out(player(0));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_placements_get_next_active_player_rejects_zero_players() {
        let placements = PlayerPlacements::new();

        let _ = placements.get_next_active_player(player(0), 0);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_placements_get_next_active_player_rejects_too_many_players() {
        let placements = PlayerPlacements::new();

        let _ = placements.get_next_active_player(player(0), consts::MAX_PLAYERS + 1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_placements_get_next_active_player_rejects_current_player_outside_active_range() {
        assert!(consts::MAX_PLAYERS >= 2);

        let placements = PlayerPlacements::new();

        let _ = placements.get_next_active_player(player(1), 1);
    }

    #[test]
    fn hand_sizes_new_stores_values() {
        let values = player_values();
        let hand_sizes = HandSizes::new(values);

        assert_eq!(hand_sizes.get(), &values);
    }

    #[test]
    fn hand_sizes_empty_sets_all_sizes_to_zero() {
        let hand_sizes = HandSizes::empty();

        assert_eq!(hand_sizes.get(), &[0; consts::MAX_PLAYERS]);
    }

    #[test]
    fn hand_sizes_empty_players_are_empty() {
        let hand_sizes = HandSizes::empty();

        for player_id in PlayerID::all_player_ids(consts::MAX_PLAYERS) {
            assert!(hand_sizes.is_empty(player_id));
            assert_eq!(hand_sizes[player_id], 0);
        }
    }

    #[test]
    fn hand_sizes_is_empty_returns_false_for_nonzero_size() {
        let mut hand_sizes = HandSizes::empty();

        hand_sizes.add_cards(player(0), count(3));

        assert!(!hand_sizes.is_empty(player(0)));
    }

    #[test]
    fn hand_sizes_get_returns_all_sizes() {
        let values = player_values();
        let hand_sizes = HandSizes::new(values);

        assert_eq!(hand_sizes.get(), &values);
    }

    #[test]
    fn hand_sizes_index_reads_size_for_player() {
        let values = player_values();
        let hand_sizes = HandSizes::new(values);

        assert_eq!(hand_sizes[player(0)], values[0]);
        assert_eq!(hand_sizes[player(1)], values[1]);
    }

    #[test]
    fn hand_sizes_add_cards_increases_size_for_player() {
        let mut hand_sizes = HandSizes::empty();

        hand_sizes.add_cards(player(2), count(4));

        assert_eq!(hand_sizes[player(2)], 4);
        assert!(!hand_sizes.is_empty(player(2)));
    }

    #[test]
    fn hand_sizes_add_cards_does_not_change_other_players() {
        let mut hand_sizes = HandSizes::empty();

        hand_sizes.add_cards(player(2), count(4));

        assert_eq!(hand_sizes[player(0)], 0);
        assert_eq!(hand_sizes[player(1)], 0);
        assert_eq!(hand_sizes[player(2)], 4);
    }

    #[test]
    fn hand_sizes_remove_cards_decreases_size_for_player() {
        let mut hand_sizes = HandSizes::empty();

        hand_sizes.add_cards(player(1), count(5));
        hand_sizes.remove_cards(player(1), count(2));

        assert_eq!(hand_sizes[player(1)], 3);
    }

    #[test]
    fn hand_sizes_remove_cards_can_make_player_empty() {
        let mut hand_sizes = HandSizes::empty();

        hand_sizes.add_cards(player(1), count(5));
        hand_sizes.remove_cards(player(1), count(5));

        assert_eq!(hand_sizes[player(1)], 0);
        assert!(hand_sizes.is_empty(player(1)));
    }

    #[test]
    fn hand_sizes_clone_preserves_sizes() {
        let mut hand_sizes = HandSizes::empty();
        hand_sizes.add_cards(player(1), count(5));

        let cloned = hand_sizes.clone();

        assert_eq!(hand_sizes, cloned);
        assert_eq!(cloned[player(1)], 5);
    }

    #[test]
    fn hand_sizes_is_transparent_player_indexed_usize_newtype() {
        assert_eq!(size_of::<HandSizes>(), size_of::<PlayerIndexed<usize>>());
        assert_eq!(align_of::<HandSizes>(), align_of::<PlayerIndexed<usize>>());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn hand_sizes_remove_cards_rejects_underflow() {
        let mut hand_sizes = HandSizes::empty();

        hand_sizes.remove_cards(player(0), count(1));
    }

    #[test]
    fn player_placements_all_out_but_one_returns_true_when_exactly_one_active_player_remains() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(0));
        placements.mark_out(player(1));
        placements.mark_out(player(3));

        assert!(placements.all_out_but_one(4));
    }

    #[test]
    fn player_placements_all_out_but_one_returns_false_when_no_players_are_out() {
        assert!(consts::MAX_PLAYERS >= 4);

        let placements = PlayerPlacements::new();

        assert!(!placements.all_out_but_one(4));
    }

    #[test]
    fn player_placements_all_out_but_one_returns_false_when_two_active_players_remain_at_front() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(2));
        placements.mark_out(player(3));

        assert!(!placements.all_out_but_one(4));
    }

    #[test]
    fn player_placements_all_out_but_one_returns_false_when_two_active_players_remain_at_end() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(0));
        placements.mark_out(player(1));

        assert!(!placements.all_out_but_one(4));
    }

    #[test]
    fn player_placements_all_out_but_one_returns_false_when_two_active_players_are_separated() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(1));
        placements.mark_out(player(3));

        assert!(!placements.all_out_but_one(4));
    }

    #[test]
    fn player_placements_all_out_but_one_ignores_inactive_slots() {
        assert!(consts::MAX_PLAYERS >= 4);

        let mut placements = PlayerPlacements::new();

        // Active players are 0, 1, and 2.
        // Player 3 is inactive for this call, so it should not matter that they are not out.
        placements.mark_out(player(0));
        placements.mark_out(player(1));

        assert!(placements.all_out_but_one(3));
    }

    #[test]
    fn player_placements_all_out_but_one_returns_true_for_one_player_game() {
        let placements = PlayerPlacements::new();

        assert!(placements.all_out_but_one(1));
    }

    #[test]
    fn player_placements_all_out_but_one_returns_true_when_all_active_players_are_out() {
        assert!(consts::MAX_PLAYERS >= 3);

        let mut placements = PlayerPlacements::new();
        placements.mark_out(player(0));
        placements.mark_out(player(1));
        placements.mark_out(player(2));

        assert!(placements.all_out_but_one(3));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_placements_all_out_but_one_rejects_zero_players() {
        let placements = PlayerPlacements::new();

        let _ = placements.all_out_but_one(0);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_placements_all_out_but_one_rejects_too_many_players() {
        let placements = PlayerPlacements::new();

        let _ = placements.all_out_but_one(consts::MAX_PLAYERS + 1);
    }
}
