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
