use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::{consts, eval};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerID(usize);

impl PlayerID {
    #[inline]
    pub fn new(id: usize) -> Self {
        debug_assert!(id < consts::MAX_PLAYERS);
        return Self(id);
    }

    #[inline]
    pub fn get(self) -> usize {
        return self.0;
    }

    pub fn all_player_ids(number_of_players: usize) -> impl Iterator<Item = PlayerID> {
        debug_assert!(number_of_players <= consts::MAX_PLAYERS);
        debug_assert!(number_of_players > 0);
        return (0..number_of_players).map(Self::new);
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardRank(usize);

impl CardRank {
    pub const WILD: CardRank = CardRank(0);
    pub const LOWEST: CardRank = CardRank(consts::MAX_CARD_ORDINALITY);
    pub const HIGHEST: CardRank = CardRank(1);

    #[inline]
    pub fn new(rank: usize) -> Self {
        debug_assert!(rank < consts::MAX_CARD_ORDINALITY);
        return Self(rank);
    }

    #[inline]
    pub fn get(self) -> usize {
        return self.0;
    }

    pub fn all() -> impl Iterator<Item = CardRank> {
        return (0..consts::MAX_CARD_ORDINALITY).map(Self::new);
    }

    pub fn non_wilds() -> impl Iterator<Item = CardRank> {
        return (1..consts::MAX_CARD_ORDINALITY).map(Self::new);
    }

    pub fn non_wilds_below(rank: Self) -> impl Iterator<Item = CardRank> {
        return (1..rank.0).map(Self::new);
    }
}

impl eval::RankCompressible for CardRank {
    fn rank_compress(
        &self,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<eval::RankCompressed<Self>, eval::NormalizationError> {
        return rank_compression_map.compress_rank(*self);
    }

    fn rank_decompress(
        compressed: &eval::RankCompressed<Self>,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<Self, eval::NormalizationError> {
        return rank_compression_map.decompress_rank(&compressed);
    }
}

pub const MAX_TOTAL_PLAY: usize = consts::MAX_CARD_NUMBER * 2;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardCount(usize);

impl CardCount {
    #[inline]
    pub fn new(count: usize) -> Self {
        debug_assert!(count <= MAX_TOTAL_PLAY);
        return Self(count);
    }

    #[inline]
    pub fn get(self) -> usize {
        return self.0;
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        return self.0 == 0;
    }

    pub fn choices(max: CardCount) -> impl Iterator<Item = CardCount> {
        return (0..=max.get()).map(CardCount::new);
    }

    pub fn choices_largest_first(max: CardCount) -> impl Iterator<Item = CardCount> {
        return (0..=max.get()).rev().map(CardCount::new);
    }
}

impl Add for CardCount {
    type Output = CardCount;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        return CardCount::new(self.0 + rhs.0);
    }
}

impl AddAssign for CardCount {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for CardCount {
    type Output = CardCount;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        debug_assert!(rhs.0 <= self.0);
        return CardCount::new(self.0 - rhs.0);
    }
}

impl SubAssign for CardCount {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    fn player_ids_to_usize(ids: impl Iterator<Item = PlayerID>) -> Vec<usize> {
        ids.map(PlayerID::get).collect()
    }

    fn ranks_to_usize(ranks: impl Iterator<Item = CardRank>) -> Vec<usize> {
        ranks.map(CardRank::get).collect()
    }

    fn counts_to_usize(counts: impl Iterator<Item = CardCount>) -> Vec<usize> {
        counts.map(CardCount::get).collect()
    }

    #[test]
    fn player_id_new_stores_id() {
        let player = PlayerID::new(0);
        assert_eq!(player.get(), 0);

        let player = PlayerID::new(consts::MAX_PLAYERS - 1);
        assert_eq!(player.get(), consts::MAX_PLAYERS - 1);
    }

    #[test]
    fn player_id_is_copy_and_comparable() {
        let player = PlayerID::new(2);
        let copied = player;

        assert_eq!(player, copied);
        assert_eq!(player.get(), copied.get());
    }

    #[test]
    fn player_id_is_transparent_usize_newtype() {
        assert_eq!(size_of::<PlayerID>(), size_of::<usize>());
        assert_eq!(align_of::<PlayerID>(), align_of::<usize>());
    }

    #[test]
    fn all_player_ids_returns_zero_to_number_of_players_exclusive() {
        let players = player_ids_to_usize(PlayerID::all_player_ids(4));

        assert_eq!(players, vec![0, 1, 2, 3]);
    }

    #[test]
    fn all_player_ids_returns_all_max_players() {
        let players = player_ids_to_usize(PlayerID::all_player_ids(consts::MAX_PLAYERS));

        let expected: Vec<usize> = (0..consts::MAX_PLAYERS).collect();
        assert_eq!(players, expected);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn player_id_new_rejects_max_players() {
        let _ = PlayerID::new(consts::MAX_PLAYERS);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn all_player_ids_rejects_zero_players() {
        let _ = PlayerID::all_player_ids(0).collect::<Vec<_>>();
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn all_player_ids_rejects_too_many_players() {
        let _ = PlayerID::all_player_ids(consts::MAX_PLAYERS + 1).collect::<Vec<_>>();
    }

    #[test]
    fn card_rank_wild_is_zero() {
        assert_eq!(CardRank::WILD.get(), 0);
    }

    #[test]
    fn card_rank_new_stores_rank() {
        let rank = CardRank::new(0);
        assert_eq!(rank.get(), 0);

        let rank = CardRank::new(consts::MAX_CARD_ORDINALITY - 1);
        assert_eq!(rank.get(), consts::MAX_CARD_ORDINALITY - 1);
    }

    #[test]
    fn card_rank_is_copy_comparable_and_ordered() {
        let low = CardRank::new(1);
        let high = CardRank::new(2);
        let copied = low;

        assert_eq!(low, copied);
        assert!(low < high);
        assert!(high > low);
    }

    #[test]
    fn card_rank_is_transparent_usize_newtype() {
        assert_eq!(size_of::<CardRank>(), size_of::<usize>());
        assert_eq!(align_of::<CardRank>(), align_of::<usize>());
    }

    #[test]
    fn card_rank_all_returns_every_rank_in_order() {
        let ranks = ranks_to_usize(CardRank::all());

        let expected: Vec<usize> = (0..consts::MAX_CARD_ORDINALITY).collect();
        assert_eq!(ranks, expected);
    }

    #[test]
    fn card_rank_non_wilds_excludes_wild_and_returns_remaining_ranks_in_order() {
        let ranks = ranks_to_usize(CardRank::non_wilds());

        let expected: Vec<usize> = (1..consts::MAX_CARD_ORDINALITY).collect();
        assert_eq!(ranks, expected);
    }

    #[test]
    fn card_rank_non_wilds_below_wild_is_empty() {
        let ranks = ranks_to_usize(CardRank::non_wilds_below(CardRank::WILD));

        assert!(ranks.is_empty());
    }

    #[test]
    fn card_rank_non_wilds_below_first_non_wild_is_empty() {
        let ranks = ranks_to_usize(CardRank::non_wilds_below(CardRank::new(1)));

        assert!(ranks.is_empty());
    }

    #[test]
    fn card_rank_non_wilds_below_middle_rank_returns_lower_non_wilds() {
        let upper_rank = CardRank::new(5);
        let ranks = ranks_to_usize(CardRank::non_wilds_below(upper_rank));

        assert_eq!(ranks, vec![1, 2, 3, 4]);
    }

    #[test]
    fn card_rank_non_wilds_below_last_rank_returns_all_lower_non_wilds() {
        let upper_rank = CardRank::new(consts::MAX_CARD_ORDINALITY - 1);
        let ranks = ranks_to_usize(CardRank::non_wilds_below(upper_rank));

        let expected: Vec<usize> = (1..consts::MAX_CARD_ORDINALITY - 1).collect();
        assert_eq!(ranks, expected);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn card_rank_new_rejects_max_card_ordinality() {
        let _ = CardRank::new(consts::MAX_CARD_ORDINALITY);
    }

    #[test]
    fn max_total_play_matches_card_number_times_two() {
        assert_eq!(MAX_TOTAL_PLAY, consts::MAX_CARD_NUMBER * 2);
    }

    #[test]
    fn card_count_new_stores_count() {
        let count = CardCount::new(0);
        assert_eq!(count.get(), 0);

        let count = CardCount::new(MAX_TOTAL_PLAY);
        assert_eq!(count.get(), MAX_TOTAL_PLAY);
    }

    #[test]
    fn card_count_is_zero_identifies_zero_only() {
        assert!(CardCount::new(0).is_zero());
        assert!(!CardCount::new(1).is_zero());
    }

    #[test]
    fn card_count_is_copy_comparable_and_ordered() {
        let low = CardCount::new(1);
        let high = CardCount::new(2);
        let copied = low;

        assert_eq!(low, copied);
        assert!(low < high);
        assert!(high > low);
    }

    #[test]
    fn card_count_is_transparent_usize_newtype() {
        assert_eq!(size_of::<CardCount>(), size_of::<usize>());
        assert_eq!(align_of::<CardCount>(), align_of::<usize>());
    }

    #[test]
    fn card_count_choices_includes_zero_and_max_in_ascending_order() {
        let choices = counts_to_usize(CardCount::choices(CardCount::new(4)));

        assert_eq!(choices, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn card_count_choices_with_zero_max_returns_only_zero() {
        let choices = counts_to_usize(CardCount::choices(CardCount::new(0)));

        assert_eq!(choices, vec![0]);
    }

    #[test]
    fn card_count_choices_largest_first_includes_max_and_zero_in_descending_order() {
        let choices = counts_to_usize(CardCount::choices_largest_first(CardCount::new(4)));

        assert_eq!(choices, vec![4, 3, 2, 1, 0]);
    }

    #[test]
    fn card_count_choices_largest_first_with_zero_max_returns_only_zero() {
        let choices = counts_to_usize(CardCount::choices_largest_first(CardCount::new(0)));

        assert_eq!(choices, vec![0]);
    }

    #[test]
    fn card_count_add_returns_sum() {
        let left = CardCount::new(2);
        let right = CardCount::new(3);

        let result = left + right;

        assert_eq!(result.get(), 5);
    }

    #[test]
    fn card_count_add_assign_adds_into_left_hand_side() {
        let mut count = CardCount::new(2);

        count += CardCount::new(3);

        assert_eq!(count.get(), 5);
    }

    #[test]
    fn card_count_sub_returns_difference() {
        let left = CardCount::new(5);
        let right = CardCount::new(3);

        let result = left - right;

        assert_eq!(result.get(), 2);
    }

    #[test]
    fn card_count_sub_can_return_zero() {
        let left = CardCount::new(3);
        let right = CardCount::new(3);

        let result = left - right;

        assert_eq!(result.get(), 0);
        assert!(result.is_zero());
    }

    #[test]
    fn card_count_sub_assign_subtracts_from_left_hand_side() {
        let mut count = CardCount::new(5);

        count -= CardCount::new(3);

        assert_eq!(count.get(), 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn card_count_new_rejects_count_above_max_total_play() {
        let _ = CardCount::new(MAX_TOTAL_PLAY + 1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn card_count_add_rejects_result_above_max_total_play() {
        let _ = CardCount::new(MAX_TOTAL_PLAY) + CardCount::new(1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn card_count_add_assign_rejects_result_above_max_total_play() {
        let mut count = CardCount::new(MAX_TOTAL_PLAY);

        count += CardCount::new(1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn card_count_sub_rejects_underflow() {
        let _ = CardCount::new(0) - CardCount::new(1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn card_count_sub_assign_rejects_underflow() {
        let mut count = CardCount::new(0);

        count -= CardCount::new(1);
    }
}
