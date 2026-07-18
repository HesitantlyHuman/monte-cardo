use monte_cardo_core::consts;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

use crate::rank_count_editor::{RankCountEditor, RankCountEditorState};
use crate::settings::GameSettings;

const EDITABLE_VALUE_FG: Color = Color::Gray;
const EDITABLE_VALUE_BG: Color = Color::Rgb(22, 22, 22);

const SELECTED_EDITABLE_VALUE_FG: Color = Color::White;
const SELECTED_EDITABLE_VALUE_BG: Color = Color::Rgb(60, 60, 60);

const EDITING_VALUE_FG: Color = Color::LightGreen;
const EDITING_VALUE_BG: Color = Color::Rgb(40, 70, 40);

const SELECTED_BUTTON_FG: Color = Color::Black;
const SELECTED_BUTTON_BG: Color = Color::Green;

const MUTED_FG: Color = Color::DarkGray;

const LIVE_PLAYER_START_COL_WIDTH: usize = 8;
const LIVE_PLAYER_NAME_COL_WIDTH: usize = 28;
const LIVE_PLAYER_CARDS_COL_WIDTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveSetupFocus {
    Hand,
    Players,
    Start,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivePlayerColumn {
    StartingPlayer,
    HandSize,
}

#[derive(Debug, Clone)]
pub struct LiveSetupState {
    pub focus: LiveSetupFocus,

    pub hand_counts: [usize; consts::MAX_CARD_ORDINALITY],
    pub hand_editor: RankCountEditorState,

    pub hand_sizes: [usize; consts::MAX_PLAYERS],
    pub hand_size_player: usize,
    pub player_column: LivePlayerColumn,
    pub hand_size_editing: bool,
    pub hand_size_edit_buffer: String,

    pub starting_player: usize,
}

impl LiveSetupState {
    pub fn new() -> Self {
        Self {
            focus: LiveSetupFocus::Hand,

            hand_counts: [0; consts::MAX_CARD_ORDINALITY],
            hand_editor: RankCountEditorState::new(),

            hand_sizes: [0; consts::MAX_PLAYERS],
            hand_size_player: 0,
            player_column: LivePlayerColumn::StartingPlayer,
            hand_size_editing: false,
            hand_size_edit_buffer: String::new(),

            starting_player: 0,
        }
    }

    pub fn clamp_player_column(&mut self) {
        if self.hand_size_player == 0 && self.player_column == LivePlayerColumn::HandSize {
            self.player_column = LivePlayerColumn::StartingPlayer;
        }
    }

    pub fn reset_from_settings(&mut self, settings: &GameSettings) {
        self.focus = LiveSetupFocus::Hand;

        self.hand_counts = [0; consts::MAX_CARD_ORDINALITY];
        self.hand_editor.reset();

        self.hand_sizes = [0; consts::MAX_PLAYERS];

        let total_cards: usize = settings.deck.iter().sum();
        let base = total_cards / settings.number_of_players;
        let remainder = total_cards % settings.number_of_players;

        for player_index in 0..settings.number_of_players {
            self.hand_sizes[player_index] = base + usize::from(player_index < remainder);
        }

        self.hand_size_player = 0;
        self.player_column = LivePlayerColumn::StartingPlayer;
        self.hand_size_editing = false;
        self.hand_size_edit_buffer.clear();

        self.starting_player = 0;
        self.sync_perspective_hand_size();
    }

    pub fn sync_perspective_hand_size(&mut self) {
        self.hand_sizes[0] = self.hand_counts.iter().sum();
    }

    pub fn move_hand_rank(&mut self, delta: isize) {
        self.hand_editor.move_rank(delta);
    }

    pub fn start_hand_editing(&mut self) {
        self.hand_editor.start_editing();
    }

    pub fn finish_hand_editing(&mut self) {
        self.hand_editor.finish_editing();
    }

    pub fn adjust_hand_count(&mut self, settings: &GameSettings, delta: isize) {
        self.hand_editor
            .adjust_count(&mut self.hand_counts, &settings.deck, delta);

        self.sync_perspective_hand_size();
    }

    pub fn input_hand_digit(&mut self, settings: &GameSettings, c: char) {
        self.hand_editor
            .input_digit(&mut self.hand_counts, &settings.deck, c);

        self.sync_perspective_hand_size();
    }

    pub fn backspace_hand_digit(&mut self, settings: &GameSettings) {
        self.hand_editor
            .backspace_digit(&mut self.hand_counts, &settings.deck);

        self.sync_perspective_hand_size();
    }

    pub fn move_hand_size_player(&mut self, settings: &GameSettings, delta: isize) {
        if delta > 0 {
            self.hand_size_player = (self.hand_size_player + 1).min(settings.number_of_players - 1);
        } else if delta < 0 {
            self.hand_size_player = self.hand_size_player.saturating_sub(1);
        }

        self.clamp_player_column();
    }

    pub fn current_hand_size_is_editable(&self) -> bool {
        self.player_column == LivePlayerColumn::HandSize && self.hand_size_player != 0
    }

    pub fn start_hand_size_editing(&mut self) {
        if self.current_hand_size_is_editable() {
            self.hand_size_editing = true;
            self.hand_size_edit_buffer.clear();
        }
    }

    pub fn finish_hand_size_editing(&mut self) {
        self.hand_size_editing = false;
        self.hand_size_edit_buffer.clear();
    }

    pub fn adjust_hand_size(&mut self, settings: &GameSettings, delta: isize) {
        if !self.current_hand_size_is_editable() {
            return;
        }

        self.hand_sizes[self.hand_size_player] = adjust_usize(
            self.hand_sizes[self.hand_size_player],
            delta,
            1,
            0,
            settings.deck.iter().sum(),
        );
    }

    pub fn input_hand_size_digit(&mut self, settings: &GameSettings, c: char) {
        if !self.current_hand_size_is_editable() || !c.is_ascii_digit() {
            return;
        }

        self.hand_size_edit_buffer.push(c);

        if let Ok(value) = self.hand_size_edit_buffer.parse::<usize>() {
            self.hand_sizes[self.hand_size_player] = value.min(settings.deck.iter().sum());
        }
    }

    pub fn backspace_hand_size_digit(&mut self) {
        if !self.current_hand_size_is_editable() {
            return;
        }

        self.hand_size_edit_buffer.pop();

        if self.hand_size_edit_buffer.is_empty() {
            self.hand_sizes[self.hand_size_player] = 0;
        } else if let Ok(value) = self.hand_size_edit_buffer.parse::<usize>() {
            self.hand_sizes[self.hand_size_player] = value;
        }
    }

    // TODO: Why do we not need this?
    // pub fn adjust_starting_player(&mut self, settings: &GameSettings, delta: isize) {
    //     if delta > 0 {
    //         self.starting_player = (self.starting_player + 1).min(settings.number_of_players - 1);
    //     } else if delta < 0 {
    //         self.starting_player = self.starting_player.saturating_sub(1);
    //     }
    // }

    pub fn move_player_column(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }

        self.player_column = match self.player_column {
            LivePlayerColumn::StartingPlayer => {
                if self.hand_size_player == 0 {
                    LivePlayerColumn::StartingPlayer
                } else {
                    LivePlayerColumn::HandSize
                }
            }
            LivePlayerColumn::HandSize => LivePlayerColumn::StartingPlayer,
        };
    }

    pub fn select_current_player_as_starting_player(&mut self) {
        self.starting_player = self.hand_size_player;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedAction {
    Play,
    Pass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedMoveFocus {
    Action,
    Rank,
    NonWilds,
    Wilds,
    Submit,
}

#[derive(Debug, Clone)]
pub struct ObservedMoveInputState {
    pub focus: ObservedMoveFocus,
    pub action: ObservedAction,
    pub rank: usize,
    pub non_wilds: usize,
    pub wilds: usize,

    pub editing_count: bool,
    pub edit_buffer: String,
}

impl ObservedMoveInputState {
    pub fn new() -> Self {
        Self {
            focus: ObservedMoveFocus::Action,
            action: ObservedAction::Play,
            rank: 1,
            non_wilds: 1,
            wilds: 0,

            editing_count: false,
            edit_buffer: String::new(),
        }
    }

    pub fn reset_for_next_move(&mut self) {
        self.focus = ObservedMoveFocus::Action;
        self.action = ObservedAction::Play;
        self.rank = 1;
        self.non_wilds = 1;
        self.wilds = 0;
        self.editing_count = false;
        self.edit_buffer.clear();
    }

    pub fn move_focus_up(&mut self) {
        self.focus = match self.focus {
            ObservedMoveFocus::Action => ObservedMoveFocus::Submit,

            ObservedMoveFocus::Rank | ObservedMoveFocus::NonWilds | ObservedMoveFocus::Wilds => {
                ObservedMoveFocus::Action
            }

            ObservedMoveFocus::Submit => {
                if self.action == ObservedAction::Play {
                    ObservedMoveFocus::Wilds
                } else {
                    ObservedMoveFocus::Action
                }
            }
        };
    }

    pub fn move_focus_down(&mut self) {
        self.focus = match self.focus {
            ObservedMoveFocus::Action => {
                if self.action == ObservedAction::Play {
                    ObservedMoveFocus::Rank
                } else {
                    ObservedMoveFocus::Submit
                }
            }

            ObservedMoveFocus::Rank | ObservedMoveFocus::NonWilds | ObservedMoveFocus::Wilds => {
                ObservedMoveFocus::Submit
            }

            ObservedMoveFocus::Submit => ObservedMoveFocus::Action,
        };
    }

    pub fn move_left_right(&mut self, delta: isize) {
        match self.focus {
            ObservedMoveFocus::Action => {
                if delta != 0 {
                    self.action = match self.action {
                        ObservedAction::Play => ObservedAction::Pass,
                        ObservedAction::Pass => ObservedAction::Play,
                    };
                }
            }

            ObservedMoveFocus::Rank => {
                self.focus = if delta < 0 {
                    ObservedMoveFocus::Wilds
                } else {
                    ObservedMoveFocus::NonWilds
                };
            }

            ObservedMoveFocus::NonWilds => {
                self.focus = if delta < 0 {
                    ObservedMoveFocus::Rank
                } else {
                    ObservedMoveFocus::Wilds
                };
            }

            ObservedMoveFocus::Wilds => {
                self.focus = if delta < 0 {
                    ObservedMoveFocus::NonWilds
                } else {
                    ObservedMoveFocus::Rank
                };
            }

            ObservedMoveFocus::Submit => {}
        }
    }

    pub fn start_count_editing(&mut self) {
        if matches!(
            self.focus,
            ObservedMoveFocus::Rank | ObservedMoveFocus::NonWilds | ObservedMoveFocus::Wilds
        ) {
            self.editing_count = true;
            self.edit_buffer.clear();
        }
    }

    pub fn finish_count_editing(&mut self) {
        self.editing_count = false;
        self.edit_buffer.clear();
    }

    pub fn adjust_current_count(&mut self, delta: isize) {
        match self.focus {
            ObservedMoveFocus::Rank => {
                self.rank = adjust_usize(self.rank, delta, 1, 1, consts::MAX_CARD_ORDINALITY - 1);
            }

            ObservedMoveFocus::NonWilds => {
                self.non_wilds = adjust_usize(self.non_wilds, delta, 1, 0, 99);
            }

            ObservedMoveFocus::Wilds => {
                self.wilds = adjust_usize(self.wilds, delta, 1, 0, 99);
            }

            _ => {}
        }
    }

    pub fn input_count_digit(&mut self, c: char) {
        if !c.is_ascii_digit() {
            return;
        }

        self.edit_buffer.push(c);

        if let Ok(value) = self.edit_buffer.parse::<usize>() {
            match self.focus {
                ObservedMoveFocus::Rank => {
                    self.rank = value.clamp(1, consts::MAX_CARD_ORDINALITY - 1);
                }
                ObservedMoveFocus::NonWilds => self.non_wilds = value.min(99),
                ObservedMoveFocus::Wilds => self.wilds = value.min(99),
                _ => {}
            }
        }
    }

    pub fn backspace_count_digit(&mut self) {
        self.edit_buffer.pop();

        let value = if self.edit_buffer.is_empty() {
            0
        } else {
            self.edit_buffer.parse::<usize>().unwrap_or(0).min(99)
        };

        match self.focus {
            ObservedMoveFocus::Rank => {
                self.rank = value.clamp(1, consts::MAX_CARD_ORDINALITY - 1);
            }
            ObservedMoveFocus::NonWilds => self.non_wilds = value,
            ObservedMoveFocus::Wilds => self.wilds = value,
            _ => {}
        }
    }
}

pub struct LiveSetupPage<'a> {
    settings: &'a GameSettings,
    state: &'a LiveSetupState,
    error: Option<&'a str>,
}

impl<'a> LiveSetupPage<'a> {
    pub fn new(
        settings: &'a GameSettings,
        state: &'a LiveSetupState,
        error: Option<&'a str>,
    ) -> Self {
        Self {
            settings,
            state,
            error,
        }
    }
}

impl Widget for LiveSetupPage<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 8 {
            return;
        }

        Block::new()
            .title(" Live Game Setup ")
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .render(area, buf);

        let inner = inner_rect(area, 1, 1);

        let hand_height = 4;
        let footer_height = 3;

        let hand_area = Rect::new(inner.x, inner.y, inner.width, hand_height);

        let footer_area = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(footer_height),
            inner.width,
            footer_height,
        );

        let content_y = hand_area.y + hand_area.height;
        let content_height = footer_area.y.saturating_sub(content_y);

        let players_area = Rect::new(inner.x, content_y, inner.width, content_height);

        RankCountEditor::new(
            "Your Dealt Hand",
            &self.state.hand_counts,
            &self.state.hand_editor,
            self.state.focus == LiveSetupFocus::Hand,
            self.settings.inverted_ordering,
        )
        .render(hand_area, buf);

        LivePlayersPanel::new(self.settings, self.state).render(players_area, buf);

        LiveSetupFooter::new(self.state, self.error).render(footer_area, buf);
    }
}

