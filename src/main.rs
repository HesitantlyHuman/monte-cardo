mod ai;
mod consts;
mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::{
    io::{self, stdout},
    vec,
};

fn update_state_and_trick_history_with_ai_move(
    game_state: &mut ai::game::FullInformationGameState,
    player_names: &Vec<String>,
    trick_history: &mut Vec<ui::cards::TrickHistoryEntry>,
    heuristic: &dyn ai::monte_carlo::Heuristic,
) {
    let current_player = game_state.current_player_number;
    if game_state.player_is_out[current_player] {
        game_state.current_player_number = ai::game::get_next_active_player(
            &game_state.player_is_out,
            game_state.current_player_number,
        )
        .unwrap();
    }
    let incomplete_information_game_state =
        ai::game::create_incomplete_information_game_state(*game_state, current_player);
    let game_move =
        ai::monte_carlo::get_best_move(incomplete_information_game_state, heuristic, 20_000);
    ai::game::update_full_information_game_state(game_state, &game_move);
    match game_move {
        ai::game::Move::Play(play) => {
            let player_name = player_names[current_player].clone();
            let player_move = ui::cards::Move::new(
                play.rank as u8,
                play.num_wilds as u8,
                play.num_non_wilds as u8,
            );
            // Add the move to the trick history
            let trick_history_entry = ui::cards::TrickHistoryEntry::new(player_name, player_move);
            trick_history.insert(0, trick_history_entry);
        }
        ai::game::Move::Pass => {
            match game_state.trick.top_set {
                Some(_) => {}
                None => {
                    // AI passing just ended the trick
                    trick_history.clear();
                }
            }
        }
    }
}

fn update_state_and_trick_history_with_player_move(
    game_state: &mut ai::game::FullInformationGameState,
    player_names: &Vec<String>,
    trick_history: &mut Vec<ui::cards::TrickHistoryEntry>,
    move_: Option<ui::cards::Move>,
) {
    let game_move = match move_ {
        Some(m) => ai::game::Move::Play(ai::game::Play::new(
            m.rank as usize,
            m.num_wilds.into(),
            m.num_non_wilds.into(),
        )),
        None => ai::game::Move::Pass,
    };
    let current_player = game_state.current_player_number;
    ai::game::update_full_information_game_state(game_state, &game_move);
    match game_move {
        ai::game::Move::Play(play) => {
            let player_name = player_names[current_player].clone();
            let player_move = ui::cards::Move::new(
                play.rank as u8,
                play.num_wilds as u8,
                play.num_non_wilds as u8,
            );
            // Add to the front of the trick history
            let history_entry = ui::cards::TrickHistoryEntry::new(player_name, player_move);
            trick_history.insert(0, history_entry);
        }
        ai::game::Move::Pass => {
            match game_state.trick.top_set {
                Some(_) => {}
                None => {
                    // Player passing just ended the trick
                    trick_history.clear();
                }
            }
        }
    }
}

