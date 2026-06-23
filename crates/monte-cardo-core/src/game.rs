use std::ops::{Add, AddAssign, Index, IndexMut, Sub, SubAssign};

use rand::{rngs::SmallRng, RngExt};
use thiserror::Error;

use crate::consts;

#[derive(Error, Debug)]
pub enum GameLogicError {
    #[error("Tried to apply invalid move for given game state: {0}")]
    InvalidMove(String),
}

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
    fn new() -> Trick {
        return Trick {
            top_set: None,
            has_passed: PlayerIndexed::filled(false),
        };
    }
}

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

pub fn all_players_have_passed(
    has_passed: &PlayerIndexed<bool>,
    player_placements: &PlayerPlacements,
    number_of_players: usize,
    top_set_player: PlayerID,
) -> bool {
    // Check if all players have passed (except the top set player)
    for (player, has_passed) in has_passed.iter_active(number_of_players) {
        if player == top_set_player {
            continue;
        }

        if player_placements.is_out(player) {
            continue;
        }

        if !has_passed {
            return false;
        }
    }

    return true;
}

pub fn update_full_information_game_state(
    game_state: &mut FullInformationGameState,
    player_move: Move,
) -> Result<bool, GameLogicError> {
    match player_move {
        Move::Play(play) => {
            // Update the player's hand
            game_state.player_hands[game_state.current_player_number][CardRank::WILD] -=
                play.num_wilds;
            game_state.player_hands[game_state.current_player_number][play.rank] -=
                play.num_non_wilds;

            // Update the top set
            game_state.trick.top_set = Some(TopSet::new(
                game_state.current_player_number,
                play.rank,
                play.total_count(),
            ));

            // Check if the player is out
            if game_state.player_hands[game_state.current_player_number].is_empty() {
                game_state
                    .player_placements
                    .mark_out(game_state.current_player_number);
            }

            // Reset the has_passed array
            game_state.trick.has_passed = PlayerIndexed::filled(false);

            // Update the player number
            match game_state.player_placements.get_next_active_player(
                game_state.current_player_number,
                game_state.number_of_players,
            ) {
                Some(player_number) => {
                    game_state.current_player_number = player_number;
                    return Ok(false);
                }
                None => return Ok(true),
            };
        }
        Move::Pass => {
            // Update the has_passed array
            game_state.trick.has_passed[game_state.current_player_number] = true;

            let top_set_player = match game_state.trick.top_set {
                Some(top_set) => top_set.player,
                None => {
                    return Err(GameLogicError::InvalidMove(
                        "Tried to pass on empty trick!".to_string(),
                    ))
                }
            };

            if all_players_have_passed(
                &game_state.trick.has_passed,
                &game_state.player_placements,
                game_state.number_of_players,
                top_set_player,
            ) {
                // Start a new trick

                // Reset the has_passed array
                game_state.trick.has_passed = PlayerIndexed::filled(false);

                // Reset the top set
                let trick_winner = top_set_player;
                game_state.trick.top_set = None;

                // Update the player number
                if game_state.player_placements.is_out(trick_winner) {
                    // Player still in after trick winner starts the next trick
                    match game_state
                        .player_placements
                        .get_next_active_player(trick_winner, game_state.number_of_players)
                    {
                        Some(player_number) => {
                            game_state.current_player_number = player_number;
                            return Ok(false);
                        }
                        None => return Ok(true),
                    };
                } else {
                    // Trick winner starts the next trick
                    game_state.current_player_number = trick_winner;
                    return Ok(false);
                }
            } else {
                // Update the player number
                game_state.current_player_number = game_state
                    .player_placements
                    .get_next_active_player(
                        game_state.current_player_number,
                        game_state.number_of_players,
                    )
                    .unwrap();
                return Ok(false);
            }
        }
    }
}

