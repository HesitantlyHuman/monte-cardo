use crate::cards::Move;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use super::cards::TrickHistory;

#[derive(Debug, Clone)]
pub enum Player {
    PerspectivePlayer,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct TopSet {
    top_set_move: Move,
    top_set_player: Player,
}

impl TopSet {
    pub fn new(top_set_move: Move, top_set_player: Player) -> Self {
        TopSet {
            top_set_move,
            top_set_player,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Table {
    current_player: Player,
    top_set: Option<TopSet>,
}

impl Table {
    pub fn new(current_player: Player, top_set: Option<TopSet>) -> Self {
        Table {
            current_player,
            top_set,
        }
    }
}

impl Widget for Table {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width <= 2 || area.height == 0 {
            return;
        }
        // First, turn indicator
        let turn_indicator_message = match &self.current_player {
            Player::PerspectivePlayer => "Your Turn".to_string(),
            Player::Other(name) => {
                if name.ends_with('s') {
                    format!("{}' Turn", name)
                } else {
                    format!("{}'s Turn", name)
                }
            }
        };

        let mut current_y = area.y;
        if area.height >= 12 {
            // Draw our horizontal line
            let line = "─".repeat(area.width as usize);
            let line = Line::from(Span::styled(line, Color::DarkGray));
            line.render(area, buf);
            current_y += 1;
        }

        // Draw the turn indicator centered
        let line = Line::from(Span::raw(turn_indicator_message));
        let paragraph = Paragraph::new(line).alignment(Alignment::Center);
        paragraph.render(
            Rect {
                x: area.x + 1,
                y: current_y,
                width: area.width - 2,
                height: 1,
            },
            buf,
        );
        current_y += 1;

        if current_y >= area.y + area.height {
            return;
        }

        // Now, draw our second horizontal line
        let line = "─".repeat(area.width as usize);
        let line = Line::from(Span::styled(line, Color::DarkGray));
        line.render(
            Rect {
                x: area.x,
                y: current_y,
                width: area.width,
                height: 1,
            },
            buf,
        );
        current_y += 1;

        // Now, we need to create a message for the top set
        let remaining_height = (area.y + area.height) - current_y;
        let message = match &self.top_set {
            Some(top_set) => {
                let mut message = String::new();
                match &top_set.top_set_player {
                    Player::PerspectivePlayer => {
                        message.push_str("You are leading the trick with ")
                    }
                    Player::Other(name) => {
                        message.push_str(&format!("{} is leading the trick with ", name))
                    }
                }
                let num_cards = top_set.top_set_move.num_wilds + top_set.top_set_move.num_non_wilds;
                message.push_str(&format!("{} ", num_cards));
                match top_set.top_set_move.rank {
                    0 => message.push_str("joker"),
                    1 => message.push_str("one"),
                    2 => message.push_str("two"),
                    3 => message.push_str("three"),
                    4 => message.push_str("four"),
                    5 => message.push_str("five"),
                    6 => message.push_str("six"),
                    7 => message.push_str("seven"),
                    8 => message.push_str("eight"),
                    9 => message.push_str("nine"),
                    10 => message.push_str("ten"),
                    11 => message.push_str("eleven"),
                    12 => message.push_str("twelve"),
                    13 => message.push_str("thirteen"),
                    14 => message.push_str("fourteen"),
                    15 => message.push_str("fifteen"),
                    16 => message.push_str("sixteen"),
                    17 => message.push_str("seventeen"),
                    18 => message.push_str("eighteen"),
                    _ => panic!("Invalid rank"),
                }
                if num_cards > 1 {
                    message.push_str("s");
                }
                if remaining_height > 7 {
                    let mut top_padding = (remaining_height - 7) / 2;
                    if top_padding > 0 {
                        top_padding -= 1;
                        current_y += top_padding;
                    }
                }
                message
            }
            None => {
                if remaining_height > 1 {
                    current_y += (remaining_height - 1) / 2;
                }
                match self.current_player {
                    Player::PerspectivePlayer => "Your move to start the trick".to_string(),
                    Player::Other(name) => {
                        if name.ends_with('s') {
                            format!("{}' move to start the trick", name)
                        } else {
                            format!("{}'s move to start the trick", name)
                        }
                    }
                }
            }
        };
        if current_y >= area.y + area.height {
            return;
        }

        let line = Line::from(Span::raw(message));
        let paragraph = Paragraph::new(line)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        paragraph.render(
            Rect {
                x: area.x + 1,
                y: current_y,
                width: area.width - 2,
                height: 2.min(remaining_height),
            },
            buf,
        );

        current_y += 2;
        if current_y >= area.y + area.height {
            return;
        }

        let remaining_height = (area.y + area.height) - current_y;
        // Now, lets draw in our move
        match &self.top_set {
            Some(top_set) => {
                let top_set_move = crate::cards::Move::new(
                    top_set.top_set_move.rank,
                    top_set.top_set_move.num_wilds,
                    top_set.top_set_move.num_non_wilds,
                );
                top_set_move.render(
                    Rect {
                        x: area.x + 1,
                        y: current_y,
                        width: area.width - 2,
                        height: 5.min(remaining_height),
                    },
                    buf,
                );
            }
            None => {}
        }
    }
}

pub struct TableAndTrickHistory {
    table: Table,
    history: TrickHistory,
}

impl TableAndTrickHistory {
    pub fn new(table: Table, history: TrickHistory) -> Self {
        TableAndTrickHistory { table, history }
    }
}

impl Widget for TableAndTrickHistory {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 {
            return;
        }

        let trick_history_width = if area.width <= 80 {
            20.min(area.width - 1)
        } else if area.width >= 165 {
            27
        } else {
            ((330 - area.width) * area.width) / 1000
        };
        let table_width = (area.width - 1) - trick_history_width;

        // Box
        Block::new().borders(Borders::RIGHT).render(
            Rect {
                x: area.x,
                y: area.y,
                width: table_width + 1,
                height: area.height,
            },
            buf,
        );

        // Now we need to patch
        buf[(area.x + table_width, area.y - 1)].set_symbol("┯");
        if buf.area.height > 3 {
            buf[(area.x + table_width, area.y + area.height)].set_symbol("┷");
        }

        if area.height == 0 {
            return;
        }

        // Render table
        self.table.render(
            Rect {
                x: area.x,
                y: area.y,
                width: table_width,
                height: area.height,
            },
            buf,
        );

        // Render trick history
        let span = Span::raw("Trick History");
        let paragraph = Paragraph::new(span).alignment(Alignment::Center);
        paragraph.render(
            Rect {
                x: area.x + table_width + 1,
                y: area.y,
                width: trick_history_width,
                height: 1,
            },
            buf,
        );

        if area.height <= 1 {
            return;
        }

        // Separator
        let line = Line::from(Span::raw("─".repeat(trick_history_width as usize)));
        line.render(
            Rect {
                x: area.x + table_width + 1,
                y: area.y + 1,
                width: trick_history_width,
                height: 1,
            },
            buf,
        );

        // Stich
        buf[(area.x + table_width, area.y + 1)].set_symbol("├");

        if area.height <= 2 {
            return;
        }

        self.history.render(
            Rect {
                x: area.x + table_width + 1,
                y: area.y + 2,
                width: trick_history_width,
                height: area.height - 2,
            },
            buf,
        );
    }
}