struct LivePlayersPanel<'a> {
    settings: &'a GameSettings,
    state: &'a LiveSetupState,
}

impl<'a> LivePlayersPanel<'a> {
    fn new(settings: &'a GameSettings, state: &'a LiveSetupState) -> Self {
        Self { settings, state }
    }
}

impl Widget for LivePlayersPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        Block::new()
            .title(" Players ")
            .borders(Borders::ALL)
            .render(area, buf);

        let inner = inner_rect(area, 1, 1);

        if inner.height == 0 {
            return;
        }

        let header_style = Style::default().fg(Color::Gray);

        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{:<width$}", "Start", width = LIVE_PLAYER_START_COL_WIDTH),
                header_style,
            ),
            Span::styled(
                format!("{:<width$}", "Player", width = LIVE_PLAYER_NAME_COL_WIDTH),
                header_style,
            ),
            Span::styled(
                format!("{:<width$}", "Cards", width = LIVE_PLAYER_CARDS_COL_WIDTH),
                header_style,
            ),
        ]))
        .render(Rect::new(inner.x, inner.y, inner.width, 1), buf);

        if inner.height <= 1 {
            return;
        }

        let visible_player_rows = inner.height.saturating_sub(1) as usize;

        let scroll = scroll_offset_for_selected(
            self.state.hand_size_player,
            visible_player_rows,
            self.settings.number_of_players,
        );

        for visible_row in 0..visible_player_rows {
            let player_index = scroll + visible_row;

            if player_index >= self.settings.number_of_players {
                break;
            }

            let y = inner.y + 1 + visible_row as u16;

            if y >= inner.y + inner.height {
                break;
            }

            let row_selected = self.state.focus == LiveSetupFocus::Players
                && self.state.hand_size_player == player_index;

            let start_selected =
                row_selected && self.state.player_column == LivePlayerColumn::StartingPlayer;

            let cards_selected =
                row_selected && self.state.player_column == LivePlayerColumn::HandSize;

            let cards_editing = cards_selected && self.state.hand_size_editing;

            let label = self
                .settings
                .player_names
                .get(player_index)
                .map(String::as_str)
                .unwrap_or("Player");

            let start_symbol = if self.state.starting_player == player_index {
                "◉"
            } else {
                "○"
            };

            let start_style = if start_selected {
                Style::default()
                    .fg(SELECTED_BUTTON_FG)
                    .bg(SELECTED_BUTTON_BG)
                    .add_modifier(Modifier::BOLD)
            } else if self.state.starting_player == player_index {
                Style::default()
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let value_text = if cards_editing && !self.state.hand_size_edit_buffer.is_empty() {
                format!("{}▌", self.state.hand_size_edit_buffer)
            } else if cards_editing {
                format!("{}▌", self.state.hand_sizes[player_index])
            } else {
                self.state.hand_sizes[player_index].to_string()
            };

            let cards_style = if player_index == 0 {
                Style::default().fg(Color::DarkGray)
            } else {
                editable_style(cards_selected, cards_editing)
            };

            let player_text = format!("P{} {}", player_index + 1, label);

            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(
                        "{:<width$}",
                        start_symbol,
                        width = LIVE_PLAYER_START_COL_WIDTH
                    ),
                    start_style,
                ),
                Span::raw(format!(
                    "{:<width$}",
                    player_text,
                    width = LIVE_PLAYER_NAME_COL_WIDTH,
                )),
                Span::styled(
                    format!(
                        "{:<width$}",
                        value_text,
                        width = LIVE_PLAYER_CARDS_COL_WIDTH
                    ),
                    cards_style,
                ),
            ]))
            .render(Rect::new(inner.x, y, inner.width, 1), buf);
        }

        render_scroll_markers(
            inner,
            scroll,
            visible_player_rows,
            self.settings.number_of_players,
            buf,
        );
    }
}
struct LiveSetupFooter<'a> {
    state: &'a LiveSetupState,
    error: Option<&'a str>,
}

