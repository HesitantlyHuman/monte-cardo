use crate::ai::game;
use crate::consts;

const SWAP_DEPTH: usize = 4;
const TEMPERATURE: f32 = 2.0;

type ActionValueMatrix =
    [[f32; consts::MAX_PLAYERS]; consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY];
type ActionProbabilties = [f32; consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY];

fn value_to_probabilities(
    action_value_matrix: ActionValueMatrix,
    player: game::PlayerNumber,
) -> ActionProbabilties {
    [0.0; consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY]
}

fn choose_action(incomplete_information_state: game::IncompleteInformationGameState) -> game::Move {
    game::Move::Pass
}

fn full_tree_evaluation(
    incomplete_information_state: game::IncompleteInformationGameState,
    current_depth: usize,
) -> ActionValueMatrix {
    debug_assert_eq!(
        incomplete_information_state.current_player_number,
        incomplete_information_state.perspective_player_number
    );
    [[0.0; consts::MAX_PLAYERS]; consts::MAX_CARD_NUMBER * consts::MAX_CARD_ORDINALITY]
}

struct PUCTKey {
    player_hand: [u8; consts::MAX_CARD_ORDINALITY],
    opponent_cards: [u8; consts::MAX_CARD_ORDINALITY],
    hand_sizes: [u16; consts::MAX_PLAYERS],
    trick: game::Trick,
}

fn get_puct_key(incomplete_information_state: game::IncompleteInformationGameState) -> PUCTKey {
    PUCTKey {
        player_hand: incomplete_information_state.player_hand,
        opponent_cards: incomplete_information_state.opponent_cards,
        hand_sizes: incomplete_information_state.hand_sizes,
        trick: incomplete_information_state.trick,
    }
}
