use std::io;

use crate::main_menu_widgets::MainMenu;
use crate::settings_widgets::SettingsPage;
use crate::{
    cards,
    settings::{
        adjust_deck_count, adjust_number_of_players, set_deck_count_from_text,
        set_number_of_players_from_text, GameMode, GameSettings, PlayerPanelSelection,
        SettingsField, SettingsFocus, SettingsFormState,
    },
    solver_worker::{SolverClient, SolverPurpose, SolverResponse},
    view_model,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use monte_cardo_core::{eval, game as core_game};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget, Wrap},
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    Settings,
    LiveHandInput,
    Playing,
    GameOver,
}

#[derive(Debug, Clone)]
pub struct GameSession {
    pub mode: GameMode,
    pub game_state: core_game::FullInformationGameState,
    pub trick_history: Vec<cards::TrickHistoryEntry>,
    pub player_names: Vec<String>,
    pub ui_player_number: usize,
    pub current_selected_move: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct PendingSolverRequest {
    request_id: u64,
    purpose: SolverPurpose,
}

pub struct App {
    screen: Screen,
    main_menu_index: usize,
    settings: GameSettings,
    settings_form: SettingsFormState,
    session: Option<GameSession>,

    solver_client: SolverClient,
    pending_solver_request: Option<PendingSolverRequest>,
    current_action_values: Option<Vec<(core_game::Move, f32)>>,
    solver_error: Option<String>,

    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::MainMenu,
            main_menu_index: 0,
            settings: GameSettings::default(),
            settings_form: SettingsFormState::new(),
            session: None,

            solver_client: SolverClient::new(),
            pending_solver_request: None,
            current_action_values: None,
            solver_error: None,

            should_quit: false,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.size();

        match self.screen {
            Screen::MainMenu => self.render_main_menu(frame, area),
            Screen::Settings => self.render_settings(frame, area),
            Screen::LiveHandInput => self.render_live_hand_input(frame, area),
            Screen::Playing => self.render_playing(frame, area),
            Screen::GameOver => self.render_game_over(frame, area),
        }
    }

    pub fn tick(&mut self) {
        for response in self.solver_client.drain_responses() {
            self.handle_solver_response(response);
        }

        self.ensure_solver_request();
    }

    pub fn handle_event(&mut self, event: Event) -> io::Result<()> {
        let Event::Key(key) = event else {
            return Ok(());
        };

        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('q') {
            match self.screen {
                Screen::Playing | Screen::Settings | Screen::LiveHandInput | Screen::GameOver => {
                    self.return_to_main_menu();
                }
                Screen::MainMenu => self.should_quit = true,
            }
            return Ok(());
        }

        match self.screen {
            Screen::MainMenu => self.handle_main_menu_key(key),
            Screen::Settings => self.handle_settings_key(key),
            Screen::LiveHandInput => self.handle_live_hand_input_key(key),
            Screen::Playing => self.handle_playing_key(key),
            Screen::GameOver => self.handle_game_over_key(key),
        }

        Ok(())
    }

    fn render_main_menu(&self, frame: &mut Frame, area: Rect) {
        let panel = centered_rect(area, 76, 20);

        frame.render_widget(MainMenu::new(self.main_menu_index), panel);
    }

    fn render_settings(&self, frame: &mut Frame, area: Rect) {
        let panel = centered_rect(area, 100, area.height.saturating_sub(2).min(40));

        frame.render_widget(
            SettingsPage::new(&self.settings, &self.settings_form),
            panel,
        );
    }

