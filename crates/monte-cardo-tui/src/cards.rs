use monte_cardo_core::consts::MAX_CARD_ORDINALITY;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    text::{Line, Span},
    widgets::{Clear, Paragraph, Widget},
};

pub struct Card {
    rank: u8,
    deactivated: bool,
}

impl Card {
    pub fn new(rank: u8, deactivated: bool) -> Self {
        Self { rank, deactivated }
    }

    fn construct_full_card_lines(&self) -> Vec<String> {
        let rank_mark = if self.rank == 0 {
            "J".to_owned()
        } else {
            format!("{}", self.rank)
        };
        let mut lines = vec![CARD_TOP.into()];
        // Now, the top left rank marking
        lines.push(format!("│{:<5}│", rank_mark));
        // Now, the empty line
        lines.push(CARD_EMPTY.into());
        // Now the bottom right rank marking
        lines.push(format!("│{:>5}│", rank_mark));
        // Now the bottom line
        lines.push(CARD_BOTTOM.into());
        lines
    }
}

const CARD_TOP: &str = "╭─────╮";
const CARD_BOTTOM: &str = "╰─────╯";
const CARD_EMPTY: &str = "│  ⬦  │";

impl Widget for Card {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let clear_area = Rect::new(area.x, area.y, area.width.min(7), area.height.min(5));
        Clear.render(clear_area, buf);
        // First, lets collect the lines we need
        let lines = self.construct_full_card_lines();
        // Now, we need to trim our lines to fit the area
        // First, trim the height by setting lines to be only the first area.height lines
        let lines = lines.iter().take(area.height as usize).collect::<Vec<_>>();
        // Now, we need to trim the width by setting each line to be only the first area.width characters
        let lines = lines
            .iter()
            .map(|line| line.chars().take(area.width as usize).collect::<String>())
            .collect::<Vec<_>>();

        let mut formatted_lines: Vec<Line> = Vec::new();
        for line in lines.iter() {
            let span = if self.deactivated {
                Span::styled(line, Color::DarkGray)
            } else {
                Span::raw(line)
            };
            formatted_lines.push(span.into());
        }

        Paragraph::new(formatted_lines).render(area, buf);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub rank: u8,
    pub num_wilds: u8,
    pub num_non_wilds: u8,
}

impl Move {
    pub fn new(rank: u8, num_wilds: u8, num_non_wilds: u8) -> Self {
        Self {
            rank,
            num_wilds,
            num_non_wilds,
        }
    }
}

impl Widget for Move {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // First, calculate our render width
        let num_cards = self.num_wilds + self.num_non_wilds;
        let mut card_spacing = 4;
        let mut render_width = (card_spacing * num_cards as u16) + 3;
        if render_width > area.width {
            card_spacing = 3;
            render_width = (card_spacing * num_cards as u16) + 3;
        }
        let render_height = 5;

        // Now, we want to center our cards in the area provided
        let initial_x = if area.width >= render_width {
            area.x + (area.width - render_width) / 2
        } else {
            area.x
        };
        let initial_y = if area.height >= render_height {
            area.y + (area.height - render_height) / 2
        } else {
            area.y
        };

