use monte_cardo_core::consts;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

const EDITABLE_VALUE_FG: Color = Color::Gray;
const EDITABLE_VALUE_BG: Color = Color::Rgb(22, 22, 22);

const SELECTED_EDITABLE_VALUE_FG: Color = Color::White;
const SELECTED_EDITABLE_VALUE_BG: Color = Color::Rgb(60, 60, 60);

const EDITING_VALUE_FG: Color = Color::LightGreen;
const EDITING_VALUE_BG: Color = Color::Rgb(40, 70, 40);

const RANK_COUNT_LABEL_WIDTH: u16 = 7;
const RANK_COUNT_CELL_WIDTH: u16 = 5;

#[derive(Debug, Clone)]
pub struct RankCountEditorState {
    pub selected_rank: usize,
    pub editing: bool,
    pub edit_buffer: String,
}

impl RankCountEditorState {
    pub fn new() -> Self {
        Self {
            selected_rank: 0,
            editing: false,
            edit_buffer: String::new(),
        }
    }

    pub fn reset(&mut self) {
        self.selected_rank = 0;
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn reset_to_rank(&mut self, rank: usize) {
        self.selected_rank = rank.min(consts::MAX_CARD_ORDINALITY - 1);
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn move_rank(&mut self, delta: isize) {
        if delta > 0 {
            self.selected_rank = (self.selected_rank + 1).min(consts::MAX_CARD_ORDINALITY - 1);
        } else if delta < 0 {
            self.selected_rank = self.selected_rank.saturating_sub(1);
        }
    }

    pub fn start_editing(&mut self) {
        self.editing = true;
        self.edit_buffer.clear();
    }

    pub fn finish_editing(&mut self) {
        self.editing = false;
        self.edit_buffer.clear();
    }

    pub fn adjust_count(
        &mut self,
        counts: &mut [usize; consts::MAX_CARD_ORDINALITY],
        max_counts: &[usize; consts::MAX_CARD_ORDINALITY],
        delta: isize,
    ) {
        let rank = self.selected_rank;
        counts[rank] = adjust_usize(counts[rank], delta, 1, 0, max_counts[rank]);
    }

    pub fn input_digit(
        &mut self,
        counts: &mut [usize; consts::MAX_CARD_ORDINALITY],
        max_counts: &[usize; consts::MAX_CARD_ORDINALITY],
        c: char,
    ) {
        if !c.is_ascii_digit() {
            return;
        }

        self.edit_buffer.push(c);

        if let Ok(value) = self.edit_buffer.parse::<usize>() {
            let rank = self.selected_rank;
            counts[rank] = value.min(max_counts[rank]);
        }
    }

    pub fn backspace_digit(
        &mut self,
        counts: &mut [usize; consts::MAX_CARD_ORDINALITY],
        max_counts: &[usize; consts::MAX_CARD_ORDINALITY],
    ) {
        self.edit_buffer.pop();

        let rank = self.selected_rank;

        if self.edit_buffer.is_empty() {
            counts[rank] = 0;
        } else if let Ok(value) = self.edit_buffer.parse::<usize>() {
            counts[rank] = value.min(max_counts[rank]);
        }
    }

    pub fn clamp_rank(&mut self) {
        self.selected_rank = self.selected_rank.min(consts::MAX_CARD_ORDINALITY - 1);
    }
}

pub fn fixed_max_counts(max: usize) -> [usize; consts::MAX_CARD_ORDINALITY] {
    [max; consts::MAX_CARD_ORDINALITY]
}

pub struct RankCountEditor<'a> {
    title: &'a str,
    counts: &'a [usize; consts::MAX_CARD_ORDINALITY],
    state: &'a RankCountEditorState,
    focused: bool,
    inverted_ordering: bool,
}

impl<'a> RankCountEditor<'a> {
    pub fn new(
        title: &'a str,
        counts: &'a [usize; consts::MAX_CARD_ORDINALITY],
        state: &'a RankCountEditorState,
        focused: bool,
        inverted_ordering: bool,
    ) -> Self {
        Self {
            title,
            counts,
            state,
            focused,
            inverted_ordering,
        }
    }
}

impl Widget for RankCountEditor<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        Block::new()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .render(area, buf);

        let inner = inner_rect(area, 1, 1);

        if inner.width <= RANK_COUNT_LABEL_WIDTH || inner.height < 2 {
            return;
        }

        let label_style = Style::default().fg(Color::Gray);

        Paragraph::new(Line::from(Span::styled(" Rank", label_style)))
            .render(Rect::new(inner.x, inner.y, RANK_COUNT_LABEL_WIDTH, 1), buf);

        Paragraph::new(Line::from(Span::styled(" Count", label_style))).render(
            Rect::new(inner.x, inner.y + 1, RANK_COUNT_LABEL_WIDTH, 1),
            buf,
        );

        let mut ranks: Vec<usize> = (0..consts::MAX_CARD_ORDINALITY).collect();

        if self.inverted_ordering {
            let mut non_wilds: Vec<usize> = (1..consts::MAX_CARD_ORDINALITY).collect();
            non_wilds.reverse();

            ranks.clear();
            ranks.push(0);
            ranks.extend(non_wilds);
        }

        let available_width = inner.width - RANK_COUNT_LABEL_WIDTH;
        let max_cells = (available_width / RANK_COUNT_CELL_WIDTH).max(1) as usize;

        for (display_index, rank) in ranks.into_iter().take(max_cells).enumerate() {
            let x = inner.x + RANK_COUNT_LABEL_WIDTH + display_index as u16 * RANK_COUNT_CELL_WIDTH;

            let selected = self.focused && self.state.selected_rank == rank;
            let editing = selected && self.state.editing;

            let rank_label = if rank == 0 {
                "W".to_string()
            } else {
                rank.to_string()
            };

            Paragraph::new(Line::from(Span::raw(format!("{:^4}", rank_label))))
                .render(Rect::new(x, inner.y, RANK_COUNT_CELL_WIDTH, 1), buf);

            let count_text = if editing && !self.state.edit_buffer.is_empty() {
                format!("{}▌", self.state.edit_buffer)
            } else if editing {
                format!("{}▌", self.counts[rank])
            } else {
                self.counts[rank].to_string()
            };

            Paragraph::new(Line::from(Span::styled(
                format!("{:^4}", count_text),
                editable_style(selected, editing),
            )))
            .render(Rect::new(x, inner.y + 1, RANK_COUNT_CELL_WIDTH, 1), buf);
        }
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
