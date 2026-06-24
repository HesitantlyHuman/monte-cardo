mod cards;
mod game;
mod hand;
mod players;
mod table;

use std::{
    io::{self, stdout},
    vec,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

fn update_state_and_trick_history_with_ai_move<H: monte_cardo_core::eval::ActionPriorHeuristic>(
    game_state: &mut monte_cardo_core::game::FullInformationGameState,
    player_names: &Vec<String>,
    trick_history: &mut Vec<cards::TrickHistoryEntry>,
    search_context: &mut monte_cardo_core::eval::SearchContext<H>,
) {
    let current_player = game_state.current_player_number;

    let incomplete_information_state =
        monte_cardo_core::game::create_incomplete_information_game_state(
            game_state,
            current_player,
        );
    let game_move =
        monte_cardo_core::eval::choose_best_action(&incomplete_information_state, search_context)
            .unwrap();

    monte_cardo_core::game::update_full_information_game_state(game_state, game_move).unwrap();
    match game_move {
        monte_cardo_core::game::Move::Play(play) => {
            let player_name = player_names[current_player.get()].clone();
            let player_move = crate::cards::Move::new(
                play.rank.get() as u8,
                play.num_wilds.get() as u8,
                play.num_non_wilds.get() as u8,
            );
            // Add the move to the trick history
            let trick_history_entry =
                crate::cards::TrickHistoryEntry::new(player_name, player_move);
            trick_history.insert(0, trick_history_entry);
        }
        monte_cardo_core::game::Move::Pass => {
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
    game_state: &mut monte_cardo_core::game::FullInformationGameState,
    player_names: &Vec<String>,
    trick_history: &mut Vec<crate::cards::TrickHistoryEntry>,
    move_: Option<crate::cards::Move>,
) {
    let game_move = match move_ {
        Some(m) => monte_cardo_core::game::Move::Play(monte_cardo_core::game::Play::new(
            monte_cardo_core::game::CardRank::new(m.rank as usize),
            monte_cardo_core::game::CardCount::new(m.num_wilds.into()),
            monte_cardo_core::game::CardCount::new(m.num_non_wilds.into()),
        )),
        None => monte_cardo_core::game::Move::Pass,
    };
    let current_player = game_state.current_player_number;
    monte_cardo_core::game::update_full_information_game_state(game_state, game_move).unwrap();
    match game_move {
        monte_cardo_core::game::Move::Play(play) => {
            let player_name = player_names[current_player.get()].clone();
            let player_move = crate::cards::Move::new(
                play.rank.get() as u8,
                play.num_wilds.get() as u8,
                play.num_non_wilds.get() as u8,
            );
            // Add to the front of the trick history
            let history_entry = crate::cards::TrickHistoryEntry::new(player_name, player_move);
            trick_history.insert(0, history_entry);
        }
        monte_cardo_core::game::Move::Pass => {
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
    ai_suggestion: Option<monte_cardo_core::game::Move>,
    game_state: monte_cardo_core::game::FullInformationGameState,
    trick_history: &Vec<crate::cards::TrickHistoryEntry>,
    player_names: &Vec<String>,
) -> crate::game::GameState {
    let mut players = vec![];
    for player_id in monte_cardo_core::game::PlayerID::all_player_ids(num_players) {
        let player_hand = &game_state.player_hands[player_id];
        let player_name = player_names[player_id.get()].clone();

        let mut hand_size = 0;
        for card_count in player_hand.iter() {
            hand_size += card_count.get() as u16;
        }

        let player_state = if player_id == game_state.current_player_number {
            crate::players::PlayerState::Active
        } else if hand_size == 0 {
            match game_state.trick.top_set {
                Some(top_set) => {
                    if top_set.player == player_id {
                        crate::players::PlayerState::LeadingOut
                    } else {
                        crate::players::PlayerState::NormalOut
                    }
                }
                None => crate::players::PlayerState::NormalOut,
            }
        } else if game_state.trick.has_passed[player_id] {
            crate::players::PlayerState::Passed
        } else if game_state.trick.top_set.is_some()
            && game_state.trick.top_set.unwrap().player == player_id
        {
            crate::players::PlayerState::Leading
        } else {
            crate::players::PlayerState::Normal
        };

        players.push(crate::players::Player::new(
            player_name,
            player_state,
            hand_size,
        ));
    }
    let current_player = if monte_cardo_core::game::PlayerID::new(ui_player_number)
        == game_state.current_player_number
    {
        crate::table::Player::PerspectivePlayer
    } else {
        crate::table::Player::Other(player_names[game_state.current_player_number.get()].clone())
    };
    let top_set = if trick_history.len() > 0 {
        let current_leader_number = game_state.trick.top_set.unwrap().player;
        let top_set_player =
            if current_leader_number == monte_cardo_core::game::PlayerID::new(ui_player_number) {
                crate::table::Player::PerspectivePlayer
            } else {
                crate::table::Player::Other(player_names[current_leader_number.get()].clone())
            };
        Some(crate::table::TopSet::new(
            trick_history[0].player_move,
            top_set_player,
        ))
    } else {
        None
    };
    let table = crate::table::Table::new(current_player, top_set);

    let player_hand =
        &game_state.player_hands[monte_cardo_core::game::PlayerID::new(ui_player_number)];

    let mut available_moves =
        monte_cardo_core::game::get_available_moves(&player_hand, &game_state.trick.top_set);
    available_moves.reverse();
    let mut converted_moves = Vec::new();
    for player_move in available_moves {
        match player_move {
            monte_cardo_core::game::Move::Play(play) => {
                converted_moves.push(crate::cards::Move::new(
                    play.rank.get() as u8,
                    play.num_wilds.get() as u8,
                    play.num_non_wilds.get() as u8,
                ));
            }
            monte_cardo_core::game::Move::Pass => {}
        }
    }
    let suggested_move = match ai_suggestion {
        Some(ai_move) => match ai_move {
            monte_cardo_core::game::Move::Play(play) => crate::hand::SuggestedMove::Suggestion(
                crate::hand::MoveSuggestion::Move(crate::cards::Move::new(
                    play.rank.get() as u8,
                    play.num_wilds.get() as u8,
                    play.num_non_wilds.get() as u8,
                )),
            ),
            monte_cardo_core::game::Move::Pass => {
                crate::hand::SuggestedMove::Suggestion(crate::hand::MoveSuggestion::Pass)
            }
        },
        None => crate::hand::SuggestedMove::Disabled,
    };
    let player_hand = if game_state.current_player_number
        == monte_cardo_core::game::PlayerID::new(ui_player_number)
    {
        crate::hand::PlayerHand::CurrentTurn(crate::hand::PlayerTurnHand::new(
            game_state.trick.top_set.is_some(),
            player_hand.to_usize_counts().map(|x| x as u8),
            suggested_move,
            converted_moves,
            current_selected_move,
        ))
    } else {
        crate::hand::PlayerHand::NotPlayerTurn(player_hand.to_usize_counts().map(|x| x as u8))
    };

    let players = crate::players::Players::new(players);
    let trick_history = crate::cards::TrickHistory::new(trick_history.to_vec());
    let game = crate::game::GameState::new(players, table, trick_history, player_hand);
    game
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = ratatui::Terminal::new(ratatui::prelude::CrosstermBackend::new(stdout()))?;

    let mut heuristic = monte_cardo_core::eval::NaiveHeuristic::new();
    let search_config = monte_cardo_core::eval::SearchConfig::inference();
    let mut search_context =
        monte_cardo_core::eval::SearchContext::with_seed(&mut heuristic, search_config, 42);

    let mut should_quit = false;
    let num_players = 4;
    let mut current_game_state = monte_cardo_core::game::generate_random_initial_game_state(
        num_players,
        &monte_cardo_core::consts::DEFAULT_DALMUTI_DECK,
        &mut search_context.rng,
    );
    let player_names = vec![
        "Tanner".to_string(),
        "Tiffany".to_string(),
        "Kieran".to_string(),
        "Dallin".to_string(),
    ];
    let ui_player_number = 0;

    let incomplete_information_state =
        monte_cardo_core::game::create_incomplete_information_game_state(
            &current_game_state,
            current_game_state.current_player_number,
        );
    let mut current_ai_suggestion = Some(
        monte_cardo_core::eval::choose_best_action(
            &incomplete_information_state,
            &mut search_context,
        )
        .unwrap(),
    );

    let mut current_selected_move = None;
    let mut trick_history = vec![];
    let mut current_ui_state = internal_state_to_ui_state(
        num_players,
        ui_player_number,
        current_selected_move,
        current_ai_suggestion,
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
            &mut search_context,
        )? {
            EventResult::Quit => {
                should_quit = true;
            }
            EventResult::Redraw => {
                current_ui_state = internal_state_to_ui_state(
                    num_players,
                    ui_player_number,
                    current_selected_move,
                    current_ai_suggestion,
                    current_game_state.clone(),
                    &trick_history,
                    &player_names,
                );
            }
            EventResult::NoChange => {}
        }
        if current_game_state.current_player_number
            != monte_cardo_core::game::PlayerID::new(ui_player_number)
        {
            current_ai_suggestion = None;
        } else if current_game_state.current_player_number
            == monte_cardo_core::game::PlayerID::new(ui_player_number)
            && current_ai_suggestion.is_none()
        {
            let incomplete_information_state =
                monte_cardo_core::game::create_incomplete_information_game_state(
                    &current_game_state,
                    monte_cardo_core::game::PlayerID::new(ui_player_number),
                );
            current_ai_suggestion = Some(
                monte_cardo_core::eval::choose_best_action(
                    &incomplete_information_state,
                    &mut search_context,
                )
                .unwrap(),
            );

            current_ui_state = internal_state_to_ui_state(
                num_players,
                ui_player_number,
                current_selected_move,
                current_ai_suggestion,
                current_game_state.clone(),
                &trick_history,
                &player_names,
            );
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

fn handle_events<H: monte_cardo_core::eval::ActionPriorHeuristic>(
    ui_player_number: usize,
    player_names: &Vec<String>,
    current_selected_move: &mut Option<usize>,
    game_state: &mut monte_cardo_core::game::FullInformationGameState,
    ui_state: &crate::game::GameState,
    trick_history: &mut Vec<crate::cards::TrickHistoryEntry>,
    search_context: &mut monte_cardo_core::eval::SearchContext<H>,
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
            if game_state.current_player_number
                == monte_cardo_core::game::PlayerID::new(ui_player_number)
            {
                // Controls are active
                let can_pass = !game_state.trick.top_set.is_none();
                let can_move_right = match &ui_state.player_hand {
                    crate::hand::PlayerHand::CurrentTurn(player_turn_hand) => {
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
                    crate::hand::PlayerHand::CurrentTurn(_) => match current_selected_move {
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
                            crate::hand::PlayerHand::CurrentTurn(player_turn_hand) => Some(
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
    if game_state.current_player_number != monte_cardo_core::game::PlayerID::new(ui_player_number) {
        update_state_and_trick_history_with_ai_move(
            game_state,
            player_names,
            trick_history,
            search_context,
        );
        requires_redraw = true;
    }

    if requires_redraw {
        Ok(EventResult::Redraw)
    } else {
        Ok(EventResult::NoChange)
    }
}
