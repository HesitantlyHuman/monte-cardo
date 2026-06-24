use std::ops::{Index, IndexMut};

use crate::consts;
use crate::game::primitives::{CardCount, CardRank, PlayerID};

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[repr(transparent)]
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