fn internal_state_to_ui_state(
    num_players: usize,
    ui_player_number: usize,
    current_selected_move: Option<usize>,
    game_state: ai::game::FullInformationGameState,
    trick_history: &Vec<ui::cards::TrickHistoryEntry>,
    player_names: &Vec<String>,
) -> ui::game::GameState {
    let mut players = vec![];
    for i in 0..num_players {
        let player_hand = &game_state.player_hands[i];
        let player_name = player_names[i].clone();

        let mut hand_size = 0;
        for card_count in player_hand {
            hand_size += *card_count as u16;
        }

        let player_state = if i == game_state.current_player_number {
            ui::players::PlayerState::Active
        } else if hand_size == 0 {
            match game_state.trick.top_set {
                Some(top_set) => {
                    if top_set.player == i {
                        ui::players::PlayerState::LeadingOut
                    } else {
                        ui::players::PlayerState::NormalOut
                    }
                }
                None => ui::players::PlayerState::NormalOut,
            }
        } else if game_state.trick.has_passed[i] {
            ui::players::PlayerState::Passed
        } else if game_state.trick.top_set.is_some()
            && game_state.trick.top_set.unwrap().player == i
        {
            ui::players::PlayerState::Leading
        } else {
            ui::players::PlayerState::Normal
        };

        players.push(ui::players::Player::new(
            player_name,
            player_state,
            hand_size,
        ));
    }
    let current_player = if ui_player_number == game_state.current_player_number {
        ui::table::Player::PerspectivePlayer
    } else {
        ui::table::Player::Other(player_names[game_state.current_player_number].clone())
    };
    let top_set = if trick_history.len() > 0 {
        let current_leader_number = game_state.trick.top_set.unwrap().player;
        let top_set_player = if current_leader_number == ui_player_number {
            ui::table::Player::PerspectivePlayer
        } else {
            ui::table::Player::Other(player_names[current_leader_number].clone())
        };
        Some(ui::table::TopSet::new(
            trick_history[0].player_move,
            top_set_player,
        ))
    } else {
        None
    };
    let table = ui::table::Table::new(current_player, top_set);

    let player_hand = game_state.player_hands[ui_player_number];
    let mut available_moves = ai::game::get_available_moves(player_hand, game_state.trick.top_set);
    available_moves.reverse();
    let mut converted_moves = Vec::new();
    for player_move in available_moves {
        match player_move {
            ai::game::Move::Play(play) => {
                converted_moves.push(ui::cards::Move::new(
                    play.rank as u8,
                    play.num_wilds as u8,
                    play.num_non_wilds as u8,
                ));
            }
            ai::game::Move::Pass => {}
        }
    }
    let player_hand = if game_state.current_player_number == ui_player_number {
        ui::hand::PlayerHand::CurrentTurn(ui::hand::PlayerTurnHand::new(
            game_state.trick.top_set.is_some(),
            player_hand,
            ui::hand::SuggestedMove::Disabled,
            converted_moves,
            current_selected_move,
        ))
    } else {
        ui::hand::PlayerHand::NotPlayerTurn(player_hand)
    };

    let players = ui::players::Players::new(players);
    let trick_history = ui::cards::TrickHistory::new(trick_history.to_vec());
    let game = ui::game::GameState::new(players, table, trick_history, player_hand);
    game
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = ratatui::Terminal::new(ratatui::prelude::CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    let num_players = 4;
    let mut current_game_state =
        ai::game::generate_random_initial_game_state(num_players, consts::DEFAULT_DALMUTI_DECK);
    let player_names = vec![
        "Tanner".to_string(),
        "Tiffany".to_string(),
        "Kieran".to_string(),
        "Dallin".to_string(),
    ];
    let ui_player_number = 0;
    let mut current_selected_move = None;
    let mut trick_history = vec![];
    let mut current_ui_state = internal_state_to_ui_state(
        num_players,
        ui_player_number,
        current_selected_move,
        current_game_state.clone(),
        &trick_history,
        &player_names,
    );
    while !should_quit {
        terminal.draw(|frame| {
            frame.render_widget(current_ui_state.clone(), frame.size());
        })?;
        match handle_events(
            ui_player_number,
            &player_names,
            &mut current_selected_move,
            &mut current_game_state,
            &current_ui_state,
            &mut trick_history,
        )? {
            EventResult::Quit => {
                should_quit = true;
            }
            EventResult::Redraw => {
                current_ui_state = internal_state_to_ui_state(
                    num_players,
                    ui_player_number,
                    current_selected_move,
                    current_game_state.clone(),
                    &trick_history,
                    &player_names,
                );
            }
            EventResult::NoChange => {}
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

pub enum EventResult {
    Quit,
    Redraw,
    NoChange,
}

fn handle_events(
    ui_player_number: usize,
    player_names: &Vec<String>,
    current_selected_move: &mut Option<usize>,
    game_state: &mut ai::game::FullInformationGameState,
    ui_state: &ui::game::GameState,
    trick_history: &mut Vec<ui::cards::TrickHistoryEntry>,
) -> io::Result<EventResult> {
    let mut requires_redraw = false;
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            // Check if we should quit
            if key.modifiers == KeyModifiers::CONTROL
                && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('q'))
            {
                return Ok(EventResult::Quit);
            }

            // Now, check if we should update based on player input
            if game_state.current_player_number == ui_player_number {
                // Controls are active
                let can_pass = !game_state.trick.top_set.is_none();
                let can_move_right = match &ui_state.player_hand {
                    ui::hand::PlayerHand::CurrentTurn(player_turn_hand) => {
                        match current_selected_move {
                            Some(selected_move) => {
                                *selected_move < player_turn_hand.available_moves.len() - 1
                            }
                            None => true,
                        }
                    }
                    _ => false,
                };
                let can_move_left = match &ui_state.player_hand {
                    ui::hand::PlayerHand::CurrentTurn(_) => match current_selected_move {
                        Some(selected_move) => *selected_move > 0,
                        None => false,
                    },
                    _ => false,
                };
                let can_confirm_move = current_selected_move.is_some();

                if key.code == KeyCode::Right && can_move_right {
                    match current_selected_move {
                        Some(selected_move) => {
                            *selected_move += 1;
                        }
                        None => {
                            *current_selected_move = Some(0);
                        }
                    }
                    requires_redraw = true;
                } else if key.code == KeyCode::Left && can_move_left {
                    match current_selected_move {
                        Some(selected_move) => {
                            *selected_move -= 1;
                        }
                        None => {}
                    }
                    requires_redraw = true;
                } else if key.code == KeyCode::Tab && can_pass {
                    update_state_and_trick_history_with_player_move(
                        game_state,
                        player_names,
                        trick_history,
                        None,
                    );
                    *current_selected_move = None;
                    requires_redraw = true;
                } else if key.code == KeyCode::Enter && can_confirm_move {
                    update_state_and_trick_history_with_player_move(
                        game_state,
                        player_names,
                        trick_history,
                        match &ui_state.player_hand {
                            ui::hand::PlayerHand::CurrentTurn(player_turn_hand) => Some(
                                player_turn_hand.available_moves[current_selected_move.unwrap()],
                            ),
                            _ => None,
                        },
                    );
                    *current_selected_move = None;
                    requires_redraw = true;
                }
            }
            if requires_redraw {
                return Ok(EventResult::Redraw);
            } else {
                return Ok(EventResult::NoChange);
            }
        }
    }

    // Check if we need to make an AI move
    if game_state.current_player_number != ui_player_number {
        update_state_and_trick_history_with_ai_move(
            game_state,
            player_names,
            trick_history,
            &ai::monte_carlo::BasicHeuristic {},
        );
        requires_redraw = true;
    }

    if requires_redraw {
        Ok(EventResult::Redraw)
    } else {
        Ok(EventResult::NoChange)
    }
}
