use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::consts;

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