        // Now, we want to render the cards
        let mut x = initial_x;
        for current_card in 0..(self.num_wilds + self.num_non_wilds) {
            if x >= area.x + area.width {
                break;
            }
            let available_width = area.x + area.width - x;
            let available_height = area.y + area.height - initial_y;

            let card_rank = if current_card < self.num_wilds {
                0
            } else {
                self.rank
            };
            Card::new(card_rank, false).render(
                Rect::new(x, initial_y, available_width, available_height),
                buf,
            );
            x += card_spacing;
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrickHistoryEntry {
    pub player_name: String,
    pub player_move: Move,
}

impl TrickHistoryEntry {
    pub fn new(player_name: String, player_move: Move) -> Self {
        Self {
            player_name,
            player_move,
        }
    }

    fn get_render_settings(&self, available_renderable_width: u16) -> (u16, u16, u16, bool) {
        let mut padding = 2;
        let mut use_alternate_format = false;
        let mut final_card_full_width = true;
        let mut hypothetical_render_width = available_renderable_width;
        let num_cards = self.player_move.num_wilds + self.player_move.num_non_wilds;

        loop {
            // If we have exhausted all of our trimming options, we should just break
            if padding == 0 && use_alternate_format {
                break;
            }

            hypothetical_render_width = if use_alternate_format {
                if self.player_move.num_wilds > 0 && self.player_move.num_non_wilds > 0 {
                    17 + (padding * 2)
                } else {
                    8 + (padding * 2)
                }
            } else {
                if final_card_full_width {
                    (num_cards as u16 * 3 + 4) + (padding * 2)
                } else {
                    (num_cards as u16 * 3) + (padding * 2)
                }
            };

            if hypothetical_render_width > available_renderable_width {
                let difference = hypothetical_render_width - available_renderable_width;
                if use_alternate_format {
                    padding -= 1;
                } else {
                    if final_card_full_width {
                        final_card_full_width = false;
                    } else {
                        if difference > 2 || padding == 1 {
                            use_alternate_format = true;
                            padding = 2;
                        } else {
                            padding -= 1;
                        }
                    }
                }
            } else {
                // We have found a good fit
                break;
            }
        }

        if hypothetical_render_width > available_renderable_width {
            hypothetical_render_width = available_renderable_width;
        }

        // Calculate the x start position to right align the move
        let x_start = (available_renderable_width - hypothetical_render_width) + padding;
        (
            x_start,
            hypothetical_render_width,
            padding,
            use_alternate_format,
        )
    }
}

impl Widget for TrickHistoryEntry {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (cards_x_start, render_width, padding, use_alternate_format) =
            self.get_render_settings(area.width);

        // First, render the player name
        // We want it to be right aligned
        let available_width = area.width - (padding * 2);
        let truncated_player_name = if self.player_name.len() > available_width as usize {
            self.player_name
                .chars()
                .take(available_width as usize)
                .collect::<String>()
        } else {
            // Front pad the player name
            format!(
                "{:>width$}",
                self.player_name,
                width = available_width as usize
            )
        };

        let player_name_line: Line = Span::raw(truncated_player_name).into();
        player_name_line.render(Rect::new(area.x + padding, area.y, available_width, 1), buf);

        if area.height <= 1 {
            return;
        }

        // Now, the move
        if use_alternate_format {
            // First, create a temp buffer to hold our output
            let mut temp_buf = Buffer::empty(Rect::new(0, 0, 17, 2));
            let mut current_write_x = 0;

            // First, render the wilds. Just one card with a height of 2 and a width of 3
            if self.player_move.num_wilds > 0 {
                Card::new(0, false).render(Rect::new(current_write_x, 0, 3, 2), &mut temp_buf);
                // Now, render the multiplication symbol
                let multiplication_symbol: Line = Span::raw("×").into();
                multiplication_symbol
                    .render(Rect::new(current_write_x + 4, 1, 1, 1), &mut temp_buf);

                // Now, render the number of wilds
                let num_wilds: Line = Span::raw(format!("{}", self.player_move.num_wilds)).into();
                num_wilds.render(Rect::new(current_write_x + 6, 1, 2, 1), &mut temp_buf);

                current_write_x += 9;
            }

            // Now, render the non-wilds. Just one card with a height of 2 and a width of 3
            if self.player_move.num_non_wilds > 0 {
                Card::new(self.player_move.rank, false)
                    .render(Rect::new(current_write_x, 0, 3, 2), &mut temp_buf);
                // Now, render the multiplication symbol
                let multiplication_symbol: Line = Span::raw("×").into();
                multiplication_symbol
                    .render(Rect::new(current_write_x + 4, 1, 1, 1), &mut temp_buf);

                // Now, render the number of non-wilds
                let num_non_wilds: Line =
                    Span::raw(format!("{}", self.player_move.num_non_wilds)).into();
                num_non_wilds.render(Rect::new(current_write_x + 6, 1, 2, 1), &mut temp_buf);
            }

            // Now, write the temp buffer to the main buffer, making sure to only write what fits
            for y in 0..2 {
                let new_y = area.y + y + 1;
                if new_y >= area.y + area.height {
                    break;
                }
                for x in 0..17.min(area.width - 1) {
                    // Calculate new positions
                    let new_x = area.x + cards_x_start + x;
                    if new_x >= area.x + area.width {
                        break;
                    }

                    let cell = temp_buf.get(x, y).clone();
                    buf.get_mut(new_x, new_y).set_symbol(cell.symbol());
                }
            }
        } else {
            // Render the move in the standard format
            let move_area = Rect::new(
                area.x + cards_x_start,
                area.y + 1,
                render_width.min(area.width) - (2 * padding),
                2.min(area.height - 1),
            );
            Move::new(
                self.player_move.rank,
                self.player_move.num_wilds,
                self.player_move.num_non_wilds,
            )
            .render(move_area, buf);
        }

        if area.height <= 3 {
            return;
        }

        // Now, render the move separator
        let separator_width = area.width - (2 * padding);
        let separator_line: Line =
            Span::styled("┄".repeat(separator_width as usize), Color::DarkGray).into();
        separator_line.render(
            Rect::new(area.x + padding, area.y + 3, separator_width, 1),
            buf,
        );
    }
}

#[derive(Debug, Clone)]
pub struct TrickHistory {
    pub player_moves: Vec<TrickHistoryEntry>,
}

impl TrickHistory {
    pub fn new(player_moves: Vec<TrickHistoryEntry>) -> Self {
        Self { player_moves }
    }
}

impl Widget for TrickHistory {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Basically just start rendering until we run out of lines
        let mut current_y = area.y;
        for player_move in self.player_moves.iter() {
            if current_y >= area.y + area.height {
                break;
            }

            player_move.clone().render(
                Rect::new(
                    area.x,
                    current_y,
                    area.width,
                    4.min((area.y + area.height) - current_y),
                ),
                buf,
            );
            current_y += 4;
        }
    }
}

