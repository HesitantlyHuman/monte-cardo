use crate::consts;
use crate::eval::actions::{ActionMask, MoveID};
use crate::eval::config::ActionPriorHeuristic;
use crate::eval::normalize::NormalizedIncompleteInformation;
use crate::eval::puct::ActionProbabilities;
use crate::game::{
    CardRank, IncompleteInformationGameState, Move, PlayerHand, PlayerID, PlayerPlacements,
    MAX_TOTAL_PLAY,
};

pub struct NaiveHeuristic {}

impl NaiveHeuristic {
    pub fn new() -> Self {
        Self {}
    }
}

impl ActionPriorHeuristic for NaiveHeuristic {
    fn action_priors(&mut self, _: &NormalizedIncompleteInformation) -> ActionProbabilities {
        return ActionProbabilities::ones();
    }
}

pub struct SimpleHeuristic {
    has_beating_play: f32,
    contains_low_value: f32,
    contains_high_value: f32,
    temperature: f32,
}

impl SimpleHeuristic {
    pub fn new(
        has_beating_play: f32,
        contains_low_value: f32,
        contains_high_value: f32,
        temperature: f32,
    ) -> Self {
        debug_assert!(temperature > 0.0);
        debug_assert!(has_beating_play > 0.0);
        debug_assert!(contains_low_value > 0.0);
        debug_assert!(contains_high_value > 0.0);

        return SimpleHeuristic {
            has_beating_play: has_beating_play,
            contains_low_value: contains_low_value,
            contains_high_value: contains_high_value,
            temperature: temperature,
        };
    }

    pub fn default() -> Self {
        return Self::new(5.0, 2.0, 0.0, 0.9);
    }
}

impl ActionPriorHeuristic for SimpleHeuristic {
    fn action_priors(&mut self, state: &NormalizedIncompleteInformation) -> ActionProbabilities {
        let can_beat = player_can_beat_array(&state.player_hand, &state.opponent_cards);

        let card_values = linear_card_values(self.contains_high_value, self.contains_low_value);

        let mut probs = ActionProbabilities::zeros();
        let valid_actions = ActionMask::from_hand_and_top(&state.player_hand, &state.trick.top_set);
        for move_id in MoveID::all() {
            if !valid_actions[move_id] {
                continue;
            }

            let candidate_move = move_id
                .to_move(&state.player_hand)
                .expect("ActionMask gave an invalid MoveID");

            let play = match candidate_move {
                Move::Play(play) => play,
                Move::Pass => continue,
            };

            if can_beat[play.total_count().get()] {
                probs[move_id] += self.has_beating_play
            }

            probs[move_id] += card_values[play.rank.get()] * play.total_count().get() as f32;
        }

        let max_val = probs
            .iter()
            .fold(f32::NEG_INFINITY, |current_max, &x| current_max.max(x));
        let mut total = 0.0;
        for move_id in MoveID::all() {
            let scaled = (probs[move_id] - max_val) / self.temperature;
            let weight = scaled.exp();

            probs[move_id] = weight;
            total += weight;
        }

        debug_assert!(total > 0.0 && total.is_finite());

        for move_id in MoveID::all() {
            probs[move_id] /= total;
        }

        return probs;
    }
}

fn player_can_beat_array(
    player_hand: &PlayerHand,
    opponent_cards: &PlayerHand,
) -> [bool; MAX_TOTAL_PLAY] {
    let mut can_beat = [false; MAX_TOTAL_PLAY];
    let mut fill_pointer = 0;
    for card_rank in CardRank::all() {
        let max_player_play_count = player_hand[CardRank::WILD] + player_hand[card_rank];
        let max_opponent_play_count = opponent_cards[CardRank::WILD] + opponent_cards[card_rank];

        if max_player_play_count > max_opponent_play_count {
            while fill_pointer < max_player_play_count.get() {
                can_beat[fill_pointer] = true;
                fill_pointer += 1;
            }
        }

        fill_pointer = max_player_play_count
            .get()
            .max(max_opponent_play_count.get())
            + 1;

        if fill_pointer == MAX_TOTAL_PLAY {
            break;
        }
    }
    return can_beat;
}

fn linear_card_values(
    high_card_value: f32,
    low_card_value: f32,
) -> [f32; consts::MAX_CARD_ORDINALITY] {
    let mut card_values = [0.0; consts::MAX_CARD_ORDINALITY];
    for card_index in 0..consts::MAX_CARD_ORDINALITY {
        let normalized_card_index = card_index as f32 / consts::MAX_CARD_ORDINALITY as f32;
        card_values[card_index] = (1.0 - normalized_card_index) * high_card_value
            + normalized_card_index * low_card_value;
    }
    return card_values;
}