pub fn update_incomplete_information_game_state(
    game_state: &mut IncompleteInformationGameState,
    player_move: Move,
) -> Result<(), GameLogicError> {
    match player_move {
        Move::Play(play) => {
            if game_state.current_player_number != game_state.perspective_player_number {
                // Update the opponent's hand
                game_state.opponent_cards[CardRank::WILD] -= play.num_wilds;
                game_state.opponent_cards[play.rank] -= play.num_non_wilds;
            } else {
                // Update the player's hand
                game_state.player_hand[CardRank::WILD] -= play.num_wilds;
                game_state.player_hand[play.rank] -= play.num_non_wilds;
            }

            // Update hand sizes
            game_state
                .hand_sizes
                .remove_cards(game_state.current_player_number, play.total_count());
            // Update the top set
            game_state.trick.top_set = Some(TopSet::new(
                game_state.current_player_number,
                play.rank,
                play.total_count(),
            ));

            // Check if the player is out
            if game_state
                .hand_sizes
                .is_empty(game_state.current_player_number)
            {
                game_state
                    .player_placements
                    .mark_out(game_state.current_player_number);
            }

            // Reset the has_passed array
            game_state.trick.has_passed = PlayerIndexed::filled(false);

            // Update the player number
            match game_state.player_placements.get_next_active_player(
                game_state.current_player_number,
                game_state.number_of_players,
            ) {
                Some(player_number) => {
                    game_state.current_player_number = player_number;
                }
                None => {}
            };
        }
        Move::Pass => {
            // Update the has_passed array
            game_state.trick.has_passed[game_state.current_player_number] = true;

            let top_set_player = match game_state.trick.top_set {
                Some(top_set) => top_set.player,
                None => {
                    return Err(GameLogicError::InvalidMove(
                        "Tried to pass on empty trick!".to_string(),
                    ))
                }
            };

            // Check if all players have passed (except the top set player)
            if all_players_have_passed(
                &game_state.trick.has_passed,
                &game_state.player_placements,
                game_state.number_of_players,
                top_set_player,
            ) {
                // Start a new trick

                // Reset the has_passed array
                game_state.trick.has_passed = PlayerIndexed::filled(false);

                // Reset the top set
                let trick_winner = top_set_player;
                game_state.trick.top_set = None;

                // Update the player number
                if game_state.player_placements.is_out(trick_winner) {
                    // Player still in after trick winner starts the next trick
                    match game_state
                        .player_placements
                        .get_next_active_player(trick_winner, game_state.number_of_players)
                    {
                        Some(player_number) => {
                            game_state.current_player_number = player_number;
                        }
                        None => {}
                    };
                } else {
                    // Trick winner starts the next trick
                    game_state.current_player_number = trick_winner;
                }
            } else {
                // Update the player number
                game_state.current_player_number = game_state
                    .player_placements
                    .get_next_active_player(
                        game_state.current_player_number,
                        game_state.number_of_players,
                    )
                    .unwrap();
            }
        }
    }

    return Ok(());
}

pub fn get_available_moves(hand: &PlayerHand, top_set: Option<TopSet>) -> Vec<Move> {
    match top_set {
        Some(top_set) => {
            // We must play something from our hand with the same card number
            // and a lower card type
            let mut moves = Vec::new();
            let num_wilds = hand[CardRank::WILD];

            for rank in CardRank::non_wilds_below(top_set.rank) {
                let available_non_wilds = hand[rank];

                if available_non_wilds + num_wilds < top_set.number {
                    continue;
                }

                let max_non_wilds_playable = available_non_wilds.min(top_set.number);

                for num_non_wilds_played in CardCount::choices_largest_first(max_non_wilds_playable)
                {
                    let num_wilds_needed = top_set.number - num_non_wilds_played;
                    if num_wilds_needed > num_wilds {
                        break;
                    }

                    moves.push(Move::Play(Play::new(
                        rank,
                        num_non_wilds_played,
                        num_wilds_needed,
                    )));
                }
            }
            moves.push(Move::Pass);
            moves
        }
        None => {
            let mut moves = Vec::new();
            let num_wilds_available = hand[CardRank::WILD];

            for rank in CardRank::non_wilds() {
                let num_non_wilds_available = hand[rank];

                for num_non_wilds_played in CardCount::choices(num_non_wilds_available) {
                    for num_wilds_played in CardCount::choices(num_wilds_available) {
                        if num_non_wilds_played.is_zero() && num_wilds_played.is_zero() {
                            continue;
                        }

                        moves.push(Move::Play(Play::new(
                            rank,
                            num_non_wilds_played,
                            num_wilds_played,
                        )));
                    }
                }
            }
            moves
        }
    }
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

pub fn create_incomplete_information_game_state(
    full_information_game_state: &FullInformationGameState,
    perspective_player_number: PlayerID,
) -> IncompleteInformationGameState {
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
