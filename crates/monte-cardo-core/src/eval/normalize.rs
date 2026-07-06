use thiserror::Error;

use crate::consts;
use crate::eval::actions::MoveID;
use crate::game;
use crate::game::CardCount;
use crate::game::CardRank;
use crate::game::TopSet;

#[derive(Error, Debug)]
pub enum NormalizationError {
    #[error("PlayerHand has card rank which is not in RankCompressionMap: {0}")]
    RankCompressionError(String),
    #[error("CompressedPlayerHand has card rank which is not in RankCompressionMap: {0}")]
    RankDecompressionError(String),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NormalizedIncompleteInformation {
    pub number_of_players: usize,
    pub player_hand: RankCompressed<game::PlayerHand>,
    pub opponent_cards: RankCompressed<game::PlayerHand>,
    pub hand_sizes: game::HandSizes,
    pub trick: RankCompressed<game::Trick>,
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RankCompressed<T>(T);

impl<T> RankCompressed<T> {
    #[inline]
    pub(crate) fn new_unchecked(value: T) -> Self {
        return Self(value);
    }

    #[inline]
    pub(crate) fn inner(&self) -> &T {
        return &self.0;
    }

    #[inline]
    pub(crate) fn inner_mut(&mut self) -> &mut T {
        return &mut self.0;
    }
}

#[derive(Debug, Clone)]
pub struct RankCompressionMap {
    rank_original_to_compressed: [Option<game::CardRank>; consts::MAX_CARD_ORDINALITY],
    rank_compressed_to_original: [Option<game::CardRank>; consts::MAX_CARD_ORDINALITY],
    // action_original_to_compressed: [Option<MoveID>; NUM_ACTIONS],
    // action_compressed_to_original: [Option<MoveID>; NUM_ACTIONS],
}

pub trait RankCompressible: Sized {
    fn rank_compress(
        &self,
        rank_compression_map: &RankCompressionMap,
    ) -> Result<RankCompressed<Self>, NormalizationError>;

    fn rank_decompress(
        compressed: &RankCompressed<Self>,
        rank_compression_map: &RankCompressionMap,
    ) -> Result<Self, NormalizationError>;
}

impl RankCompressionMap {
    pub fn from_hands_and_top_set(
        player_hand: &game::PlayerHand,
        opponent_cards: &game::PlayerHand,
        top_set: &Option<TopSet>,
    ) -> Self {
        // First the rank mapping
        let mut rank_original_to_compressed = [None; consts::MAX_CARD_ORDINALITY];
        let mut rank_compressed_to_original = [None; consts::MAX_CARD_ORDINALITY];

        rank_original_to_compressed[CardRank::WILD.get()] = Some(CardRank::WILD);
        rank_compressed_to_original[CardRank::WILD.get()] = Some(CardRank::WILD);

        let mut compressed_index = 1;
        for uncompressed_rank in CardRank::non_wilds() {
            // If either player has this rank, or the top set is this rank, then we need to keep it.
            // Otherwise, compress it.
            if player_hand[uncompressed_rank] + opponent_cards[uncompressed_rank]
                == CardCount::new(0)
                && top_set.is_none_or(|set| set.rank != uncompressed_rank)
            {
                continue;
            }

            rank_original_to_compressed[uncompressed_rank.get()] =
                Some(CardRank::new(compressed_index));
            rank_compressed_to_original[compressed_index] = Some(uncompressed_rank);

            compressed_index += 1;
        }

        // // Now the action space mapping
        // let mut action_original_to_compressed = [None; NUM_ACTIONS];
        // let mut action_compressed_to_original = [None; NUM_ACTIONS];

        // action_original_to_compressed[0] = Some(MoveID::PASS);
        // action_compressed_to_original[0] = Some(MoveID::PASS);

        // for action_id in MoveID::all_non_pass() {
        //     let (count, rank) = action_id.to_count_and_rank();
        //     let compressed_rank = match rank_original_to_compressed[rank.get()] {
        //         Some(compressed_rank) => compressed_rank,
        //         None => continue,
        //     };
        //     let compressed_move_id = MoveID::from_count_and_rank(count, compressed_rank)
        //         .expect("MoveID::all_non_pass gave invalid count and rank!");
        //     action_original_to_compressed[action_id.get()] = Some(compressed_move_id);
        //     action_compressed_to_original[compressed_move_id.get()] = Some(action_id);
        // }

        // // Sanity checks
        // for original_action in MoveID::all() {
        //     if let Some(compressed_action) = action_original_to_compressed[original_action.get()] {
        //         let roundtrip = action_compressed_to_original[compressed_action.get()];

        //         debug_assert_eq!(
        //     roundtrip,
        //     Some(original_action),
        //     "Action compression roundtrip failed. original: {:?}, compressed: {:?}, decompressed: {:?}",
        //     original_action,
        //     compressed_action,
        //     roundtrip,
        // );
        //     }
        // }
        for original_rank in CardRank::all() {
            if let Some(compressed_rank) = rank_original_to_compressed[original_rank.get()] {
                let roundtrip = rank_compressed_to_original[compressed_rank.get()];

                debug_assert_eq!(
            roundtrip,
            Some(original_rank),
            "Rank compression roundtrip failed. original: {:?}, compressed: {:?}, decompressed: {:?}",
            original_rank,
            compressed_rank,
            roundtrip,
        );
            }
        }

        return Self {
            rank_original_to_compressed,
            rank_compressed_to_original,
            // action_compressed_to_original,
            // action_original_to_compressed,
        };
    }

    pub(crate) fn compress_rank(
        &self,
        card_rank: CardRank,
    ) -> Result<RankCompressed<CardRank>, NormalizationError> {
        match self.rank_original_to_compressed[card_rank.get()] {
            Some(rank) => return Ok(RankCompressed::new_unchecked(rank)),
            None => {
                return Err(NormalizationError::RankCompressionError(format!(
                    "Attempted to compress CardRank: {:?} which is not in map!",
                    card_rank
                )))
            }
        }
    }

    pub(crate) fn decompress_rank(
        &self,
        card_rank: &RankCompressed<CardRank>,
    ) -> Result<CardRank, NormalizationError> {
        match self.rank_compressed_to_original[card_rank.inner().get()] {
            Some(rank) => return Ok(rank),
            None => {
                return Err(NormalizationError::RankDecompressionError(format!(
                    "Attempted to decompress CardRank: {:?} which is not in map!",
                    card_rank.inner()
                )))
            }
        }
    }

    pub(crate) fn compress_move_id(
        &self,
        move_id: MoveID,
    ) -> Result<RankCompressed<MoveID>, NormalizationError> {
        if move_id == MoveID::PASS {
            return Ok(RankCompressed::new_unchecked(MoveID::PASS));
        }

        let (count, original_rank) = move_id.to_count_and_rank();
        let compressed_rank = self.compress_rank(original_rank)?;

        let compressed_move_id = MoveID::from_count_and_rank(count, *compressed_rank.inner())
            .map_err(|err| {
                NormalizationError::RankCompressionError(format!(
                    "Failed to compress MoveID {:?}: {}",
                    move_id, err
                ))
            })?;

        Ok(RankCompressed::new_unchecked(compressed_move_id))
    }

    pub(crate) fn decompress_move_id(
        &self,
        move_id: &RankCompressed<MoveID>,
    ) -> Result<MoveID, NormalizationError> {
        if *move_id.inner() == MoveID::PASS {
            return Ok(MoveID::PASS);
        }

        let (count, compressed_rank) = move_id.inner().to_count_and_rank();
        let compressed_rank = RankCompressed::new_unchecked(compressed_rank);
        let original_rank = self.decompress_rank(&compressed_rank)?;

        MoveID::from_count_and_rank(count, original_rank).map_err(|err| {
            NormalizationError::RankDecompressionError(format!(
                "Failed to decompress MoveID {:?}: {}",
                move_id.inner(),
                err
            ))
        })
    }
}

fn normalize_player_index(
    absolute_player: game::PlayerID,
    perspective_player: game::PlayerID,
    number_of_players: usize,
) -> game::PlayerID {
    debug_assert!(number_of_players > 0);
    debug_assert!(absolute_player.get() < number_of_players);
    debug_assert!(perspective_player.get() < number_of_players);

    return game::PlayerID::new(
        (absolute_player.get() + number_of_players - perspective_player.get()) % number_of_players,
    );
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
    trick: &game::Trick,
    current_player_number: game::PlayerID,
    number_of_players: usize,
) -> Result<game::Trick, NormalizationError> {
    let top_set = match trick.top_set {
        Some(set) => {
            let normalized_player_number =
                normalize_player_index(set.player, current_player_number, number_of_players);
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
        trick.has_passed.get(),
        &mut rotated_has_passed,
        number_of_players,
        current_player_number.get(),
    );

    return Ok(game::Trick {
        top_set: top_set,
        has_passed: game::PlayerIndexed::new(rotated_has_passed),
    });
}

pub fn normalize_incomplete_information_state(
    incomplete_information_state: &game::IncompleteInformationGameState,
) -> Result<(NormalizedIncompleteInformation, RankCompressionMap), NormalizationError> {
    let mut rotated_hand_sizes = [0; consts::MAX_PLAYERS];
    left_rotate_array(
        incomplete_information_state.hand_sizes.get(),
        &mut rotated_hand_sizes,
        incomplete_information_state.number_of_players,
        incomplete_information_state.current_player_number.get(),
    );

    let rank_compression_map = RankCompressionMap::from_hands_and_top_set(
        &incomplete_information_state.player_hand,
        &incomplete_information_state.opponent_cards,
        &incomplete_information_state.trick.top_set,
    );
    let normalized_player_hand = match incomplete_information_state
        .player_hand
        .rank_compress(&rank_compression_map)
    {
        Ok(compressed_hand) => compressed_hand,
        Err(err) => {
            return Err(NormalizationError::RankCompressionError(format!(
                "Error compressing player hand while normalizing incomplete information state: {}",
                err
            )))
        }
    };
    let normalized_opponent_cards = match incomplete_information_state
        .opponent_cards
        .rank_compress(&rank_compression_map)
    {
        Ok(compressed_hand) => compressed_hand,
        Err(err) => {
            return Err(NormalizationError::RankCompressionError(format!(
            "Error compressing opponent cards while normalizing incomplete information state: {}",
            err
        )))
        }
    };

    let normalized_trick = normalize_trick(
        &incomplete_information_state.trick,
        incomplete_information_state.current_player_number,
        incomplete_information_state.number_of_players,
    )?;
    let normalized_trick = normalized_trick.rank_compress(&rank_compression_map)?;

    return Ok((
        NormalizedIncompleteInformation {
            number_of_players: incomplete_information_state.number_of_players,
            player_hand: normalized_player_hand,
            opponent_cards: normalized_opponent_cards,
            hand_sizes: game::HandSizes::new(rotated_hand_sizes),
            trick: normalized_trick,
        },
        rank_compression_map,
    ));
}
