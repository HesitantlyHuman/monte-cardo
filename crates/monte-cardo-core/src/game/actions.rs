use crate::eval;
use crate::game::collections::PlayerIndexed;
use crate::game::primitives::{CardCount, CardRank, PlayerID, MAX_TOTAL_PLAY};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Play {
    pub rank: CardRank,
    pub num_non_wilds: CardCount,
    pub num_wilds: CardCount,
}

impl Play {
    #[inline]
    pub fn new(rank: CardRank, num_non_wilds: CardCount, num_wilds: CardCount) -> Play {
        debug_assert!(rank != CardRank::WILD);
        debug_assert!(!num_non_wilds.is_zero() || !num_wilds.is_zero());
        debug_assert!(num_non_wilds + num_wilds <= CardCount::new(MAX_TOTAL_PLAY));

        return Play {
            rank,
            num_non_wilds,
            num_wilds,
        };
    }

    #[inline]
    pub fn total_count(self) -> CardCount {
        return self.num_non_wilds + self.num_wilds;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Move {
    Play(Play),
    Pass,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct TopSet {
    pub player: PlayerID,
    pub rank: CardRank,
    pub number: CardCount,
}

impl TopSet {
    #[inline]
    pub fn new(player: PlayerID, rank: CardRank, number: CardCount) -> TopSet {
        debug_assert!(rank != CardRank::WILD);
        debug_assert!(!number.is_zero());

        return TopSet {
            player,
            rank,
            number,
        };
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Trick {
    pub top_set: Option<TopSet>,
    pub has_passed: PlayerIndexed<bool>,
}

impl Trick {
    pub fn new() -> Trick {
        return Trick {
            top_set: None,
            has_passed: PlayerIndexed::filled(false),
        };
    }
}

impl eval::RankCompressible for Trick {
    fn rank_compress(
        &self,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<eval::RankCompressed<Self>, eval::NormalizationError> {
        let mut rank_compressed_trick = self.clone();
        match rank_compressed_trick.top_set {
            Some(set) => {
                let compressed_set_rank = match set.rank.rank_compress(rank_compression_map) {
                    Ok(compressed_rank) => compressed_rank,
                    Err(err) => {
                        return Err(eval::NormalizationError::RankCompressionError(format!(
                            "Encountered error while rank compressing Trick: {}",
                            err
                        )))
                    }
                };
                rank_compressed_trick.top_set = Some(TopSet::new(
                    set.player,
                    *compressed_set_rank.inner(),
                    set.number,
                ));
            }
            None => {}
        }

        return Ok(eval::RankCompressed::new_unchecked(rank_compressed_trick));
    }

    fn rank_decompress(
        compressed: &eval::RankCompressed<Self>,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<Self, eval::NormalizationError> {
        let mut rank_decompressed_trick = compressed.inner().clone();
        match rank_decompressed_trick.top_set {
            Some(set) => {
                let decompressed_set_rank = CardRank::rank_decompress(
                    &set.rank.rank_compress(rank_compression_map)?,
                    rank_compression_map,
                )?;
                rank_decompressed_trick.top_set =
                    Some(TopSet::new(set.player, decompressed_set_rank, set.number));
            }
            None => {}
        }

        return Ok(rank_decompressed_trick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::mem::{align_of, size_of};

    fn player(index: usize) -> PlayerID {
        PlayerID::new(index)
    }

    fn non_wild_rank(index: usize) -> CardRank {
        debug_assert!(index > 0);
        CardRank::new(index)
    }

    fn count(value: usize) -> CardCount {
        CardCount::new(value)
    }

    fn hash_value<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    fn assert_has_at_least_one_non_wild_rank() {
        assert!(
            crate::consts::MAX_CARD_ORDINALITY > 1,
            "actions.rs tests require at least one non-wild rank"
        );
    }

    #[test]
    fn play_new_stores_rank_and_counts() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(2), count(3));

        assert_eq!(play.rank, non_wild_rank(1));
        assert_eq!(play.num_non_wilds, count(2));
        assert_eq!(play.num_wilds, count(3));
    }

    #[test]
    fn play_new_allows_non_wild_only_play() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(3), count(0));

        assert_eq!(play.rank, non_wild_rank(1));
        assert_eq!(play.num_non_wilds, count(3));
        assert_eq!(play.num_wilds, count(0));
        assert_eq!(play.total_count(), count(3));
    }

    #[test]
    fn play_new_allows_wild_assisted_play() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(2), count(1));

        assert_eq!(play.num_non_wilds, count(2));
        assert_eq!(play.num_wilds, count(1));
        assert_eq!(play.total_count(), count(3));
    }

    #[test]
    fn play_new_allows_all_wild_play_with_declared_non_wild_rank() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(0), count(2));

        assert_eq!(play.rank, non_wild_rank(1));
        assert_eq!(play.num_non_wilds, count(0));
        assert_eq!(play.num_wilds, count(2));
        assert_eq!(play.total_count(), count(2));
    }

    #[test]
    fn play_total_count_adds_non_wilds_and_wilds() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(4), count(2));