impl<'a> LiveSetupFooter<'a> {
    fn new(state: &'a LiveSetupState, error: Option<&'a str>) -> Self {
        Self { state, error }
    }
}

impl Widget for LiveSetupFooter<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let start_selected = self.state.focus == LiveSetupFocus::Start;

        let start_style = if start_selected {
            Style::default()
                .fg(SELECTED_BUTTON_FG)
                .bg(SELECTED_BUTTON_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        Paragraph::new(Line::from(Span::styled("[ Start Live Game ]", start_style)))
            .alignment(Alignment::Center)
            .render(Rect::new(area.x, area.y, area.width, 1), buf);

        if let Some(error) = self.error {
            if area.height > 1 {
                Paragraph::new(Line::from(Span::styled(error, Color::LightRed)))
                    .alignment(Alignment::Center)
                    .render(Rect::new(area.x, area.y + 1, area.width, 1), buf);
            }
        }

        if area.height > 2 {
            let controls = match self.state.focus {
                LiveSetupFocus::Hand if self.state.hand_editor.editing => {
                    "Hand count: type number or ↑/↓ adjust    Enter/Esc finish"
                }
                LiveSetupFocus::Hand => {
                    "Hand: ←/→ rank    Enter edit count    ↓ players    Esc settings"
                }
                LiveSetupFocus::Players if self.state.hand_size_editing => {
                    "Hand size: type number or ↑/→ adjust    ↓/← adjust    Enter/Esc finish"
                }
                LiveSetupFocus::Players => "Players: ↑/↓ player    ←/→ column    Enter select/edit",
                LiveSetupFocus::Start => "Start: Enter begin    ↑ players    ↓ hand",
            };

            Paragraph::new(Line::from(Span::styled(controls, MUTED_FG)))
                .alignment(Alignment::Center)
                .render(Rect::new(area.x, area.y + 2, area.width, 1), buf);
        }
    }
}

