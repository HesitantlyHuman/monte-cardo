use std::io;

use crate::live_widgets::{
    LivePlayerColumn, LiveSetupFocus, LiveSetupPage, LiveSetupState, ObservedAction,
    ObservedMoveFocus, ObservedMoveInputState, ObservedMovePanel,
};
use crate::main_menu_widgets::MainMenu;
use crate::rank_count_editor::fixed_max_counts;
use crate::settings_widgets::SettingsPage;
use crate::AppKey;
use crate::{
    cards,
    settings::{
        adjust_number_of_players, set_number_of_players_from_text, GameMode, GameSettings,
        PlayerPanelSelection, SettingsFocus, SettingsFormState,
    },
    solver_worker::{SolverClient, SolverPurpose, SolverResponse},
    view_model,
};
use crossterm::event::{Event, KeyModifiers};
use monte_cardo_core::{eval, game as core_game};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
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

#[derive(Debug, Clone, Copy)]
struct PendingSolverRequest {
    request_id: u64,
    purpose: SolverPurpose,
}

#[derive(Debug, Clone)]
pub enum SessionState {
    Full(core_game::FullInformationGameState),
    Live(core_game::IncompleteInformationGameState),
}

#[derive(Debug, Clone)]
pub struct GameSession {
    pub mode: GameMode,
    pub state: SessionState,
    pub trick_history: Vec<cards::TrickHistoryEntry>,
    pub player_names: Vec<String>,
    pub ui_player_number: usize,
    pub current_selected_move: Option<usize>,
}

impl GameSession {
    pub fn number_of_players(&self) -> usize {
        match &self.state {
            SessionState::Full(state) => state.number_of_players,
            SessionState::Live(state) => state.number_of_players,
        }
    }

    pub fn current_player_number(&self) -> core_game::PlayerID {
        match &self.state {
            SessionState::Full(state) => state.current_player_number,
            SessionState::Live(state) => state.current_player_number,
        }
    }

    pub fn placement_for(&self, player_id: core_game::PlayerID) -> usize {
        match &self.state {
            SessionState::Full(state) => state.player_placements[player_id],
            SessionState::Live(state) => state.player_placements[player_id],
        }
    }

    pub fn incomplete_state_for_solver(
        &self,
        perspective_player_number: core_game::PlayerID,
    ) -> core_game::IncompleteInformationGameState {
        match &self.state {
            SessionState::Full(game_state) => core_game::create_incomplete_information_game_state(
                game_state,
                perspective_player_number,
            ),

            SessionState::Live(game_state) => {
                debug_assert_eq!(
                    game_state.perspective_player_number, perspective_player_number,
                    "Live mode can only solve from the known player's perspective"
                );

                game_state.clone()
            }
        }
    }
}

pub struct App {
    screen: Screen,
    main_menu_index: usize,
    settings: GameSettings,
    settings_form: SettingsFormState,
    live_setup: LiveSetupState,
    observed_move: ObservedMoveInputState,
    session: Option<GameSession>,

    #[cfg(not(target_arch = "wasm32"))]
    solver_client: SolverClient,
    #[cfg(not(target_arch = "wasm32"))]
    pending_solver_request: Option<PendingSolverRequest>,
    #[cfg(not(target_arch = "wasm32"))]
    current_action_values: Option<Vec<(core_game::Move, f32)>>,
    #[cfg(not(target_arch = "wasm32"))]
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
            live_setup: LiveSetupState::new(),
            observed_move: ObservedMoveInputState::new(),
            session: None,

            #[cfg(not(target_arch = "wasm32"))]
            solver_client: SolverClient::new(),
            #[cfg(not(target_arch = "wasm32"))]
            pending_solver_request: None,
            #[cfg(not(target_arch = "wasm32"))]
            current_action_values: None,
            #[cfg(not(target_arch = "wasm32"))]
            solver_error: None,

