use std::ops::{Index, IndexMut};

use thiserror::Error;

use crate::game::{self, CardCount, CardRank, PlayerHand, TopSet};
use crate::{consts, eval};

// The assumption is that we only consider playing the minimum number of wilds. Using all the wilds and all of the cards the max you could play in one go is consts::MAX_CARD_NUMBER * 2
pub const NUM_ACTIONS: usize = 1 + consts::MAX_CARD_ORDINALITY * game::MAX_TOTAL_PLAY;

#[derive(Error, Debug)]
pub enum MoveIDError {
    #[error("The provided card count {0:?} is out of range")]
    CardCountOutOfRange(CardCount),
    #[error("The provided move is not playable from the given hand. Move is {count:?} cards of rank {rank:?} but hand contains only {available_wilds:?} wilds and {available_non_wilds:?} available non-wilds.")]
    NotPlayableFromHand {
        count: game::CardCount,
        rank: game::CardRank,
        available_wilds: game::CardCount,
        available_non_wilds: game::CardCount,
    },
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveID(usize);

impl MoveID {
    pub const PASS: MoveID = MoveID(0);

    #[inline]
    pub fn new(value: usize) -> Self {
        debug_assert!(value < NUM_ACTIONS);
        Self(value)
    }

    #[inline]
    pub fn get(self) -> usize {
        self.0
    }

    pub fn from_count_and_rank(count: CardCount, rank: CardRank) -> Result<Self, MoveIDError> {
        if count == game::CardCount::new(0) || count > game::CardCount::new(game::MAX_TOTAL_PLAY) {
            return Err(MoveIDError::CardCountOutOfRange(count));
        }

        let id = consts::MAX_CARD_ORDINALITY * (count.get() - 1) + rank.get() + 1;
        debug_assert!(id < NUM_ACTIONS);

        Ok(Self(id))
    }

    pub fn from_move(game_move: &game::Move) -> Result<Self, MoveIDError> {
        match game_move {
            game::Move::Pass => Ok(MoveID::PASS),
            game::Move::Play(game::Play {
                rank,
                num_non_wilds,
                num_wilds,
            }) => return MoveID::from_count_and_rank(*num_non_wilds + *num_wilds, *rank),
        }
    }

    pub fn to_count_and_rank(&self) -> (CardCount, CardRank) {
        let play_id = self.0 - 1;
        let (num_to_play, rank_to_play) = (
            play_id / (consts::MAX_CARD_ORDINALITY) + 1,
            play_id % (consts::MAX_CARD_ORDINALITY),
        );
        return (CardCount::new(num_to_play), CardRank::new(rank_to_play));
    }

    pub fn to_move(self, current_hand: &game::PlayerHand) -> Result<game::Move, MoveIDError> {
        if self == MoveID::PASS {
            return Ok(game::Move::Pass);
        }

        let (num_to_play, rank_to_play) = self.to_count_and_rank();

        debug_assert!(rank_to_play.get() < consts::MAX_CARD_ORDINALITY);
        debug_assert!(num_to_play > CardCount::new(0));
        debug_assert!(num_to_play <= CardCount::new(game::MAX_TOTAL_PLAY));

        let available_to_play = current_hand[rank_to_play];
        let wilds = current_hand[game::CardRank::WILD];

        if available_to_play + wilds < num_to_play {
            return Err(MoveIDError::NotPlayableFromHand {
                count: num_to_play,
                rank: rank_to_play,
                available_wilds: wilds,
                available_non_wilds: available_to_play,
            });
        }

        let wilds_to_use = num_to_play.get().saturating_sub(available_to_play.get());
        let wilds_to_use = game::CardCount::new(wilds_to_use);
        let non_wilds_to_use = num_to_play - wilds_to_use;

        return Ok(game::Move::Play(game::Play {
            rank: rank_to_play,
            num_non_wilds: non_wilds_to_use,
            num_wilds: wilds_to_use,
        }));
    }

    pub fn all() -> impl Iterator<Item = Self> {
        return (0..NUM_ACTIONS).map(MoveID::new);
    }

    pub fn all_non_pass() -> impl Iterator<Item = Self> {
        return (1..NUM_ACTIONS).map(MoveID::new);
    }
}

impl eval::RankCompressible for MoveID {
    fn rank_compress(
        &self,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<eval::RankCompressed<Self>, eval::NormalizationError> {
        return rank_compression_map.compress_move_id(*self);
    }

    fn rank_decompress(
        compressed: &eval::RankCompressed<Self>,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<Self, eval::NormalizationError> {
        return rank_compression_map.decompress_move_id(compressed);
    }
}

#[derive(Debug, Clone)]
pub struct ActionMask([bool; NUM_ACTIONS]);

impl ActionMask {
    pub fn empty() -> Self {
        return Self([false; NUM_ACTIONS]);
    }

    pub fn num_valid(&self) -> usize {
        return self.0.iter().filter(|&&x| x).count();
    }

    pub fn from_hand_and_top(player_hand: &PlayerHand, top_set: &Option<TopSet>) -> Self {
        let mut valid_action_mask = [false; NUM_ACTIONS];
        for available_move in game::get_available_moves(player_hand, top_set) {
            valid_action_mask[MoveID::from_move(&available_move)
                .expect("get_available_moves returned an invalid output")
                .get()] = true;
        }
        return Self(valid_action_mask);
    }

    pub fn iter(&self) -> impl Iterator<Item = &bool> {
        self.0.iter()
    }
}

impl Index<MoveID> for ActionMask {
    type Output = bool;

    fn index(&self, index: MoveID) -> &Self::Output {
        return &self.0[index.get()];
    }
}

impl IndexMut<MoveID> for ActionMask {
    fn index_mut(&mut self, index: MoveID) -> &mut Self::Output {
        return &mut self.0[index.get()];
    }
}

impl eval::RankCompressible for ActionMask {
    fn rank_compress(
        &self,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<eval::RankCompressed<Self>, eval::NormalizationError> {
        let mut compressed_action_mask = ActionMask::empty();

        for uncompressed_move_id in MoveID::all() {
            match uncompressed_move_id.rank_compress(rank_compression_map) {
                Ok(compressed_id) => {
                    compressed_action_mask[*compressed_id.inner()] = self[uncompressed_move_id]
                }
                Err(_) => {}
            }
        }

        return Ok(eval::RankCompressed::new_unchecked(compressed_action_mask));
    }

    fn rank_decompress(
        compressed: &eval::RankCompressed<Self>,
        rank_compression_map: &eval::RankCompressionMap,
    ) -> Result<Self, eval::NormalizationError> {
        let mut decompressed_action_mask = ActionMask::empty();

        for compressed_move_id in MoveID::all() {
            let compressed_move_id = eval::RankCompressed::new_unchecked(compressed_move_id);
            match MoveID::rank_decompress(&compressed_move_id, rank_compression_map) {
                Ok(decompressed_id) => {
                    decompressed_action_mask[decompressed_id] =
                        compressed.inner()[*compressed_move_id.inner()];
                }
                Err(_) => {}
            }
        }

        return Ok(decompressed_action_mask);
    }
}
