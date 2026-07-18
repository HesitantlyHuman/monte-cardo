use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

use crate::rank_count_editor::RankCountEditor;
use crate::settings::{
    GameMode, GameSettings, PlayerPanelSelection, SettingsField, SettingsFocus, SettingsFormState,
};

const EDITABLE_VALUE_FG: Color = Color::Gray;
const EDITABLE_VALUE_BG: Color = Color::Rgb(22, 22, 22);

const SELECTED_EDITABLE_VALUE_FG: Color = Color::White;
const SELECTED_EDITABLE_VALUE_BG: Color = Color::Rgb(60, 60, 60);

const EDITING_VALUE_FG: Color = Color::LightGreen;
const EDITING_VALUE_BG: Color = Color::Rgb(40, 70, 40);

const SELECTED_LABEL_FG: Color = Color::Gray;
const START_BUTTON_FG: Color = Color::Black;
const START_BUTTON_BG: Color = Color::Green;

pub struct SettingsPage<'a> {
    settings: &'a GameSettings,
    form: &'a SettingsFormState,
}

impl<'a> SettingsPage<'a> {
    pub fn new(settings: &'a GameSettings, form: &'a SettingsFormState) -> Self {
        Self { settings, form }
    }
}

impl Widget for SettingsPage<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        Block::new()
            .title(" Game Settings ")
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .render(area, buf);

        let inner = inner_rect(area, 1, 1);

        if inner.height < 15 {
            Paragraph::new("Terminal is too small for settings.")
                .alignment(Alignment::Center)
                .render(inner, buf);
            return;
        }

        let mode_height = 3;
        let deck_height = 4;
        let footer_height = 2;

        let mode_area = Rect::new(inner.x, inner.y, inner.width, mode_height);

        let deck_area = Rect::new(
            inner.x,
            mode_area.y + mode_area.height,
            inner.width,
            deck_height,
        );

        let footer_area = Rect::new(
            inner.x,
            inner.y + inner.height - footer_height,
            inner.width,
            footer_height,
        );

        let content_y = deck_area.y + deck_area.height;
        let content_height = footer_area.y.saturating_sub(content_y);

        let player_width = if inner.width >= 90 {
            34
        } else {
            (inner.width / 3).max(24)
        }
        .min(inner.width);

        let player_area = Rect::new(inner.x, content_y, player_width, content_height);

        let rules_area = if inner.width > player_width + 1 {
            Rect::new(
                inner.x + player_width + 1,
                content_y,
                inner.width - player_width - 1,
                content_height,
            )
        } else {
            Rect::new(inner.x, content_y, 0, 0)
        };

        ModeSelector::new(self.settings, self.form).render(mode_area, buf);
        RankCountEditor::new(
            "Deck",
            &self.settings.deck,
            &self.form.deck_editor,
            self.form.focus == SettingsFocus::Deck,
            self.settings.inverted_ordering,
        )
        .render(deck_area, buf);
        PlayerNamesPanel::new(self.settings, self.form).render(player_area, buf);

        if rules_area.width > 0 && rules_area.height > 0 {
            RulesPanel::new(self.settings, self.form).render(rules_area, buf);
        }

        StartFooter::new(self.form).render(footer_area, buf);
    }
}

struct ModeSelector<'a> {
    settings: &'a GameSettings,
    form: &'a SettingsFormState,
}

impl<'a> ModeSelector<'a> {
    fn new(settings: &'a GameSettings, form: &'a SettingsFormState) -> Self {
        Self { settings, form }
    }
}