pub struct ObservedMovePanel<'a> {
    state: &'a ObservedMoveInputState,
    player_name: &'a str,
}

impl<'a> ObservedMovePanel<'a> {
    pub fn new(state: &'a ObservedMoveInputState, player_name: &'a str) -> Self {
        Self { state, player_name }
    }
}

impl Widget for ObservedMovePanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        Block::new()
            .title(format!(" Input Move: {} ", self.player_name))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .render(area, buf);

        let inner = inner_rect(area, 1, 1);

        let action_selected = self.state.focus == ObservedMoveFocus::Action;

        let play_style = button_style(
            action_selected && self.state.action == ObservedAction::Play,
            self.state.action == ObservedAction::Play,
        );

        let pass_style = button_style(
            action_selected && self.state.action == ObservedAction::Pass,
            self.state.action == ObservedAction::Pass,
        );

        // One row of space above this selection.
        Paragraph::new(Line::from(vec![
            Span::styled("[ Play ]", play_style),
            Span::raw("   "),
            Span::styled("[ Pass ]", pass_style),
        ]))
        .alignment(Alignment::Center)
        .render(Rect::new(inner.x, inner.y + 1, inner.width, 1), buf);

        if self.state.action == ObservedAction::Play {
            let rank_selected = self.state.focus == ObservedMoveFocus::Rank;
            let non_wilds_selected = self.state.focus == ObservedMoveFocus::NonWilds;
            let wilds_selected = self.state.focus == ObservedMoveFocus::Wilds;

            let rank_text = count_text(
                self.state.rank,
                rank_selected && self.state.editing_count,
                &self.state.edit_buffer,
            );

            let non_wilds_text = count_text(
                self.state.non_wilds,
                non_wilds_selected && self.state.editing_count,
                &self.state.edit_buffer,
            );

            let wilds_text = count_text(
                self.state.wilds,
                wilds_selected && self.state.editing_count,
                &self.state.edit_buffer,
            );

            Paragraph::new(Line::from(vec![
                Span::raw("Rank: "),
                Span::styled(
                    format!(" {:<3}", rank_text),
                    editable_style(rank_selected, rank_selected && self.state.editing_count),
                ),
                Span::raw("   Non-wilds: "),
                Span::styled(
                    format!(" {:<3}", non_wilds_text),
                    editable_style(
                        non_wilds_selected,
                        non_wilds_selected && self.state.editing_count,
                    ),
                ),
                Span::raw("   Wilds: "),
                Span::styled(
                    format!(" {:<3}", wilds_text),
                    editable_style(wilds_selected, wilds_selected && self.state.editing_count),
                ),
            ]))
            .alignment(Alignment::Center)
            .render(Rect::new(inner.x, inner.y + 3, inner.width, 1), buf);
        }

        let submit_selected = self.state.focus == ObservedMoveFocus::Submit;

        let submit_style = if submit_selected {
            Style::default()
                .fg(SELECTED_BUTTON_FG)
                .bg(SELECTED_BUTTON_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        Paragraph::new(Line::from(Span::styled("[ Confirm ]", submit_style)))
            .alignment(Alignment::Center)
            .render(Rect::new(inner.x, inner.y + 5, inner.width, 1), buf);

        if inner.height > 7 {
            Paragraph::new(Line::from(Span::styled(
                observed_move_controls(self.state),
                MUTED_FG,
            )))
            .alignment(Alignment::Center)
            .render(Rect::new(inner.x, inner.y + 7, inner.width, 1), buf);
        }
    }
}

