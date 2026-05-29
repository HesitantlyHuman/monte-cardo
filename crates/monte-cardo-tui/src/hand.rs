use crate::cards::{Hand, Move};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Layout, Rect},
    style::{Color, Modifier, Style, Styled},
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

#[derive(Debug, Clone)]
pub enum MoveSuggestion {
    Move(Move),
    Pass,
}

#[derive(Debug, Clone)]
pub enum SuggestedMove {
    Suggestion(MoveSuggestion),
    Disabled,
}

#[derive(Debug, Clone)]
pub struct PlayerTurnHand {
    can_pass: bool,
    hand: [u8; monte_cardo_core::consts::MAX_CARD_ORDINALITY],
    suggested_move: SuggestedMove,
    pub available_moves: Vec<Move>,
    currently_selected: Option<usize>,
}

impl PlayerTurnHand {
    pub fn new(
        can_pass: bool,
        hand: [u8; monte_cardo_core::consts::MAX_CARD_ORDINALITY],
        suggested_move: SuggestedMove,
        available_moves: Vec<Move>,
        currently_selected: Option<usize>,
    ) -> Self {
        PlayerTurnHand {
            can_pass,
            hand,
            suggested_move,
            available_moves,
            currently_selected,
        }
    }
}

fn textify_move(move_: Move) -> String {
    if move_.rank == 0 {
        let mut move_text = format!("{} joker", move_.num_wilds);
        if move_.num_wilds > 1 {
            move_text.push('s');
        }
        return move_text;
    }
    let mut move_text = format!("{} wild", move_.num_wilds);
    if move_.num_wilds > 1 {
        move_text.push('s');
    }
    move_text.push_str(format!(" and {} ", move_.num_non_wilds).as_str());
    match move_.rank {
        1 => move_text.push_str("one"),
        2 => move_text.push_str("two"),
        3 => move_text.push_str("three"),
        4 => move_text.push_str("four"),
        5 => move_text.push_str("five"),
        6 => move_text.push_str("six"),
        7 => move_text.push_str("seven"),
        8 => move_text.push_str("eight"),
        9 => move_text.push_str("nine"),
        10 => move_text.push_str("ten"),
        11 => move_text.push_str("eleven"),
        12 => move_text.push_str("twelve"),
        13 => move_text.push_str("thirteen"),
        14 => move_text.push_str("fourteen"),
        15 => move_text.push_str("fifteen"),
        16 => move_text.push_str("sixteen"),
        17 => move_text.push_str("seventeen"),
        18 => move_text.push_str("eighteen"),
        _ => panic!("Invalid rank"),
    }
    if move_.num_non_wilds > 1 {
        move_text.push('s');
    }
    move_text
}

impl Widget for PlayerTurnHand {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // First, draw the selected and suggested moves
        // We will split the area in half, reserving 1 char for a separator
        let selected_move_width = (area.width - 1) / 2;
        let suggested_move_width = (area.width - 1) - selected_move_width;

        let selected_move = if self.available_moves.len() == 0 {
            Span::styled("None", Color::DarkGray)
        } else {
            match self.currently_selected {
                Some(move_idx) => Span::raw(textify_move(self.available_moves[move_idx])),
                None => Span::styled("None", Color::DarkGray),
            }
        };
        let selected_move = Line::from(vec![Span::raw("Selected: "), selected_move]);
        let padding = if selected_move.width() < selected_move_width as usize {
            ((selected_move_width - selected_move.width() as u16) / 2).min(3)
        } else {
            0
        };
        selected_move.render(
            Rect {
                x: area.x + padding,
                y: area.y,
                width: selected_move_width,
                height: 1,
            },
            buf,
        );

        // Render separator
        buf.get_mut(area.x + selected_move_width, area.y)
            .set_symbol(symbols::line::VERTICAL);

        let suggested_move = match self.suggested_move {
            SuggestedMove::Suggestion(MoveSuggestion::Move(move_)) => {
                Span::raw(textify_move(move_))
            }
            SuggestedMove::Suggestion(MoveSuggestion::Pass) => Span::raw("Pass"),
            SuggestedMove::Disabled => Span::styled("Disabled", Color::DarkGray),
        };
        let suggested_move = Line::from(vec![Span::raw("Suggested: "), suggested_move]);
        let padding = if suggested_move.width() < suggested_move_width as usize {
            ((suggested_move_width - suggested_move.width() as u16) / 2).min(3)
        } else {
            0
        };
        suggested_move.render(
            Rect {
                x: area.x + selected_move_width + 1 + padding,
                y: area.y,
                width: suggested_move_width,
                height: 1,
            },
            buf,
        );

        if area.height == 1 {
            return;
        }