            should_quit: false,
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        match self.screen {
            Screen::MainMenu => self.render_main_menu(frame, area),
            Screen::Settings => self.render_settings(frame, area),
            Screen::LiveHandInput => self.render_live_hand_input(frame, area),
            Screen::Playing => self.render_playing(frame, area),
            Screen::GameOver => self.render_game_over(frame, area),
        }
    }

    pub fn tick(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        for response in self.solver_client.drain_responses() {
            self.handle_solver_response(response);
        }

        #[cfg(not(target_arch = "wasm32"))]
        self.ensure_solver_request();
    }

    pub fn handle_event(&mut self, key: AppKey) -> io::Result<()> {
        if key == AppKey::ControlC {
            self.should_quit = true;
            return Ok(());
        }

        if key == AppKey::ControlQ {
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
        let panel = centered_rect(area, 96, area.height.saturating_sub(2).min(20));

        frame.render_widget(
            LiveSetupPage::new(
                &self.settings,
                &self.live_setup,
                self.solver_error.as_deref(),
            ),
            panel,
        );
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

        match &session.state {
            SessionState::Full(game_state) => {
                let widget = view_model::build_game_widget(
                    game_state.number_of_players,
                    session.ui_player_number,
                    session.current_selected_move,
                    ai_suggestion,
                    game_state,
                    &session.trick_history,
                    &session.player_names,
                );

                frame.render_widget(widget, area);
            }

            SessionState::Live(game_state) => {
                let widget = view_model::build_incomplete_game_widget(
                    session.ui_player_number,
                    session.current_selected_move,
                    ai_suggestion,
                    game_state,
                    &session.trick_history,
                    &session.player_names,
                );

                frame.render_widget(widget, area);
            }
        }

        if let Some(session) = &self.session {
            if let SessionState::Live(state) = &session.state {
                if state.current_player_number != state.perspective_player_number {
                    let player_name = session
                        .player_names
                        .get(state.current_player_number.get())
                        .map(String::as_str)
                        .unwrap_or("Opponent");

                    let panel = centered_rect(area, 58, 10);

                    frame.render_widget(
                        ObservedMovePanel::new(&self.observed_move, player_name),
                        panel,
                    );
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
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
            core_game::PlayerID::all_player_ids(session.number_of_players()).collect();

        ordered_players.sort_by_key(|&player_id| {
            let placement = session.placement_for(player_id);

            if placement == 0 {
                session.number_of_players()
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

    fn handle_main_menu_key(&mut self, key: AppKey) {
        match key {
            AppKey::Up => {
                if self.main_menu_index > 0 {
                    self.main_menu_index -= 1;
                }
            }
            AppKey::Down => {
                if self.main_menu_index < 1 {
                    self.main_menu_index += 1;
                }
            }
            AppKey::Enter => {
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

    fn handle_settings_key(&mut self, key: AppKey) {
        match self.settings_form.focus {
            SettingsFocus::Mode => self.handle_mode_settings_key(key),
            SettingsFocus::Deck => self.handle_deck_settings_key(key),
            SettingsFocus::Players => self.handle_player_settings_key(key),
            SettingsFocus::Rules => self.handle_rules_settings_key(key),
            SettingsFocus::Start => self.handle_start_settings_key(key),
        }
    }

    fn handle_mode_settings_key(&mut self, key: AppKey) {
        match key {
            AppKey::Esc => self.screen = Screen::MainMenu,
            AppKey::Left => self.settings_form.move_mode_cursor(-1),
            AppKey::Right => self.settings_form.move_mode_cursor(1),
            AppKey::Enter => {
                self.settings.mode = self.settings_form.mode_cursor;
            }
            AppKey::Down => self.settings_form.focus_deck_start(),
            AppKey::Up => self.settings_form.focus_start(),
            _ => {}
        }
    }

    fn handle_deck_settings_key(&mut self, key: AppKey) {
        let max_counts = fixed_max_counts(99);

        if self.settings_form.deck_editor.editing {
            match key {
                AppKey::Esc | AppKey::Enter => {
                    self.settings_form.deck_editor.finish_editing();
                }

                AppKey::Left => {
                    self.settings_form.deck_editor.finish_editing();
                    self.settings_form.deck_editor.move_rank(-1);
                }

                AppKey::Right => {
                    self.settings_form.deck_editor.finish_editing();
                    self.settings_form.deck_editor.move_rank(1);
                }

                AppKey::Up => {
                    self.settings_form.deck_editor.adjust_count(
                        &mut self.settings.deck,
                        &max_counts,
                        1,
                    );
                    self.settings_form.deck_editor.edit_buffer.clear();
                }

                AppKey::Down => {
                    self.settings_form.deck_editor.adjust_count(
                        &mut self.settings.deck,
                        &max_counts,
                        -1,
                    );
                    self.settings_form.deck_editor.edit_buffer.clear();
                }

                AppKey::Char(c) if c.is_ascii_digit() => {
                    self.settings_form.deck_editor.input_digit(
                        &mut self.settings.deck,
                        &max_counts,
                        c,
                    );
                }

                AppKey::Backspace => {
                    self.settings_form
                        .deck_editor
                        .backspace_digit(&mut self.settings.deck, &max_counts);
                }

                _ => {}
            }

            return;
        }

        match key {
            AppKey::Esc => self.screen = Screen::MainMenu,
            AppKey::Left => self.settings_form.deck_editor.move_rank(-1),
            AppKey::Right => self.settings_form.deck_editor.move_rank(1),
            AppKey::Enter => self.settings_form.deck_editor.start_editing(),
            AppKey::Up => self.settings_form.focus_mode(),
            AppKey::Down => self.settings_form.focus_players(),
            _ => {}
        }
    }

    fn handle_player_settings_key(&mut self, key: AppKey) {
        if self.settings_form.player_name_editing {
            match key {
                AppKey::Esc | AppKey::Enter => {
                    self.settings_form.finish_player_editing();
                }
                AppKey::Char(c) if !c.is_control() => {
                    if let PlayerPanelSelection::PlayerName(index) =
                        self.settings_form.player_selection
                    {
                        self.settings.ensure_player_names();

                        if let Some(name) = self.settings.player_names.get_mut(index) {
                            name.push(c);
                        }
                    }
                }
                AppKey::Backspace => {
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
            match key {
                AppKey::Esc | AppKey::Enter => {
                    self.settings_form.finish_player_editing();
                }
                AppKey::Up | AppKey::Right => {
                    adjust_number_of_players(&mut self.settings, 1);
                    self.settings_form.player_count_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                AppKey::Down | AppKey::Left => {
                    adjust_number_of_players(&mut self.settings, -1);
                    self.settings_form.player_count_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                AppKey::Char(c) if c.is_ascii_digit() => {
                    self.settings_form.player_count_edit_buffer.push(c);
                    set_number_of_players_from_text(
                        &mut self.settings,
                        &self.settings_form.player_count_edit_buffer,
                    );
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                AppKey::Backspace => {
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

        match key {
            AppKey::Esc => self.screen = Screen::MainMenu,
            AppKey::Left | AppKey::Right => self.settings_form.focus_rules(),
            AppKey::Enter => self.settings_form.start_player_editing(),
            AppKey::Up => {
                if !self.settings_form.move_player_selection_up() {
                    self.settings_form.focus_deck_start();
                }
            }
            AppKey::Down => {
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

    fn handle_rules_settings_key(&mut self, key: AppKey) {
        if self.settings_form.rules_editing {
            let Some(field) = self.settings_form.selected_rule_field(&self.settings) else {
                self.settings_form.finish_rule_editing();
                return;
            };

            match key {
                AppKey::Esc | AppKey::Enter => {
                    self.settings_form.finish_rule_editing();
                }
                AppKey::Up | AppKey::Right => {
                    field.adjust(&mut self.settings, 1);
                    self.settings_form.rules_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                AppKey::Down | AppKey::Left => {
                    field.adjust(&mut self.settings, -1);
                    self.settings_form.rules_edit_buffer.clear();
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                AppKey::Char(c)
                    if c.is_ascii_digit() || (field.allows_decimal_text() && c == '.') =>
                {
                    self.settings_form.rules_edit_buffer.push(c);
                    field.set_from_text(&mut self.settings, &self.settings_form.rules_edit_buffer);
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                AppKey::Backspace => {
                    self.settings_form.rules_edit_buffer.pop();
                    field.set_from_text(&mut self.settings, &self.settings_form.rules_edit_buffer);
                    self.settings_form.clamp_to_settings(&self.settings);
                }
                _ => {}
            }

            return;
        }

        match key {
            AppKey::Esc => self.screen = Screen::MainMenu,
            AppKey::Left | AppKey::Right => self.settings_form.focus_players(),
            AppKey::Enter | AppKey::Char(' ') => {
                self.settings_form.start_rule_editing(&mut self.settings);
                self.settings_form.clamp_to_settings(&self.settings);
            }
            AppKey::Up => {
                if !self.settings_form.move_rules_up() {
                    self.settings_form.focus_deck_start();
                }
            }
            AppKey::Down => {
                if !self.settings_form.move_rules_down(&self.settings) {
                    self.settings_form.focus_start();
                }
            }
            _ => {}
        }
    }

    fn handle_start_settings_key(&mut self, key: AppKey) {
        match key {
            AppKey::Esc => self.screen = Screen::MainMenu,
            AppKey::Enter => self.start_from_settings(),
            AppKey::Up => self.settings_form.focus_players(),
            AppKey::Down => self.settings_form.focus_mode(),
            AppKey::Left | AppKey::Right => {}
            _ => {}
        }
    }

    fn handle_live_setup_hand_key(&mut self, key: AppKey) {
        if self.live_setup.hand_editor.editing {
            match key {
                AppKey::Esc | AppKey::Enter => {
                    self.live_setup.finish_hand_editing();
                }

                AppKey::Left => {
                    self.live_setup.finish_hand_editing();
                    self.live_setup.move_hand_rank(-1);
                }

                AppKey::Right => {
                    self.live_setup.finish_hand_editing();
                    self.live_setup.move_hand_rank(1);
                }

                AppKey::Up => {
                    self.live_setup.adjust_hand_count(&self.settings, 1);
                    self.live_setup.hand_editor.edit_buffer.clear();
                }

                AppKey::Down => {
                    self.live_setup.adjust_hand_count(&self.settings, -1);
                    self.live_setup.hand_editor.edit_buffer.clear();
                }

                AppKey::Char(c) if c.is_ascii_digit() => {
                    self.live_setup.input_hand_digit(&self.settings, c);
                }

                AppKey::Backspace => {
                    self.live_setup.backspace_hand_digit(&self.settings);
                }

                _ => {}
            }

            return;
        }

        match key {
            AppKey::Esc => self.screen = Screen::Settings,
            AppKey::Left => self.live_setup.move_hand_rank(-1),
            AppKey::Right => self.live_setup.move_hand_rank(1),
            AppKey::Enter => self.live_setup.start_hand_editing(),
            AppKey::Down => self.live_setup.focus = LiveSetupFocus::Players,
            AppKey::Up => self.live_setup.focus = LiveSetupFocus::Start,
            _ => {}
        }
    }

    fn handle_live_setup_players_key(&mut self, key: AppKey) {
        if self.live_setup.hand_size_editing {
            match key {
                AppKey::Esc | AppKey::Enter => {
                    self.live_setup.finish_hand_size_editing();
                }

                AppKey::Up | AppKey::Right => {
                    self.live_setup.adjust_hand_size(&self.settings, 1);
                    self.live_setup.hand_size_edit_buffer.clear();
                }

                AppKey::Down | AppKey::Left => {
                    self.live_setup.adjust_hand_size(&self.settings, -1);
                    self.live_setup.hand_size_edit_buffer.clear();
                }

                AppKey::Char(c) if c.is_ascii_digit() => {
                    self.live_setup.input_hand_size_digit(&self.settings, c);
                }

                AppKey::Backspace => {
                    self.live_setup.backspace_hand_size_digit();
                }

                _ => {}
            }

            return;
        }

        match key {
            AppKey::Esc => self.screen = Screen::Settings,

            AppKey::Left => self.live_setup.move_player_column(-1),
            AppKey::Right => self.live_setup.move_player_column(1),

            AppKey::Up => {
                if self.live_setup.hand_size_player == 0 {
                    self.live_setup.focus = LiveSetupFocus::Hand;
                } else if self.live_setup.hand_size_player == 1
                    && self.live_setup.player_column == LivePlayerColumn::HandSize
                {
                    self.live_setup.focus = LiveSetupFocus::Hand;
                } else {
                    self.live_setup.move_hand_size_player(&self.settings, -1);
                }
            }

            AppKey::Down => {
                if self.live_setup.hand_size_player + 1 >= self.settings.number_of_players {
                    self.live_setup.focus = LiveSetupFocus::Start;
                } else {
                    self.live_setup.move_hand_size_player(&self.settings, 1);
                }
            }

            AppKey::Enter => match self.live_setup.player_column {
                LivePlayerColumn::StartingPlayer => {
                    self.live_setup.select_current_player_as_starting_player();
                }
                LivePlayerColumn::HandSize => {
                    self.live_setup.start_hand_size_editing();
                }
            },

            _ => {}
        }
    }

    fn start_live_game_from_setup(&mut self) {
        let player_hand =
            core_game::PlayerHand::new(self.live_setup.hand_counts.map(core_game::CardCount::new));

        let perspective_player = core_game::PlayerID::new(0);
        let starting_player = core_game::PlayerID::new(self.live_setup.starting_player);

        let live_state = match create_initial_live_incomplete_state(
            &self.settings,
            player_hand,
            self.live_setup.hand_sizes,
            perspective_player,
            starting_player,
        ) {
            Ok(state) => state,
            Err(error) => {
                self.solver_error = Some(error);
                return;
            }
        };

        self.session = Some(GameSession {
            mode: GameMode::PlayLive,
            state: SessionState::Live(live_state),
            trick_history: Vec::new(),
            player_names: self.settings.player_names.clone(),
            ui_player_number: 0,
            current_selected_move: None,
        });

        self.observed_move.reset_for_next_move();
        self.pending_solver_request = None;
        self.current_action_values = None;
        self.solver_error = None;
        self.screen = Screen::Playing;
    }

    fn handle_live_setup_start_key(&mut self, key: AppKey) {
        match key {
            AppKey::Esc => self.screen = Screen::Settings,
            AppKey::Enter => self.start_live_game_from_setup(),
            AppKey::Up => self.live_setup.focus = LiveSetupFocus::Players,
            AppKey::Down => self.live_setup.focus = LiveSetupFocus::Hand,
            _ => {}
        }
    }

    fn handle_live_hand_input_key(&mut self, key: AppKey) {
        match self.live_setup.focus {
            LiveSetupFocus::Hand => self.handle_live_setup_hand_key(key),
            LiveSetupFocus::Players => self.handle_live_setup_players_key(key),
            LiveSetupFocus::Start => self.handle_live_setup_start_key(key),
        }
    }

    fn handle_game_over_key(&mut self, key: AppKey) {
        match key {
            AppKey::Enter => self.start_new_game(),
            AppKey::Esc => self.return_to_main_menu(),
            _ => {}
        }
    }

    fn current_observed_move(&self) -> Option<core_game::Move> {
        match self.observed_move.action {
            ObservedAction::Pass => Some(core_game::Move::Pass),

            ObservedAction::Play => {
                if self.observed_move.non_wilds == 0 && self.observed_move.wilds == 0 {
                    return None;
                }

                Some(core_game::Move::Play(core_game::Play::new(
                    core_game::CardRank::new(self.observed_move.rank),
                    core_game::CardCount::new(self.observed_move.non_wilds),
                    core_game::CardCount::new(self.observed_move.wilds),
                )))
            }
        }
    }

    fn handle_observed_move_key(&mut self, key: AppKey) {
        if self.observed_move.editing_count {
            match key {
                AppKey::Esc | AppKey::Enter => self.observed_move.finish_count_editing(),

                AppKey::Up | AppKey::Right => {
                    self.observed_move.adjust_current_count(1);
                    self.observed_move.edit_buffer.clear();
                }

                AppKey::Down | AppKey::Left => {
                    self.observed_move.adjust_current_count(-1);
                    self.observed_move.edit_buffer.clear();
                }

                AppKey::Char(c) if c.is_ascii_digit() => {
                    self.observed_move.input_count_digit(c);
                }

                AppKey::Backspace => {
                    self.observed_move.backspace_count_digit();
                }

                _ => {}
            }

            return;
        }

        match key {
            AppKey::Up => self.observed_move.move_focus_up(),
            AppKey::Down => self.observed_move.move_focus_down(),
            AppKey::Left => self.observed_move.move_left_right(-1),
            AppKey::Right => self.observed_move.move_left_right(1),

            AppKey::Enter => match self.observed_move.focus {
                ObservedMoveFocus::Rank
                | ObservedMoveFocus::NonWilds
                | ObservedMoveFocus::Wilds => {
                    self.observed_move.start_count_editing();
                }

                ObservedMoveFocus::Submit => {
                    if let Some(player_move) = self.current_observed_move() {
                        self.apply_core_move(player_move);
                        self.observed_move.reset_for_next_move();
                    }
                }

                ObservedMoveFocus::Action => {}

                _ => {}
            },
            _ => {}
        }
    }

    fn handle_playing_key(&mut self, key: AppKey) {
        let Some(session) = &self.session else {
            return;
        };

        let ui_player_id = core_game::PlayerID::new(session.ui_player_number);

        if session.current_player_number() != ui_player_id {
            if session.mode == GameMode::PlayLive {
                self.handle_observed_move_key(key);
            }

            return;
        }

        match key {
            AppKey::Right => self.select_next_move(),

            AppKey::Left => self.select_previous_move(),

            AppKey::Tab => {
                if self.can_pass() {
                    self.apply_core_move(core_game::Move::Pass);
                }
            }

            AppKey::Enter => {
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
            GameMode::PlayLive => {
                self.live_setup.reset_from_settings(&self.settings);
                self.solver_error = None;
                self.screen = Screen::LiveHandInput;
            }
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
            state: SessionState::Full(game_state),
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

    #[cfg(not(target_arch = "wasm32"))]
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

        let current_player = session.current_player_number();
        let ui_player_id = core_game::PlayerID::new(session.ui_player_number);

        let purpose = match session.mode {
            GameMode::PlayComputers => {
                if current_player == ui_player_id {
                    if self.current_action_values.is_some() {
                        return;
                    }

                    SolverPurpose::Suggestion
                } else {
                    SolverPurpose::AiMove
                }
            }

            GameMode::PlayLive => {
                if current_player != ui_player_id || self.current_action_values.is_some() {
                    return;
                }

                SolverPurpose::Suggestion
            }
        };

        let perspective_player = match purpose {
            SolverPurpose::Suggestion => ui_player_id,
            SolverPurpose::AiMove => current_player,
        };

        let incomplete_information_state = session.incomplete_state_for_solver(perspective_player);

        let config = self.settings.solver.to_search_config();
        let heuristic = self.settings.solver.heuristic;
        let seed = self.settings.solver.random_seed;

        let request_id = self.solver_client.request_action_values(
            incomplete_information_state,
            config,
            heuristic,
            seed,
        );

        self.pending_solver_request = Some(PendingSolverRequest {
            request_id,
            purpose,
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(not(target_arch = "wasm32"))]
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

        let mut available_moves = match &session.state {
            SessionState::Full(state) => {
                let player_hand = &state.player_hands[ui_player_id];
                core_game::get_available_moves(player_hand, &state.trick.top_set)
            }

            SessionState::Live(state) => {
                if state.current_player_number != state.perspective_player_number {
                    return Vec::new();
                }

                core_game::get_available_moves(&state.player_hand, &state.trick.top_set)
            }
        };

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

        match &session.state {
            SessionState::Full(state) => state.trick.top_set.is_some(),

            SessionState::Live(state) => {
                state.current_player_number == state.perspective_player_number
                    && state.trick.top_set.is_some()
            }
        }
    }

    fn apply_core_move(&mut self, player_move: core_game::Move) {
        let Some(session) = &mut self.session else {
            return;
        };

        let current_player = session.current_player_number();
        let current_player_name = session.player_names[current_player.get()].clone();
        let history_move = player_move;

        let update_result = match &mut session.state {
            SessionState::Full(state) => {
                core_game::update_full_information_game_state(state, player_move)
                    .map_err(|error| format!("{:?}", error))
            }

            SessionState::Live(state) => {
                if let Err(error) = validate_incomplete_move_is_possible(state, player_move) {
                    Err(error)
                } else {
                    core_game::update_incomplete_information_game_state(state, player_move)
                        .map_err(|error| format!("{:?}", error))
                }
            }
        };

        let round_finished = match update_result {
            Ok(round_finished) => round_finished,
            Err(error) => {
                self.solver_error = Some(error);
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
                let trick_is_empty = match &session.state {
                    SessionState::Full(state) => state.trick.top_set.is_none(),
                    SessionState::Live(state) => state.trick.top_set.is_none(),
                };

                if trick_is_empty {
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

fn create_initial_live_incomplete_state(
    settings: &GameSettings,
    player_hand: core_game::PlayerHand,
    hand_sizes: [usize; monte_cardo_core::consts::MAX_PLAYERS],
    perspective_player_number: core_game::PlayerID,
    starting_player_number: core_game::PlayerID,
) -> Result<core_game::IncompleteInformationGameState, String> {
    let number_of_players = settings.number_of_players;

    if number_of_players < 2 || number_of_players > monte_cardo_core::consts::MAX_PLAYERS {
        return Err(format!("Invalid number of players: {}", number_of_players));
    }

    if perspective_player_number.get() >= number_of_players {
        return Err("Perspective player is outside the active player range.".to_string());
    }

    if starting_player_number.get() >= number_of_players {
        return Err("Starting player is outside the active player range.".to_string());
    }

    let total_deck_cards: usize = settings.deck.iter().sum();

    let mut player_hand_size = 0usize;
    let mut opponent_cards = core_game::PlayerHand::empty();

    for rank in core_game::CardRank::all() {
        let deck_count = settings.deck[rank.get()];
        let player_count = player_hand[rank].get();

        if player_count > deck_count {
            return Err(format!(
                "Player has {} cards of rank {}, but the deck only has {}.",
                player_count,
                rank.get(),
                deck_count,
            ));
        }

        player_hand_size += player_count;
        opponent_cards[rank] = core_game::CardCount::new(deck_count - player_count);
    }

    if hand_sizes[perspective_player_number.get()] != player_hand_size {
        return Err(format!(
            "Your explicit hand size is {}, but your entered hand contains {} cards.",
            hand_sizes[perspective_player_number.get()],
            player_hand_size,
        ));
    }

    let total_hand_sizes: usize = hand_sizes.iter().take(number_of_players).copied().sum();

    if total_hand_sizes != total_deck_cards {
        return Err(format!(
            "The active hand sizes sum to {}, but the deck contains {} cards.",
            total_hand_sizes, total_deck_cards,
        ));
    }

    let opponent_hand_sizes: usize = hand_sizes
        .iter()
        .take(number_of_players)
        .enumerate()
        .filter(|(player_index, _)| *player_index != perspective_player_number.get())
        .map(|(_, hand_size)| *hand_size)
        .sum();

    let opponent_card_count: usize = core_game::CardRank::all()
        .map(|rank| opponent_cards[rank].get())
        .sum();

    if opponent_hand_sizes != opponent_card_count {
        return Err(format!(
            "Opponent hand sizes sum to {}, but unknown opponent cards sum to {}.",
            opponent_hand_sizes, opponent_card_count,
        ));
    }

    let mut core_hand_sizes = core_game::HandSizes::empty();

    for player_id in core_game::PlayerID::all_player_ids(number_of_players) {
        core_hand_sizes.add_cards(
            player_id,
            core_game::CardCount::new(hand_sizes[player_id.get()]),
        );
    }

    let mut trick = core_game::Trick::new();

    let mut has_passed = [true; monte_cardo_core::consts::MAX_PLAYERS];
    for player_id in core_game::PlayerID::all_player_ids(number_of_players) {
        has_passed[player_id.get()] = false;
    }
    trick.has_passed = core_game::PlayerIndexed::new(has_passed);

    Ok(core_game::IncompleteInformationGameState {
        current_player_number: starting_player_number,
        perspective_player_number,
        number_of_players,
        player_hand,
        opponent_cards,
        player_placements: core_game::PlayerPlacements::new(),
        hand_sizes: core_hand_sizes,
        trick,
    })
}

fn validate_incomplete_move_is_possible(
    state: &core_game::IncompleteInformationGameState,
    player_move: core_game::Move,
) -> Result<(), String> {
    match player_move {
        core_game::Move::Pass => {
            if state.trick.top_set.is_none() {
                return Err("Cannot pass on an empty trick.".to_string());
            }

            Ok(())
        }

        core_game::Move::Play(play) => {
            let total_count = play.total_count().get();

            if total_count == 0 {
                return Err("Cannot play zero cards.".to_string());
            }

            let current_hand_size = state.hand_sizes[state.current_player_number];

            if total_count > current_hand_size {
                return Err(format!(
                    "Player {} only has {} cards, but tried to play {}.",
                    state.current_player_number.get() + 1,
                    current_hand_size,
                    total_count,
                ));
            }

            let available_hand = if state.current_player_number == state.perspective_player_number {
                &state.player_hand
            } else {
                &state.opponent_cards
            };

            let available_moves =
                core_game::get_available_moves(available_hand, &state.trick.top_set);

            let is_legal = available_moves
                .iter()
                .any(|candidate| moves_are_equal(*candidate, player_move));

            if !is_legal {
                return Err(format!(
                    "Move {:?} is not possible from the currently known information.",
                    player_move,
                ));
            }

            Ok(())
        }
    }
}

fn moves_are_equal(a: core_game::Move, b: core_game::Move) -> bool {
    match (a, b) {
        (core_game::Move::Pass, core_game::Move::Pass) => true,

        (core_game::Move::Play(a), core_game::Move::Play(b)) => {
            a.rank == b.rank && a.num_wilds == b.num_wilds && a.num_non_wilds == b.num_non_wilds
        }

        _ => false,
    }
}