fn button_style(selected: bool, active: bool) -> Style {
    if selected {
        Style::default()
            .fg(SELECTED_BUTTON_FG)
            .bg(SELECTED_BUTTON_BG)
            .add_modifier(Modifier::BOLD)
    } else if active {
        Style::default()
            .fg(Color::LightGreen)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn count_text(value: usize, editing: bool, buffer: &str) -> String {
    if editing && !buffer.is_empty() {
        format!("{}▌", buffer)
    } else if editing {
        format!("{}▌", value)
    } else {
        value.to_string()
    }
}

fn editable_style(selected: bool, editing: bool) -> Style {
    if editing {
        Style::default()
            .fg(EDITING_VALUE_FG)
            .bg(EDITING_VALUE_BG)
            .add_modifier(Modifier::BOLD)
    } else if selected {
        Style::default()
            .fg(SELECTED_EDITABLE_VALUE_FG)
            .bg(SELECTED_EDITABLE_VALUE_BG)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(EDITABLE_VALUE_FG).bg(EDITABLE_VALUE_BG)
    }
}

fn adjust_usize(value: usize, delta: isize, step: usize, min: usize, max: usize) -> usize {
    if delta > 0 {
        value
            .saturating_add(step.saturating_mul(delta as usize))
            .min(max)
    } else if delta < 0 {
        value
            .saturating_sub(step.saturating_mul((-delta) as usize))
            .max(min)
    } else {
        value
    }
}

fn inner_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x + horizontal,
        y: area.y + vertical,
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
    }
}