        // Now, draw horizontal separator
        let line = "─".repeat(area.width as usize);
        let line = Line::from(Span::raw(line));
        line.render(
            Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: 1,
            },
            buf,
        );

        if area.height == 2 {
            return;
        }

        // Now, draw the hand
        if area.height > 4 {
            let remaining_height = area.height - 4; // Reserving 2 lines at the bottom for the footer info
            let mut available = [0; monte_cardo_core::consts::MAX_CARD_ORDINALITY];
            for move_option in self.available_moves.iter() {
                available[0] = available[0].max(move_option.num_wilds);
                available[move_option.rank as usize] =
                    available[move_option.rank as usize].max(move_option.num_non_wilds);
            }
            let mut selected = [0; monte_cardo_core::consts::MAX_CARD_ORDINALITY];
            if let Some(move_idx) = self.currently_selected {
                let move_option = self.available_moves[move_idx];
                selected[0] = move_option.num_wilds;
                selected[move_option.rank as usize] = move_option
                    .num_non_wilds
                    .max(selected[move_option.rank as usize])
            }
            let hand = Hand::new(self.hand, available, selected);
            hand.render(
                Rect {
                    x: area.x,
                    y: area.y + 2,
                    width: area.width,
                    height: remaining_height,
                },
                buf,
            );
        }

        // Now, draw the footer separator
        let line = "─".repeat(area.width as usize);
        let line = Line::from(Span::raw(line));
        line.render(
            Rect {
                x: area.x,
                y: area.y + area.height - 2,
                width: area.width,
                height: 1,
            },
            buf,
        );

        // Now, render the footer
        // Split into 4 sections with 1 char between each
        let footer_section_width = (area.width - 3) / 4;
        let first_section_width = (area.width - 3) - 3 * footer_section_width;

        // Render the next move indicator
        let next_move = if self.available_moves.len() == 0 {
            Span::styled("→ : Next Move", Color::DarkGray)
        } else {
            match self.currently_selected {
                Some(move_idx) => {
                    if move_idx == self.available_moves.len() - 1 {
                        Span::styled("→ : Next Move", Color::DarkGray)
                    } else {
                        Span::raw("→ : Next Move")
                    }
                }
                None => Span::raw("→ : Next Move"),
            }
        };
        let next_move = Line::from(vec![next_move]);
        let next_move = Paragraph::new(next_move).alignment(Alignment::Center);
        next_move.render(
            Rect {
                x: area.x,
                y: area.y + area.height - 1,
                width: first_section_width,
                height: 1,
            },
            buf,
        );

        // Render separator
        buf.get_mut(area.x + first_section_width, area.y + area.height - 1)
            .set_symbol(symbols::line::VERTICAL);

        // Render previous move indicator
        let previous_move = if self.available_moves.len() == 0 {
            Span::styled("← : Previous Move", Color::DarkGray)
        } else {
            match self.currently_selected {
                Some(move_idx) => {
                    if move_idx == 0 {
                        Span::styled("← : Previous Move", Color::DarkGray)
                    } else {
                        Span::raw("← : Previous Move")
                    }
                }
                None => Span::styled("← : Previous Move", Color::DarkGray),
            }
        };
        let previous_move = Line::from(vec![previous_move]);
        let previous_move = Paragraph::new(previous_move).alignment(Alignment::Center);
        previous_move.render(
            Rect {
                x: area.x + first_section_width + 1,
                y: area.y + area.height - 1,
                width: footer_section_width,
                height: 1,
            },
            buf,
        );

        // Render separator
        buf.get_mut(
            area.x + first_section_width + footer_section_width,
            area.y + area.height - 1,
        )
        .set_symbol(symbols::line::VERTICAL);

        // Render pass indicator
        let pass = if self.can_pass {
            Span::raw("Tab : Pass")
        } else {
            Span::styled("Tab : Pass", Color::DarkGray)
        };
        let pass = Line::from(vec![pass]);
        let pass = Paragraph::new(pass).alignment(Alignment::Center);
        pass.render(
            Rect {
                x: area.x + first_section_width + footer_section_width + 1,
                y: area.y + area.height - 1,
                width: footer_section_width,
                height: 1,
            },
            buf,
        );

        // Render separator
        buf.get_mut(
            area.x + first_section_width + 2 * footer_section_width,
            area.y + area.height - 1,
        )
        .set_symbol(symbols::line::VERTICAL);

        // Render confirmation indicator
        let confirm = match self.currently_selected {
            Some(_) => Span::raw("Enter : Confirm"),
            None => Span::styled("Enter : Confirm", Color::DarkGray),
        };
        let confirm = Line::from(vec![confirm]);
        let confirm = Paragraph::new(confirm).alignment(Alignment::Center);
        confirm.render(
            Rect {
                x: area.x + first_section_width + 2 * footer_section_width + 1,
                y: area.y + area.height - 1,
                width: footer_section_width,
                height: 1,
            },
            buf,
        );
    }
}

#[derive(Debug, Clone)]
pub enum PlayerHand {
    CurrentTurn(PlayerTurnHand),
    NotPlayerTurn([u8; monte_cardo_core::consts::MAX_CARD_ORDINALITY]),
}

impl Widget for PlayerHand {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            PlayerHand::CurrentTurn(hand) => hand.render(area, buf),
            PlayerHand::NotPlayerTurn(hand) => {
                let hand = PlayerTurnHand::new(false, hand, SuggestedMove::Disabled, vec![], None);
                hand.render(area, buf);
            }
        }
    }
}