        assert_eq!(play.total_count(), count(6));
    }

    #[test]
    fn play_total_count_for_all_wild_play_is_number_of_wilds() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(0), count(3));

        assert_eq!(play.total_count(), count(3));
    }

    #[test]
    fn play_is_copy_clone_eq_and_hashable() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(2), count(1));
        let copied = play;
        let cloned = play.clone();

        assert_eq!(play, copied);
        assert_eq!(play, cloned);
        assert_eq!(hash_value(&play), hash_value(&copied));
    }

    #[test]
    fn play_different_counts_are_not_equal() {
        assert_has_at_least_one_non_wild_rank();

        let first = Play::new(non_wild_rank(1), count(2), count(1));
        let second = Play::new(non_wild_rank(1), count(1), count(2));

        assert_ne!(first, second);
    }

    #[test]
    fn play_different_ranks_are_not_equal() {
        assert!(
            crate::consts::MAX_CARD_ORDINALITY > 2,
            "test requires at least two non-wild ranks"
        );

        let first = Play::new(non_wild_rank(1), count(1), count(0));
        let second = Play::new(non_wild_rank(2), count(1), count(0));

        assert_ne!(first, second);
    }

    #[test]
    fn play_has_expected_size_and_alignment() {
        assert_eq!(
            size_of::<Play>(),
            size_of::<CardRank>() + size_of::<CardCount>() + size_of::<CardCount>()
        );

        assert_eq!(
            align_of::<Play>(),
            align_of::<CardRank>()
                .max(align_of::<CardCount>())
                .max(align_of::<CardCount>())
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn play_new_rejects_wild_rank() {
        let _ = Play::new(CardRank::WILD, count(1), count(0));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn play_new_rejects_empty_play() {
        assert_has_at_least_one_non_wild_rank();

        let _ = Play::new(non_wild_rank(1), count(0), count(0));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn play_new_rejects_total_count_above_max_total_play() {
        assert_has_at_least_one_non_wild_rank();

        let _ = Play::new(non_wild_rank(1), count(MAX_TOTAL_PLAY), count(1));
    }

    #[test]
    fn move_play_variant_stores_play() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(2), count(1));
        let game_move = Move::Play(play);

        match game_move {
            Move::Play(stored_play) => {
                assert_eq!(stored_play, play);
            }
            Move::Pass => panic!("expected Move::Play"),
        }
    }

    #[test]
    fn move_pass_variant_matches_pass() {
        let game_move = Move::Pass;

        match game_move {
            Move::Pass => {}
            Move::Play(_) => panic!("expected Move::Pass"),
        }
    }

    #[test]
    fn move_is_copy_clone_eq_and_hashable() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(2), count(1));
        let first = Move::Play(play);
        let copied = first;
        let cloned = first.clone();

        assert_eq!(first, copied);
        assert_eq!(first, cloned);
        assert_eq!(hash_value(&first), hash_value(&copied));

        let pass = Move::Pass;
        let copied_pass = pass;

        assert_eq!(pass, copied_pass);
        assert_eq!(hash_value(&pass), hash_value(&copied_pass));
    }

    #[test]
    fn move_play_and_pass_are_not_equal() {
        assert_has_at_least_one_non_wild_rank();

        let play = Play::new(non_wild_rank(1), count(1), count(0));

        assert_ne!(Move::Play(play), Move::Pass);
    }

    #[test]
    fn different_move_plays_are_not_equal() {
        assert_has_at_least_one_non_wild_rank();

        let first = Move::Play(Play::new(non_wild_rank(1), count(1), count(0)));
        let second = Move::Play(Play::new(non_wild_rank(1), count(2), count(0)));

        assert_ne!(first, second);
    }

    #[test]
    fn top_set_new_stores_player_rank_and_number() {
        assert_has_at_least_one_non_wild_rank();

        let top_set = TopSet::new(player(0), non_wild_rank(1), count(3));

        assert_eq!(top_set.player, player(0));
        assert_eq!(top_set.rank, non_wild_rank(1));
        assert_eq!(top_set.number, count(3));
    }

    #[test]
    fn top_set_new_allows_number_one() {
        assert_has_at_least_one_non_wild_rank();

        let top_set = TopSet::new(player(0), non_wild_rank(1), count(1));

        assert_eq!(top_set.number, count(1));
    }

    #[test]
    fn top_set_new_allows_max_total_play_number() {
        assert_has_at_least_one_non_wild_rank();

        let top_set = TopSet::new(player(0), non_wild_rank(1), count(MAX_TOTAL_PLAY));

        assert_eq!(top_set.number, count(MAX_TOTAL_PLAY));
    }

    #[test]
    fn top_set_is_copy_clone_eq_and_hashable() {
        assert_has_at_least_one_non_wild_rank();

        let top_set = TopSet::new(player(0), non_wild_rank(1), count(3));
        let copied = top_set;
        let cloned = top_set.clone();

        assert_eq!(top_set, copied);
        assert_eq!(top_set, cloned);
        assert_eq!(hash_value(&top_set), hash_value(&copied));
    }

    #[test]
    fn top_set_different_players_are_not_equal() {
        assert_has_at_least_one_non_wild_rank();

        if crate::consts::MAX_PLAYERS < 2 {
            return;
        }

        let first = TopSet::new(player(0), non_wild_rank(1), count(3));
        let second = TopSet::new(player(1), non_wild_rank(1), count(3));

        assert_ne!(first, second);
    }

    #[test]
    fn top_set_different_ranks_are_not_equal() {
        assert!(
            crate::consts::MAX_CARD_ORDINALITY > 2,
            "test requires at least two non-wild ranks"
        );

        let first = TopSet::new(player(0), non_wild_rank(1), count(3));
        let second = TopSet::new(player(0), non_wild_rank(2), count(3));

        assert_ne!(first, second);
    }

    #[test]
    fn top_set_different_numbers_are_not_equal() {
        assert_has_at_least_one_non_wild_rank();

        let first = TopSet::new(player(0), non_wild_rank(1), count(2));
        let second = TopSet::new(player(0), non_wild_rank(1), count(3));

        assert_ne!(first, second);
    }

    #[test]
    fn top_set_has_expected_size_and_alignment() {
        assert_eq!(
            size_of::<TopSet>(),
            size_of::<PlayerID>() + size_of::<CardRank>() + size_of::<CardCount>()
        );

        assert_eq!(
            align_of::<TopSet>(),
            align_of::<PlayerID>()
                .max(align_of::<CardRank>())
                .max(align_of::<CardCount>())
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn top_set_new_rejects_wild_rank() {
        let _ = TopSet::new(player(0), CardRank::WILD, count(1));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn top_set_new_rejects_zero_number() {
        assert_has_at_least_one_non_wild_rank();

        let _ = TopSet::new(player(0), non_wild_rank(1), count(0));
    }

    #[test]
    fn trick_new_has_no_top_set() {
        let trick = Trick::new();

        assert_eq!(trick.top_set, None);
    }

    #[test]
    fn trick_new_marks_no_players_as_passed() {
        let trick = Trick::new();

        for (_, has_passed) in trick.has_passed.iter_active(crate::consts::MAX_PLAYERS) {
            assert!(!*has_passed);
        }
    }

    #[test]
    fn trick_new_has_all_passed_slots_false() {
        let trick = Trick::new();

        assert!(trick.has_passed.iter().all(|has_passed| !*has_passed));
    }

    #[test]
    fn trick_fields_can_store_top_set_and_pass_state() {
        assert_has_at_least_one_non_wild_rank();

        let mut trick = Trick::new();
        let top_set = TopSet::new(player(0), non_wild_rank(1), count(2));

        trick.top_set = Some(top_set);
        trick.has_passed[player(0)] = true;

        assert_eq!(trick.top_set, Some(top_set));
        assert!(trick.has_passed[player(0)]);
    }

    #[test]
    fn trick_clone_preserves_top_set_and_pass_state() {
        assert_has_at_least_one_non_wild_rank();

        let mut trick = Trick::new();
        let top_set = TopSet::new(player(0), non_wild_rank(1), count(2));

        trick.top_set = Some(top_set);
        trick.has_passed[player(0)] = true;

        let cloned = trick.clone();

        assert_eq!(trick, cloned);
        assert_eq!(cloned.top_set, Some(top_set));
        assert!(cloned.has_passed[player(0)]);
    }

    #[test]
    fn trick_equality_distinguishes_top_set() {
        assert_has_at_least_one_non_wild_rank();

        let empty = Trick::new();

        let mut with_top_set = Trick::new();
        with_top_set.top_set = Some(TopSet::new(player(0), non_wild_rank(1), count(1)));

        assert_ne!(empty, with_top_set);
    }

    #[test]
    fn trick_equality_distinguishes_pass_state() {
        let first = Trick::new();

        let mut second = Trick::new();
        second.has_passed[player(0)] = true;

        assert_ne!(first, second);
    }

    #[test]
    fn trick_is_hashable_consistently_with_equality() {
        let first = Trick::new();
        let second = Trick::new();

        assert_eq!(first, second);
        assert_eq!(hash_value(&first), hash_value(&second));
    }

    #[test]
    fn trick_has_expected_size_and_alignment() {
        assert_eq!(
            size_of::<Trick>(),
            size_of::<Option<TopSet>>() + size_of::<PlayerIndexed<bool>>()
        );

        assert_eq!(
            align_of::<Trick>(),
            align_of::<Option<TopSet>>().max(align_of::<PlayerIndexed<bool>>())
        );
    }
}
