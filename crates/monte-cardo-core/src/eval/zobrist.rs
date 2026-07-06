use crate::{
    consts,
    eval::normalize::NormalizedIncompleteInformation,
    game::{CardRank, MAX_TOTAL_PLAY},
};

#[derive(Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        let mut z = {
            self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            self.state
        };

        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZobristHash(u64);

const ZOBRIST_HAND_SIZE_MAX: usize = 100;

pub struct ZobristTable {
    // The convention is that a count of 0 is the value 0, so we don't need to do any hashing or store that value.
    player_hand: [[u64; consts::MAX_CARD_NUMBER - 1]; consts::MAX_CARD_ORDINALITY],
    opponent_hand: [[u64; consts::MAX_CARD_NUMBER - 1]; consts::MAX_CARD_ORDINALITY],
    // Once again, a convention that 0 does nothing.
    hand_sizes: [[u64; ZOBRIST_HAND_SIZE_MAX - 1]; consts::MAX_PLAYERS],
    has_passed: [u64; consts::MAX_PLAYERS],
    // For all top sets, not having a top set means not applying any values, hence we do not need a 0 count.
    // Also, since the top set cannot have a wild rank, we dont need a CardRank::WILD value
    top_set_player: [u64; consts::MAX_PLAYERS],
    top_set_rank: [u64; consts::MAX_CARD_ORDINALITY - 1],
    top_set_count: [u64; MAX_TOTAL_PLAY - 1],
}

impl ZobristTable {
    pub fn new(seed: u64) -> Self {
        let mut generator = SplitMix64::new(seed);

        let mut player_hand = [[0; consts::MAX_CARD_NUMBER - 1]; consts::MAX_CARD_ORDINALITY];
        let mut opponent_hands = [[0; consts::MAX_CARD_NUMBER - 1]; consts::MAX_CARD_ORDINALITY];

        for i in 0..consts::MAX_CARD_ORDINALITY {
            for j in 0..(consts::MAX_CARD_NUMBER - 1) {
                player_hand[i][j] = generator.next_u64();
                opponent_hands[i][j] = generator.next_u64();
            }
        }

        let mut hand_sizes = [[0; ZOBRIST_HAND_SIZE_MAX - 1]; consts::MAX_PLAYERS];
        let mut has_passed = [0; consts::MAX_PLAYERS];
        let mut top_set_player = [0; consts::MAX_PLAYERS];

        for i in 0..consts::MAX_PLAYERS {
            for j in 0..(ZOBRIST_HAND_SIZE_MAX - 1) {
                hand_sizes[i][j] = generator.next_u64();
            }
            has_passed[i] = generator.next_u64();
            top_set_player[i] = generator.next_u64();
        }

        let mut top_set_rank = [0; consts::MAX_CARD_ORDINALITY - 1];

        for i in 0..(consts::MAX_CARD_ORDINALITY - 1) {
            top_set_rank[i] = generator.next_u64();
        }

        let mut top_set_count = [0; MAX_TOTAL_PLAY - 1];

        for i in 0..(MAX_TOTAL_PLAY - 1) {
            top_set_count[i] = generator.next_u64();
        }

        Self {
            player_hand: player_hand,
            opponent_hand: opponent_hands,
            hand_sizes: hand_sizes,
            has_passed: has_passed,
            top_set_player: top_set_player,
            top_set_rank: top_set_rank,
            top_set_count: top_set_count,
        }
    }

    pub fn hash(&self, normalized_information: &NormalizedIncompleteInformation) -> ZobristHash {
        let mut hash = 0;

        for card_rank in CardRank::all() {
            let player_count = normalized_information.player_hand.inner()[card_rank];
            if player_count.get() > 0 {
                hash ^= self.player_hand[card_rank.get()][player_count.get() - 1];
            }

            let opponent_count = normalized_information.opponent_cards.inner()[card_rank];
            if opponent_count.get() > 0 {
                hash ^= self.opponent_hand[card_rank.get()][opponent_count.get() - 1];
            }
        }

        for (player, hand_size) in normalized_information.hand_sizes.get().iter().enumerate() {
            if *hand_size > 0 {
                hash ^= self.hand_sizes[player][*hand_size - 1];
            }
        }

        for (player, has_passed) in normalized_information
            .trick
            .inner()
            .has_passed
            .get()
            .iter()
            .enumerate()
        {
            if *has_passed {
                hash ^= self.has_passed[player];
            }
        }

        match normalized_information.trick.inner().top_set {
            Some(top_set) => {
                hash ^= self.top_set_player[top_set.player.get()];
                hash ^= self.top_set_rank[top_set.rank.get() - 1];
                hash ^= self.top_set_count[top_set.number.get() - 1];
            }
            None => {}
        }

        return ZobristHash(hash);
    }
}
