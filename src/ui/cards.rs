use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
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
const CARD_EMPTY: &str = "│     │";

impl Widget for Card {
    fn render(self, area: Rect, buf: &mut Buffer) {
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

pub struct Move {
    rank: u8,
    num_wilds: u8,
    num_non_wilds: u8,
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

pub struct NamedMove {
    player_name: String,
    player_move: Move,
}

pub struct TrickHistory {
    player_moves: Vec<NamedMove>,
}

impl TrickHistory {
    pub fn new(player_moves: Vec<NamedMove>) -> Self {
        Self { player_moves }
    }
}

const ODD_WIDTH_MOVE_SEPARATOR: &str = "---";
const EVEN_WIDTH_MOVE_SEPARATOR: &str = "----";

const ALTERNATE_FORMAT_WIDTH: u16 = 17;

impl Widget for TrickHistory {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut padding = 2; // Try to achieve a padding of 2
        let mut use_alternate_format = false;
        for named_move in self.player_moves.iter() {
            let num_cards = named_move.player_move.num_wilds + named_move.player_move.num_non_wilds;
            let render_width = if use_alternate_format {
                if named_move.player_move.num_wilds > 0 && named_move.player_move.num_non_wilds > 0
                {
                    17
                } else {
                    8
                }
            } else {
                (num_cards as u16 * 3) + (padding * 2)
            };

            // Try to adapt our rendering
            if render_width > area.width {
                let difference = render_width - area.width;
                if difference <= 2 && padding == 2 {
                    padding = 1;
                } else if difference > 2 && padding == 1 {
                    use_alternate_format = true;
                }
            }
        }

        // Basically just start rendering until we run out of lines
    }
}