fn observed_move_controls(state: &ObservedMoveInputState) -> &'static str {
    if state.editing_count {
        return "Type value    ↑/→ +1    ↓/← -1    Enter/Esc finish";
    }

    match state.focus {
        ObservedMoveFocus::Action => {
            if state.action == ObservedAction::Play {
                "←/→ move type    ↓ play info    ↑ confirm"
            } else {
                "←/→ move type    ↓ confirm    ↑ confirm"
            }
        }

        ObservedMoveFocus::Rank | ObservedMoveFocus::NonWilds | ObservedMoveFocus::Wilds => {
            "←/→ field    Enter edit    ↑ move type    ↓ confirm"
        }

        ObservedMoveFocus::Submit => "Enter confirm    ↑ move info    ↓ move type",
    }
}

fn scroll_offset_for_selected(
    selected_index: usize,
    visible_rows: usize,
    total_rows: usize,
) -> usize {
    if visible_rows == 0 || total_rows <= visible_rows {
        return 0;
    }

    selected_index
        .saturating_add(1)
        .saturating_sub(visible_rows)
        .min(total_rows.saturating_sub(visible_rows))
}

fn render_scroll_markers(
    inner: Rect,
    scroll: usize,
    visible_rows: usize,
    total_rows: usize,
    buf: &mut Buffer,
) {
    if visible_rows == 0 || total_rows <= visible_rows || inner.width == 0 {
        return;
    }

    let marker_x = inner.x + inner.width.saturating_sub(2);

    if scroll > 0 {
        Paragraph::new(Line::from(Span::styled("↑", Color::DarkGray)))
            .render(Rect::new(marker_x, inner.y + 1, 1, 1), buf);
    }

    if scroll + visible_rows < total_rows {
        Paragraph::new(Line::from(Span::styled("↓", Color::DarkGray))).render(
            Rect::new(marker_x, inner.y + inner.height.saturating_sub(1), 1, 1),
            buf,
        );
    }
}