pub struct Hand {
    cards: [u8; MAX_CARD_ORDINALITY],
    active: [u8; MAX_CARD_ORDINALITY],
    selected: [u8; MAX_CARD_ORDINALITY],
}

impl Hand {
    pub fn new(
        cards: [u8; MAX_CARD_ORDINALITY],
        active: [u8; MAX_CARD_ORDINALITY],
        selected: [u8; MAX_CARD_ORDINALITY],
    ) -> Self {
        Self {
            cards,
            active,
            selected,
        }
    }
}

impl Widget for Hand {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create a buffer to hold the rendered cards
        let mut num_cards = 0;
        for num_card_of_rank in self.cards.iter() {
            num_cards += num_card_of_rank;
        }

        let buffer_width = 5 * num_cards as u16 + 4;
        let buffer_height = 6;
        let mut buffer = Buffer::empty(Rect::new(0, 0, buffer_width, buffer_height));

        // Now, step through and render the cards
        let mut end_on_selected = false;
        let mut current_x = 0;
        for (rank, num_cards, num_active, num_selected) in self
            .cards
            .iter()
            .zip(self.active.iter())
            .zip(self.selected.iter())
            .enumerate()
            .map(|(index, ((num_cards_of_rank, active), selected))| {
                (index, *num_cards_of_rank, *active, *selected)
            })
        {
            if num_cards == 0 {
                continue;
            }

            let num_selected_and_active = num_selected;
            let num_unselected_and_active = num_active - num_selected_and_active;
            let num_inactive = num_cards - num_active;

            for _ in 0..num_inactive {
                Card::new(rank as u8, true).render(Rect::new(current_x, 1, 7, 5), &mut buffer);
                current_x += 3;

                end_on_selected = false;
            }

            for _ in 0..num_unselected_and_active {
                Card::new(rank as u8, false).render(Rect::new(current_x, 1, 7, 5), &mut buffer);
                current_x += 3;

                end_on_selected = false;
            }

            if num_selected_and_active > 0 && current_x != 0 {
                current_x += 1;
            }

            for _ in 0..num_selected_and_active {
                Card::new(rank as u8, false).render(Rect::new(current_x, 0, 7, 5), &mut buffer);
                current_x += 3;

                end_on_selected = true;
            }

            if num_selected_and_active > 0 {
                current_x += 2;
            }
        }

        // Now, we want to render the buffer to the main buffer such that we center the buffer in the given area
        let final_width = if end_on_selected {
            current_x + 2
        } else {
            current_x + 4
        };
        let final_height = 6;

        let x_start = if area.width >= final_width {
            area.x + (area.width - final_width) / 2
        } else {
            area.x
        };
        let y_start = if area.height >= final_height {
            area.y + (area.height - final_height) / 2
        } else {
            area.y
        };

        for y in 0..final_height {
            let new_y = y_start + y;
            if new_y >= area.y + area.height {
                break;
            }
            for x in 0..final_width {
                let new_x = x_start + x;
                if new_x >= area.x + area.width {
                    break;
                }

                let cell = buffer.get(x, y).clone();
                let buffer_cell = buf.get_mut(new_x, new_y);
                buffer_cell.set_symbol(cell.symbol());
                buffer_cell.set_style(cell.style());
            }
        }
    }
}