impl Widget for ModeSelector<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 || area.width < 4 {
            return;
        }

        Block::new()
            .title(" Mode ")
            .borders(Borders::ALL)
            .render(area, buf);

        let inner = inner_rect(area, 1, 1);

        let button_width = 22.min(inner.width / 2);
        let gap = 2.min(inner.width.saturating_sub(button_width * 2));
        let total_width = button_width * 2 + gap;
        let start_x = inner.x + inner.width.saturating_sub(total_width) / 2;

        let buttons = [GameMode::PlayComputers, GameMode::PlayLive];

        for (index, mode) in buttons.into_iter().enumerate() {
            let x = start_x + index as u16 * (button_width + gap);

            let has_focus = self.form.focus == SettingsFocus::Mode;
            let is_cursor = has_focus && self.form.mode_cursor == mode;
            let is_active = self.settings.mode == mode;

            let label = if is_active {
                format!("[✓ {} ]", mode.label())
            } else {
                format!("[  {} ]", mode.label())
            };

            let style = if is_cursor {
                Style::default()
                    .fg(START_BUTTON_FG)
                    .bg(START_BUTTON_BG)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default()
                    .fg(EDITING_VALUE_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Paragraph::new(Line::from(Span::styled(label, style)))
                .alignment(Alignment::Center)
                .render(Rect::new(x, inner.y, button_width, 1), buf);
        }
    }
}
struct PlayerNamesPanel<'a> {
    settings: &'a GameSettings,
    form: &'a SettingsFormState,
}

impl<'a> PlayerNamesPanel<'a> {
    fn new(settings: &'a GameSettings, form: &'a SettingsFormState) -> Self {
        Self { settings, form }
    }
}