    fn render_live_hand_input(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(Span::styled(
                "Live Hand Input",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("This screen is a placeholder for now."),
            Line::from("Eventually this will let the player input their dealt cards."),
            Line::from(""),
            Line::from("Enter : Start temporary random game"),
            Line::from("Esc : Settings"),
        ];

        let widget = Paragraph::new(lines)
            .block(
                Block::new()
                    .title("Play Live")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(widget, centered_rect(area, 72, 14));
    }

    fn render_playing(&self, frame: &mut Frame, area: Rect) {
        let Some(session) = &self.session else {
            self.render_missing_session(frame, area);
            return;
        };

        let ai_suggestion = self
            .current_action_values
            .as_ref()
            .and_then(|values| values.first())
            .map(|(player_move, _value)| *player_move);

        let widget = view_model::build_game_widget(
            session.game_state.number_of_players,
            session.ui_player_number,
            session.current_selected_move,
            ai_suggestion,
            &session.game_state,
            &session.trick_history,
            &session.player_names,
        );

        frame.render_widget(widget, area);

        if let Some(message) = self.solver_status_message() {
            let overlay = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1);
            Clear.render(overlay, frame.buffer_mut());

            let paragraph =
                Paragraph::new(Span::styled(message, Color::Yellow)).alignment(Alignment::Center);

            frame.render_widget(paragraph, overlay);
        }
    }

    fn render_game_over(&self, frame: &mut Frame, area: Rect) {
        let Some(session) = &self.session else {
            self.render_missing_session(frame, area);
            return;
        };

        let mut ordered_players: Vec<_> =
            core_game::PlayerID::all_player_ids(session.game_state.number_of_players).collect();

        ordered_players.sort_by_key(|&player_id| {
            let placement = session.game_state.player_placements[player_id];
            if placement == 0 {
                session.game_state.number_of_players
            } else {
                placement
            }
        });

        let mut lines = vec![
            Line::from(Span::styled(
                "Game Over",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (index, player_id) in ordered_players.into_iter().enumerate() {
            lines.push(Line::from(format!(
                "{}. {}",
                index + 1,
                session.player_names[player_id.get()]
            )));
        }

        lines.extend([
            Line::from(""),
            Line::from("Enter : Play again with same settings"),
            Line::from("Esc / Ctrl+Q : Main Menu"),
        ]);

        let widget = Paragraph::new(lines)
            .block(
                Block::new()
                    .title("Results")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick),
            )
            .alignment(Alignment::Center);

        frame.render_widget(widget, centered_rect(area, 60, 16));
    }

    fn render_missing_session(&self, frame: &mut Frame, area: Rect) {
        let widget = Paragraph::new("No active game session.")
            .block(Block::new().borders(Borders::ALL))
            .alignment(Alignment::Center);

        frame.render_widget(widget, centered_rect(area, 48, 7));
    }

    fn handle_main_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if self.main_menu_index > 0 {
                    self.main_menu_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.main_menu_index < 1 {
                    self.main_menu_index += 1;
                }
            }
            KeyCode::Enter => {
                self.settings.mode = if self.main_menu_index == 0 {
                    GameMode::PlayComputers
                } else {
                    GameMode::PlayLive
                };

                self.settings_form.clamp_to_settings(&self.settings);
                self.screen = Screen::Settings;
            }
            _ => {}
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        match self.settings_form.focus {
            SettingsFocus::Mode => self.handle_mode_settings_key(key),
            SettingsFocus::Deck => self.handle_deck_settings_key(key),
            SettingsFocus::Players => self.handle_player_settings_key(key),
            SettingsFocus::Rules => self.handle_rules_settings_key(key),
            SettingsFocus::Start => self.handle_start_settings_key(key),
        }
    }

    fn handle_mode_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.screen = Screen::MainMenu,
            KeyCode::Left => self.settings_form.move_mode_cursor(-1),
            KeyCode::Right => self.settings_form.move_mode_cursor(1),
            KeyCode::Enter => {
                self.settings.mode = self.settings_form.mode_cursor;
            }
            KeyCode::Down => self.settings_form.focus_deck_start(),
            KeyCode::Up => self.settings_form.focus_start(),
            _ => {}
        }
    }

    fn handle_deck_settings_key(&mut self, key: KeyEvent) {
        if self.settings_form.deck_editing {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.settings_form.finish_deck_editing();
                }
                KeyCode::Up => {
                    adjust_deck_count(&mut self.settings, self.settings_form.deck_rank, 1);
                    self.settings_form.deck_edit_buffer.clear();
                }
                KeyCode::Down => {
                    adjust_deck_count(&mut self.settings, self.settings_form.deck_rank, -1);
                    self.settings_form.deck_edit_buffer.clear();
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    self.settings_form.deck_edit_buffer.push(c);
                    set_deck_count_from_text(
                        &mut self.settings,
                        self.settings_form.deck_rank,
                        &self.settings_form.deck_edit_buffer,
                    );
                }
                KeyCode::Backspace => {
                    self.settings_form.deck_edit_buffer.pop();
                    set_deck_count_from_text(
                        &mut self.settings,
                        self.settings_form.deck_rank,
                        &self.settings_form.deck_edit_buffer,
                    );
                }
                _ => {}
            }

