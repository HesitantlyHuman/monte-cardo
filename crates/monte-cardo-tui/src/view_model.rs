use monte_cardo_core::game as core_game;

use crate::{cards, game, hand, players, table};

pub fn core_move_to_card_move(player_move: core_game::Move) -> Option<cards::Move> {
    match player_move {
        core_game::Move::Play(play) => Some(cards::Move::new(
            play.rank.get() as u8,
            play.num_wilds.get() as u8,
            play.num_non_wilds.get() as u8,
        )),
        core_game::Move::Pass => None,
    }
}

pub fn card_move_to_core_move(player_move: cards::Move) -> core_game::Move {
    core_game::Move::Play(core_game::Play::new(
        core_game::CardRank::new(player_move.rank as usize),
        core_game::CardCount::new(player_move.num_wilds.into()),
        core_game::CardCount::new(player_move.num_non_wilds.into()),
    ))
}

pub fn build_game_widget(
    num_players: usize,
    ui_player_number: usize,
    current_selected_move: Option<usize>,
    ai_suggestion: Option<core_game::Move>,
    game_state: &core_game::FullInformationGameState,
    trick_history: &[cards::TrickHistoryEntry],
    player_names: &[String],
) -> game::GameState {
    let ui_player_id = core_game::PlayerID::new(ui_player_number);

    let mut player_widgets = Vec::new();

    for player_id in core_game::PlayerID::all_player_ids(num_players) {
        let player_hand = &game_state.player_hands[player_id];
        let player_name = player_names[player_id.get()].clone();

        let mut hand_size = 0;
        for card_count in player_hand.iter() {
            hand_size += card_count.get() as u16;
        }

        let is_leading = match &game_state.trick.top_set {
            Some(top_set) => top_set.player == player_id,
            None => false,
        };

        let player_state = if player_id == game_state.current_player_number {
            players::PlayerState::Active
        } else if hand_size == 0 {
            if is_leading {
                players::PlayerState::LeadingOut
            } else {
                players::PlayerState::NormalOut
            }
        } else if game_state.trick.has_passed[player_id] {
            players::PlayerState::Passed
        } else if is_leading {
            players::PlayerState::Leading
        } else {
            players::PlayerState::Normal
        };

        player_widgets.push(players::Player::new(player_name, player_state, hand_size));
    }

    let current_player = if ui_player_id == game_state.current_player_number {
        table::Player::PerspectivePlayer
    } else {
        table::Player::Other(player_names[game_state.current_player_number.get()].clone())
    };

    let top_set = if !trick_history.is_empty() {
        let current_leader_number = game_state
            .trick
            .top_set
            .expect("Trick history exists, but top set is empty")
            .player;

        let top_set_player = if current_leader_number == ui_player_id {
            table::Player::PerspectivePlayer
        } else {
            table::Player::Other(player_names[current_leader_number.get()].clone())
        };

        Some(table::TopSet::new(
            trick_history[0].player_move,
            top_set_player,
        ))
    } else {
        None
    };

    let table = table::Table::new(current_player, top_set);

    let player_hand = &game_state.player_hands[ui_player_id];

    let mut available_moves =
        core_game::get_available_moves(player_hand, &game_state.trick.top_set);
    available_moves.reverse();

    let mut converted_moves = Vec::new();

    for player_move in available_moves {
        if let Some(card_move) = core_move_to_card_move(player_move) {
            converted_moves.push(card_move);
        }
    }

    let suggested_move = match ai_suggestion {
        Some(core_game::Move::Play(play)) => {
            hand::SuggestedMove::Suggestion(hand::MoveSuggestion::Move(cards::Move::new(
                play.rank.get() as u8,
                play.num_wilds.get() as u8,
                play.num_non_wilds.get() as u8,
            )))
        }
        Some(core_game::Move::Pass) => hand::SuggestedMove::Suggestion(hand::MoveSuggestion::Pass),
        None => hand::SuggestedMove::Disabled,
    };

    let player_hand_widget = if game_state.current_player_number == ui_player_id {
        hand::PlayerHand::CurrentTurn(hand::PlayerTurnHand::new(
            game_state.trick.top_set.is_some(),
            player_hand.to_usize_counts().map(|x| x as u8),
            suggested_move,
            converted_moves,
            current_selected_move,
        ))
    } else {
        hand::PlayerHand::NotPlayerTurn(player_hand.to_usize_counts().map(|x| x as u8))
    };

    game::GameState::new(
        players::Players::new(player_widgets),
        table,
        cards::TrickHistory::new(trick_history.to_vec()),
        player_hand_widget,
    )
}