impl Widget for PlayerNamesPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
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

        let number_selected = self.form.focus == SettingsFocus::Players
            && self.form.player_selection == PlayerPanelSelection::NumberOfPlayers;

        let number_text = if number_selected
            && self.form.player_count_editing
            && !self.form.player_count_edit_buffer.is_empty()
        {
            format!("{}▌", self.form.player_count_edit_buffer)
        } else if number_selected && self.form.player_count_editing {
            format!("{}▌", self.settings.number_of_players)
        } else {
            self.settings.number_of_players.to_string()
        };

        Paragraph::new(Line::from(vec![
            Span::raw("Number of Players: "),
            Span::styled(
                format!(" {:<3}", number_text),
                editable_style(
                    number_selected,
                    number_selected && self.form.player_count_editing,
                ),
            ),
        ]))
        .render(Rect::new(inner.x, inner.y, inner.width, 1), buf);

        if inner.height <= 1 {
            return;
        }

        let separator = "─".repeat(inner.width as usize);
        Paragraph::new(Line::from(Span::styled(separator, Color::DarkGray)))
            .render(Rect::new(inner.x, inner.y + 1, inner.width, 1), buf);

        if inner.height <= 2 {
            return;
        }

        let visible_player_rows = inner.height.saturating_sub(2) as usize;

        let selected_player_index = match self.form.player_selection {
            PlayerPanelSelection::PlayerName(index) => Some(index),
            PlayerPanelSelection::NumberOfPlayers => None,
        };

        let scroll = selected_player_index
            .map(|index| {
                scroll_offset_for_selected(
                    index,
                    visible_player_rows,
                    self.settings.number_of_players,
                )
            })
            .unwrap_or(0);

        for visible_row in 0..visible_player_rows {
            let player_index = scroll + visible_row;

            if player_index >= self.settings.number_of_players {
                break;
            }

            let y = inner.y + 2 + visible_row as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = self.form.focus == SettingsFocus::Players
                && self.form.player_selection == PlayerPanelSelection::PlayerName(player_index);

            let is_editing = is_selected && self.form.player_name_editing;

            let mut name = self
                .settings
                .player_names
                .get(player_index)
                .cloned()
                .unwrap_or_else(|| format!("Player {}", player_index + 1));

            if is_editing {
                name.push('▌');
            }

            Paragraph::new(Line::from(vec![
                Span::raw(format!("P{}: ", player_index + 1)),
                Span::styled(
                    format!(
                        " {:<width$}",
                        name,
                        width = inner.width.saturating_sub(5) as usize
                    ),
                    editable_style(is_selected, is_editing),
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

struct RulesPanel<'a> {
    settings: &'a GameSettings,
    form: &'a SettingsFormState,
}

impl<'a> RulesPanel<'a> {
    fn new(settings: &'a GameSettings, form: &'a SettingsFormState) -> Self {
        Self { settings, form }
    }
}

impl Widget for RulesPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 3 {
            return;
        }

        Block::new()
            .title(" Rules & Solver ")
            .borders(Borders::ALL)
            .render(area, buf);

        let inner = inner_rect(area, 1, 1);

        let fields = SettingsField::visible_rules(self.settings);
        let selected_field = self.form.selected_rule_field(self.settings);

        let scroll = self
            .form
            .rules_index
            .saturating_add(1)
            .saturating_sub(inner.height as usize)
            .min(fields.len().saturating_sub(inner.height as usize));

        for (visible_row, field) in fields.into_iter().skip(scroll).enumerate() {
            let y = inner.y + visible_row as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected =
                self.form.focus == SettingsFocus::Rules && selected_field == Some(field);

            let is_editing = is_selected && self.form.rules_editing;

            let value_text = if is_editing && !self.form.rules_edit_buffer.is_empty() {
                format!("{}▌", self.form.rules_edit_buffer)
            } else if is_editing {
                format!("{}▌", field.value(self.settings))
            } else {
                field.value(self.settings)
            };

            let label_width = (inner.width / 2).max(20).min(inner.width.saturating_sub(8));

            Paragraph::new(Line::from(vec![
                Span::raw(format!(
                    "{:<width$}",
                    field.label(),
                    width = label_width as usize
                )),
                Span::raw(" "),
                Span::styled(
                    format!(
                        " {:<width$}",
                        value_text,
                        width = inner.width.saturating_sub(label_width + 1) as usize
                    ),
                    editable_style(is_selected, is_editing),
                ),
            ]))
            .render(Rect::new(inner.x, y, inner.width, 1), buf);
        }
    }
}

struct StartFooter<'a> {
    form: &'a SettingsFormState,
}

impl<'a> StartFooter<'a> {
    fn new(form: &'a SettingsFormState) -> Self {
        Self { form }
    }
}

impl Widget for StartFooter<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let selected = self.form.focus == SettingsFocus::Start;

        let style = if selected {
            Style::default()
                .fg(START_BUTTON_FG)
                .bg(START_BUTTON_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(SELECTED_LABEL_FG)
        };

        Paragraph::new(Line::from(Span::styled("[ Start Game ]", style)))
            .alignment(Alignment::Center)
            .render(Rect::new(area.x, area.y, area.width, 1), buf);

        if area.height > 1 {
            let controls = match self.form.focus {
                SettingsFocus::Mode => {
                    "Mode: ←/→ choose button    Enter select    ↓ deck    ↑ start"
                }
                SettingsFocus::Deck if self.form.deck_editor.editing => {
                    "Deck count selected: type number or ↑/↓ adjust    Enter/Esc finish"
                }
                SettingsFocus::Deck => {
                    "Deck: ←/→ move rank    Enter edit count    ↑ mode    ↓ panels"
                }
                SettingsFocus::Players if self.form.player_name_editing => {
                    "Editing name: type to edit    Backspace delete    Enter/Esc finish"
                }
                SettingsFocus::Players if self.form.player_count_editing => {
                    "Editing player count: type number or ↑/↓ adjust    Enter/Esc finish"
                }
                SettingsFocus::Players => "Players: ↑/↓ move    Enter edit    ←/→ rules",
                SettingsFocus::Rules if self.form.rules_editing => {
                    "Editing setting: type value or arrows adjust    Enter/Esc finish"
                }
                SettingsFocus::Rules => "Rules: ↑/↓ move    Enter edit/toggle    ←/→ players",
                SettingsFocus::Start => "Start: Enter begin    ↑ players    ↓ mode",
            };

            Paragraph::new(Line::from(Span::styled(
                controls,
                Style::default().fg(Color::DarkGray),
            )))
            .alignment(Alignment::Center)
            .render(Rect::new(area.x, area.y + 1, area.width, 1), buf);
        }
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
            .render(Rect::new(marker_x, inner.y + 2, 1, 1), buf);
    }

    if scroll + visible_rows < total_rows {
        Paragraph::new(Line::from(Span::styled("↓", Color::DarkGray))).render(
            Rect::new(marker_x, inner.y + inner.height.saturating_sub(1), 1, 1),
            buf,
        );
    }
}
