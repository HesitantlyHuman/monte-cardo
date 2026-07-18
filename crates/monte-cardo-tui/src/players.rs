use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style, Styled},
    text::Span,
    widgets::{Clear, Paragraph, Widget},
};

pub struct CardPile {
    size: u16,
}

const CARD_TOP_LEFT: &str = "╭";
const CARD_SIDE: &str = "│";
const CARD_BOTTOM_LEFT: &str = "╰";
const CARD_BACK: &str = "▒";
const CARD_BOTTOM_RIGHT: &str = "╯";
const CARD_TOP_RIGHT: &str = "╮";
const CARD_EDGE: &str = "─";

impl CardPile {
    pub fn new(size: u16) -> Self {
        CardPile { size }
    }
}

impl Widget for CardPile {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.size == 0 {
            return;
        }
        for y in 0..(area.height).min(3) {
            for x in 0..(area.width).min(self.size + 2) {
                if y == 0 {
                    if x == self.size {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_EDGE);
                    } else if x >= self.size + 1 {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_TOP_RIGHT);
                    } else {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_TOP_LEFT);
                    }
                } else if y == 1 {
                    if x == self.size {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_BACK);
                    } else {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_SIDE);
                    }
                } else {
                    if x == self.size {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_EDGE);
                    } else if x >= self.size + 1 {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_BOTTOM_RIGHT);
                    } else {
                        buf[(area.x + x, area.y + y)].set_symbol(CARD_BOTTOM_LEFT);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerState {
    Normal,
    NormalOut,
    Active,
    Leading,
    LeadingOut,
    Passed,
}

#[derive(Debug, Clone)]
pub struct Player {
    name: String,
    state: PlayerState,
    hand_size: u16,
}

impl Player {
    pub fn new(name: String, state: PlayerState, hand_size: u16) -> Self {
        Player {
            name,
            state,
            hand_size,
        }
    }
}

impl Widget for Player {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let padding = 2;

        // Create a temporary buffer to render the player name and card pile
        let card_pile_width = self.hand_size + 3;
        let player_name_width = self.name.len() as u16;

        let temp_buffer_size = (2 + player_name_width.max(card_pile_width + 1)).max(area.width);
        let mut temp_buffer = Buffer::empty(Rect::new(0, 0, temp_buffer_size, 3));

        // Render the card pile
        let card_pile = CardPile::new(self.hand_size);
        card_pile.render(Rect::new(3, 0, card_pile_width, 3), &mut temp_buffer);

        // Render the player name. If the player is out, render the name with a strike-through
        let mut span = Span::raw(self.name);
        if self.state == PlayerState::NormalOut || self.state == PlayerState::LeadingOut {
            span = span.set_style(Style::default().add_modifier(Modifier::CROSSED_OUT));
        }
        let player_name = Paragraph::new(span);
        player_name.render(Rect::new(2, 1, player_name_width, 1), &mut temp_buffer);

        // If the player is leading, render the leading symbol
        if self.state == PlayerState::Leading || self.state == PlayerState::LeadingOut {
            temp_buffer[(0, 1)].set_symbol("★");
        }

        if area.width > (4 + 2 * padding) {
            // Now render the number of cards in the player's hand, 4 characters in from the right
            // Right-align the number of cards in the player's hand
            let hand_size = format!("{:2}", self.hand_size);
            let hand_size = Paragraph::new(Span::raw(hand_size));
            hand_size.render(
                Rect::new(area.width - (4 + 2 * padding), 1, 2, 1),
                &mut temp_buffer,
            );
        }

        // Clear the area
        if self.state == PlayerState::Active {
            Clear.render(area, buf);
        } else {
            Clear.render(Rect::new(area.x, area.y, area.width - 1, area.height), buf);
        }

        // Now, render the temporary buffer to the main buffer
        for y in 0..3 {
            let new_y = area.y + y;
            if new_y >= area.y + area.height {
                break;
            }
            for x in 0..temp_buffer_size {
                let new_x = area.x + x + padding;
                if new_x >= ((area.x + area.width) - 1) - padding {
                    break;
                }
                let cell = &temp_buffer[(x, y)];
                buf[(new_x, new_y)].set_symbol(cell.symbol());
                match self.state {
                    PlayerState::Normal | PlayerState::Leading | PlayerState::Active => {
                        buf[(new_x, new_y)].set_style(cell.style());
                    }
                    PlayerState::NormalOut | PlayerState::LeadingOut | PlayerState::Passed => {
                        buf[(new_x, new_y)].set_style(cell.style().fg(Color::DarkGray));
                    }
                }
            }
        }

        // Now, if the player is active, render the background dark gray
        if self.state == PlayerState::Active {
            for y in area.y..area.height + area.y {
                for x in area.x..area.width + area.x {
                    let style = &buf[(x, y)].style();
                    buf[(x, y)].set_style(style.bg(Color::DarkGray));
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Players {
    players: Vec<Player>,
}

impl Players {
    pub fn new(players: Vec<Player>) -> Self {
        Players { players }
    }
}

impl Widget for Players {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut current_y = area.y;
        for player in self.players {
            if current_y >= area.y + area.height {
                break;
            }
            let remaining_height = area.height - (current_y - area.y);
            let player_area = Rect::new(area.x, current_y, area.width, 3.min(remaining_height));
            player.render(player_area, buf);
            current_y += 3;
        }
    }
}
