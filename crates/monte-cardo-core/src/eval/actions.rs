use std::ops::Index;

use thiserror::Error;

use crate::consts;
use crate::game::{self, CardCount, IncompleteInformationGameState};

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
    #[inline]
    pub fn new(value: usize) -> Self {
        debug_assert!(value < NUM_ACTIONS);
        Self(value)
    }

    #[inline]
    pub fn get(self) -> usize {
        self.0
    }

    pub fn from_move(game_move: &game::Move) -> Result<Self, MoveIDError> {
        match game_move {
            game::Move::Pass => Ok(Self(0)),
            game::Move::Play(game::Play {
                rank,
                num_non_wilds,
                num_wilds,
            }) => {
                let total_count = *num_non_wilds + *num_wilds;
                debug_assert!(total_count > game::CardCount::new(0));

                if total_count == game::CardCount::new(0)
                    || total_count > game::CardCount::new(game::MAX_TOTAL_PLAY)
                {
                    return Err(MoveIDError::CardCountOutOfRange(total_count));
                }

                let id = consts::MAX_CARD_ORDINALITY * (total_count.get() - 1) + rank.get() + 1;
                debug_assert!(id < NUM_ACTIONS);

                Ok(Self(id))
            }
        }
    }

    pub fn to_move(self, current_hand: &game::PlayerHand) -> Result<game::Move, MoveIDError> {
        if self.0 == 0 {
            return Ok(game::Move::Pass);
        }

        let play_id = self.0 - 1;
        let (num_to_play, rank_to_play) = (
            play_id / (consts::MAX_CARD_ORDINALITY) + 1,
            play_id % (consts::MAX_CARD_ORDINALITY),
        );

        debug_assert!(rank_to_play < consts::MAX_CARD_ORDINALITY);
        debug_assert!(num_to_play > 0);
        debug_assert!(num_to_play <= game::MAX_TOTAL_PLAY);

        let num_to_play = game::CardCount::new(num_to_play);
        let rank_to_play = game::CardRank::new(rank_to_play);

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
}

#[derive(Debug, Clone)]
pub struct ActionMask([bool; NUM_ACTIONS]);

impl ActionMask {
    pub fn num_valid(&self) -> usize {
        return self.0.iter().filter(|&&x| x).count();
    }

    pub fn from_incomplete_information(
        incomplete_information: &IncompleteInformationGameState,
    ) -> Self {
        let mut valid_action_mask = [false; NUM_ACTIONS];
        for available_move in game::get_available_moves(
            &incomplete_information.player_hand,
            incomplete_information.trick.top_set,
        ) {
            valid_action_mask[MoveID::from_move(&available_move)
                .expect("get_available_moves returned an invalid output")
                .get()] = true;
        }
        return Self(valid_action_mask);
    }
}

impl Index<MoveID> for ActionMask {
    type Output = bool;

    fn index(&self, index: MoveID) -> &Self::Output {
        return &self.0[index.get()];
    }
}
