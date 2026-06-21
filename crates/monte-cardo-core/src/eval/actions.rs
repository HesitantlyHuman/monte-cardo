use crate::consts;
use crate::game::{self, IncompleteInformationGameState};

pub type ActionMask = [bool; NUM_ACTIONS];
pub type MoveID = usize;

pub const NUM_ACTIONS: usize = 1 + consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY * 2; // The assumption is that we only consider playing the minimum number of wilds. Using all the wilds and all of the cards the max you could play in one go is consts::MAX_CARD_NUMBER * 2

pub fn move_to_id(game_move: game::Move) -> MoveID {
    match game_move {
        game::Move::Pass => 0,
        game::Move::Play(game::Play {
            rank,
            num_non_wilds,
            num_wilds,
        }) => {
            let total_num_cards = num_non_wilds + num_wilds;
            debug_assert!(total_num_cards > 0);
            consts::MAX_CARD_ORDINALITY * (total_num_cards - 1) as usize + rank + 1
        }
    }
}

pub fn id_to_move(id: MoveID, hand: game::Hand) -> game::Move {
    if id == 0 {
        return game::Move::Pass;
    }

    let play_id = id - 1;
    let (num_to_play, rank_to_play) = (
        play_id / (consts::MAX_CARD_ORDINALITY) + 1,
        play_id % (consts::MAX_CARD_ORDINALITY),
    );

    let available_to_play = hand[rank_to_play as usize];
    let wilds = hand[0];
    debug_assert!(available_to_play + wilds >= num_to_play);
    let wilds_to_use = num_to_play.saturating_sub(available_to_play);
    let non_wilds_to_use = num_to_play - wilds_to_use;

    return game::Move::Play(game::Play {
        rank: rank_to_play,
        num_non_wilds: non_wilds_to_use,
        num_wilds: wilds_to_use,
    });
}

pub fn create_valid_action_mask(
    incomplete_information: IncompleteInformationGameState,
) -> ActionMask {
    let mut valid_action_mask = [false; NUM_ACTIONS];
    for available_move in game::get_available_moves(
        incomplete_information.player_hand,
        incomplete_information.trick.top_set,
    ) {
        valid_action_mask[move_to_id(available_move)] = true;
    }
    return valid_action_mask;
}