            return;
        }

        match key.code {
            KeyCode::Esc => self.screen = Screen::MainMenu,
            KeyCode::Left => self.settings_form.move_deck_rank(-1),
            KeyCode::Right => self.settings_form.move_deck_rank(1),
            KeyCode::Enter => self.settings_form.start_deck_editing(),
            KeyCode::Up => self.settings_form.focus_mode(),
            KeyCode::Down => self.settings_form.focus_players(),
            _ => {}
        }
    }

    fn handle_player_settings_key(&mut self, key: KeyEvent) {
        if self.settings_form.player_name_editing {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.settings_form.finish_player_editing();
                }
                KeyCode::Char(c) if !c.is_control() => {
                    if let PlayerPanelSelection::PlayerName(index) =
                        self.settings_form.player_selection
                    {
                        self.settings.ensure_player_names();

                        if let Some(name) = self.settings.player_names.get_mut(index) {
                            name.push(c);
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let PlayerPanelSelection::PlayerName(index) =
                        self.settings_form.player_selection
                    {
                        if let Some(name) = self.settings.player_names.get_mut(index) {
                            name.pop();
                        }
                    }
                }
                _ => {}
            }

            return;
        }

        if self.settings_form.player_count_editing {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.settings_form.finish_player_editing();
                }
                KeyCode::Up | KeyCode::Right => {
                    adjust_number_of_players(&mut self.settings, 1);
                    self.settings_form.player_count_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                KeyCode::Down | KeyCode::Left => {
                    adjust_number_of_players(&mut self.settings, -1);
                    self.settings_form.player_count_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    self.settings_form.player_count_edit_buffer.push(c);
                    set_number_of_players_from_text(
                        &mut self.settings,
                        &self.settings_form.player_count_edit_buffer,
                    );
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                KeyCode::Backspace => {
                    self.settings_form.player_count_edit_buffer.pop();
                    set_number_of_players_from_text(
                        &mut self.settings,
                        &self.settings_form.player_count_edit_buffer,
                    );
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                _ => {}
            }

            return;
        }

        match key.code {
            KeyCode::Esc => self.screen = Screen::MainMenu,
            KeyCode::Left | KeyCode::Right => self.settings_form.focus_rules(),
            KeyCode::Enter => self.settings_form.start_player_editing(),
            KeyCode::Up => {
                if !self.settings_form.move_player_selection_up() {
                    self.settings_form.focus_deck_start();
                }
            }
            KeyCode::Down => {
                if !self
                    .settings_form
                    .move_player_selection_down(&self.settings)
                {
                    self.settings_form.focus_start();
                }
            }
            _ => {}
        }
    }

    fn handle_rules_settings_key(&mut self, key: KeyEvent) {
        if self.settings_form.rules_editing {
            let Some(field) = self.settings_form.selected_rule_field(&self.settings) else {
                self.settings_form.finish_rule_editing();
                return;
            };

            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.settings_form.finish_rule_editing();
                }
                KeyCode::Up | KeyCode::Right => {
                    field.adjust(&mut self.settings, 1);
                    self.settings_form.rules_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                KeyCode::Down | KeyCode::Left => {
                    field.adjust(&mut self.settings, -1);
                    self.settings_form.rules_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                KeyCode::Char(c)
                    if c.is_ascii_digit() || (field.allows_decimal_text() && c == '.') =>
                {
                    self.settings_form.rules_edit_buffer.push(c);
                    field.set_from_text(&mut self.settings, &self.settings_form.rules_edit_buffer);
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                KeyCode::Backspace => {
                    self.settings_form.rules_edit_buffer.pop();
                    field.set_from_text(&mut self.settings, &self.settings_form.rules_edit_buffer);
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                _ => {}
            }

            return;
        }

        match key.code {
            KeyCode::Esc => self.screen = Screen::MainMenu,
            KeyCode::Left | KeyCode::Right => self.settings_form.focus_players(),
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.settings_form.start_rule_editing(&mut self.settings);
                self.settings_form.clamp_to_settings(&self.settings);
            }
            KeyCode::Up => {
                if !self.settings_form.move_rules_up() {
                    self.settings_form.focus_deck_start();
                }
            }
            KeyCode::Down => {
                if !self.settings_form.move_rules_down(&self.settings) {
                    self.settings_form.focus_start();
                }
            }
            _ => {}
        }
    }

    fn handle_start_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.screen = Screen::MainMenu,
            KeyCode::Enter => self.start_from_settings(),
            KeyCode::Up => self.settings_form.focus_players(),
            KeyCode::Down => self.settings_form.focus_mode(),
            KeyCode::Left | KeyCode::Right => {}
            _ => {}
        }
    }

    fn handle_live_hand_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Settings,
            KeyCode::Enter => self.start_new_game(),
            _ => {}
        }
    }

    fn handle_game_over_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => self.start_new_game(),
            KeyCode::Esc => self.return_to_main_menu(),
            _ => {}
        }
    }

    fn handle_playing_key(&mut self, key: KeyEvent) {
        let Some(session) = &self.session else {
            return;
        };

        let ui_player_id = core_game::PlayerID::new(session.ui_player_number);

        if session.game_state.current_player_number != ui_player_id {
            return;
        }

        match key.code {
            KeyCode::Right => self.select_next_move(),
            KeyCode::Left => self.select_previous_move(),
            KeyCode::Tab => {
                if self.can_pass() {
                    self.apply_core_move(core_game::Move::Pass);
                }
            }
            KeyCode::Enter => {
                if let Some(player_move) = self.current_selected_core_move() {
                    self.apply_core_move(player_move);
                }
            }
            _ => {}
        }
    }

    fn start_from_settings(&mut self) {
        match self.settings.mode {
            GameMode::PlayComputers => self.start_new_game(),
            GameMode::PlayLive => self.screen = Screen::LiveHandInput,
        }
    }

    fn start_new_game(&mut self) {
        let mut heuristic = eval::NaiveHeuristic::new();
        let config = self.settings.solver.to_search_config();
        let mut search_context = eval::SearchContext::with_seed(
            &mut heuristic,
            config,
            self.settings.solver.random_seed,
        );

        let game_state = core_game::generate_random_initial_game_state(
            self.settings.number_of_players,
            &self.settings.deck,
            &mut search_context.rng,
        );

        self.session = Some(GameSession {
            mode: self.settings.mode,
            game_state,
            trick_history: Vec::new(),
            player_names: self.settings.player_names.clone(),
            ui_player_number: 0,
            current_selected_move: None,
        });

        self.pending_solver_request = None;
        self.current_action_values = None;
        self.solver_error = None;
        self.screen = Screen::Playing;
    }

    fn return_to_main_menu(&mut self) {
        self.screen = Screen::MainMenu;
        self.session = None;
        self.pending_solver_request = None;
        self.current_action_values = None;
        self.solver_error = None;
    }

    fn ensure_solver_request(&mut self) {
        if self.screen != Screen::Playing {
            return;
        }

        if self.pending_solver_request.is_some() {
            return;
        }

        if !self.settings.solver.enabled || !self.settings.ai_suggestions_enabled {
            return;
        }

        let Some(session) = &self.session else {
            return;
        };

        let ui_player_id = core_game::PlayerID::new(session.ui_player_number);
        let current_player = session.game_state.current_player_number;

        let purpose = if session.mode == GameMode::PlayComputers && current_player != ui_player_id {
            SolverPurpose::AiMove
        } else if current_player == ui_player_id && self.current_action_values.is_none() {
            SolverPurpose::Suggestion
        } else {
            return;
        };

        let incomplete_information_state = core_game::create_incomplete_information_game_state(
            &session.game_state,
            current_player,
        );

        let config = self.settings.solver.to_search_config();
        let seed = self.settings.solver.random_seed;

        let request_id =
            self.solver_client
                .request_action_values(incomplete_information_state, config, seed);

        self.pending_solver_request = Some(PendingSolverRequest {
            request_id,
            purpose,
        });
    }

    fn handle_solver_response(&mut self, response: SolverResponse) {
        let SolverResponse::ActionValues { request_id, values } = response;

        let Some(pending) = self.pending_solver_request else {
            return;
        };

        if pending.request_id != request_id {
            return;
        }

        self.pending_solver_request = None;

        match values {
            Ok(action_values) => match pending.purpose {
                SolverPurpose::Suggestion => {
                    self.current_action_values = Some(action_values);
                    self.solver_error = None;
                }
                SolverPurpose::AiMove => {
                    let Some((player_move, _value)) = action_values.into_iter().next() else {
                        self.solver_error = Some("Solver returned no AI move".to_string());
                        return;
                    };

                    self.apply_core_move(player_move);
                }
            },
            Err(error) => {
                self.current_action_values = None;
                self.solver_error = Some(error);
            }
        }
    }

    fn solver_status_message(&self) -> Option<String> {
        if let Some(error) = &self.solver_error {
            return Some(format!("Solver error: {}", error));
        }

        let Some(pending) = self.pending_solver_request else {
            return None;
        };

        match pending.purpose {
            SolverPurpose::Suggestion => Some("Solver thinking...".to_string()),
            SolverPurpose::AiMove => Some("AI thinking...".to_string()),
        }
    }

    fn available_play_moves(&self) -> Vec<core_game::Move> {
        let Some(session) = &self.session else {
            return Vec::new();
        };

        let ui_player_id = core_game::PlayerID::new(session.ui_player_number);
        let player_hand = &session.game_state.player_hands[ui_player_id];

        let mut available_moves =
            core_game::get_available_moves(player_hand, &session.game_state.trick.top_set);
        available_moves.reverse();

        available_moves
            .into_iter()
            .filter(|player_move| matches!(player_move, core_game::Move::Play(_)))
            .collect()
    }

    fn select_next_move(&mut self) {
        let move_count = self.available_play_moves().len();

        if move_count == 0 {
            return;
        }

        let Some(session) = &mut self.session else {
            return;
        };

        match session.current_selected_move {
            Some(index) => {
                if index + 1 < move_count {
                    session.current_selected_move = Some(index + 1);
                }
            }
            None => session.current_selected_move = Some(0),
        }
    }

    fn select_previous_move(&mut self) {
        let Some(session) = &mut self.session else {
            return;
        };

        match session.current_selected_move {
            Some(index) if index > 0 => {
                session.current_selected_move = Some(index - 1);
            }
            _ => {}
        }
    }

    fn current_selected_core_move(&self) -> Option<core_game::Move> {
        let session = self.session.as_ref()?;
        let selected_index = session.current_selected_move?;
        self.available_play_moves().into_iter().nth(selected_index)
    }

    fn can_pass(&self) -> bool {
        let Some(session) = &self.session else {
            return false;
        };

        session.game_state.trick.top_set.is_some()
    }

    fn apply_core_move(&mut self, player_move: core_game::Move) {
        let Some(session) = &mut self.session else {
            return;
        };

        let current_player = session.game_state.current_player_number;
        let current_player_name = session.player_names[current_player.get()].clone();

        let history_move = player_move;

        let round_finished = match core_game::update_full_information_game_state(
            &mut session.game_state,
            player_move,
        ) {
            Ok(round_finished) => round_finished,
            Err(error) => {
                self.solver_error = Some(format!("{:?}", error));
                return;
            }
        };

        match history_move {
            core_game::Move::Play(play) => {
                let player_move = cards::Move::new(
                    play.rank.get() as u8,
                    play.num_wilds.get() as u8,
                    play.num_non_wilds.get() as u8,
                );

                session.trick_history.insert(
                    0,
                    cards::TrickHistoryEntry::new(current_player_name, player_move),
                );
            }
            core_game::Move::Pass => {
                if session.game_state.trick.top_set.is_none() {
                    session.trick_history.clear();
                }
            }
        }

        session.current_selected_move = None;

        self.current_action_values = None;
        self.pending_solver_request = None;
        self.solver_error = None;

        if round_finished {
            self.screen = Screen::GameOver;
        }
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}
