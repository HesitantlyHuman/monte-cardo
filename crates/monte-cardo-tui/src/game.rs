use crate::cards::TrickHistory;
use crate::hand::PlayerHand;
use crate::players::Players;
use crate::table::Table;

use ratatui::{
    buffer::Buffer,
    layout::{Layout, Rect},
    style::Color,
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

use super::hand;

#[derive(Debug, Clone)]
pub struct GameState {
    players: Players,
    table: Table,
    pub trick_history: TrickHistory,
    pub player_hand: PlayerHand,
}

impl GameState {
    pub fn new(
        players: Players,
        table: Table,
        trick_history: TrickHistory,
        player_hand: PlayerHand,
    ) -> Self {
        GameState {
            players,
            table,
            trick_history,
            player_hand,
        }
    }
}

impl Widget for GameState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut borders = Borders::ALL;
        if area.height <= 2 {
            borders ^= Borders::BOTTOM;
        }
        Block::new()
            .borders(borders)
            .title(Span::raw("ladder-shedding"))
            .border_type(BorderType::Thick)
            .render(area, buf);

        // Render the header
        let header_area = Rect::new(area.x, area.y, area.width, 3);
        if area.height > 2 {
            Block::new()
                .borders(Borders::BOTTOM)
                .border_type(BorderType::Thick)
                .render(header_area, buf);
        }
        // Split the header into two with a 1 character separation
        let header_left = (header_area.width - 3) / 2;
        let header_right = (header_area.width - 3) - header_left;
        let header_right_message = "Ctrl + Q : Quit to menu";
        let header_right_padding_size = if header_right_message.len() as u16 > header_right {
            0
        } else {
            (header_right - header_right_message.len() as u16).min(3)
        };
        let header_left_message = "Ctrl + C : Quit to terminal";
        let header_left_padding_size = if header_left_message.len() as u16 > header_left {
            0
        } else {
            (header_left - header_left_message.len() as u16).min(3)
        };
        let padding_size = header_right_padding_size.min(header_left_padding_size);

        Paragraph::new(Span::raw(header_right_message)).render(
            Rect {
                x: header_area.x + header_left + 1 + padding_size,
                y: header_area.y + 1,
                width: header_right - padding_size,
                height: 1,
            },
            buf,
        );
        Paragraph::new(Span::raw(header_left_message)).render(
            Rect {
                x: header_area.x + 1 + padding_size,
                y: header_area.y + 1,
                width: header_left - padding_size,
                height: 1,
            },
            buf,
        );
        buf.get_mut(header_area.x + header_left, header_area.y + 1)
            .set_symbol("│");

        if area.height <= 2 {
            return;
        }

        buf.get_mut(area.x, area.y + 2).set_symbol("┣");
        buf.get_mut(area.x + area.width - 1, area.y + 2)
            .set_symbol("┫");
        let player_panel_width = ((area.width as f32 * 0.25) as u16).max(28);
        buf.get_mut(area.x + player_panel_width - 1, area.y + 2)
            .set_symbol("┳");

        if area.height <= 3 {
            return;
        }

        // Render the player box
        let player_panel_height = area.height - 3;
        let player_panel_area =
            Rect::new(area.x, area.y + 3, player_panel_width, player_panel_height);
        Block::new()
            .borders(Borders::RIGHT)
            .border_type(BorderType::Thick)
            .render(player_panel_area, buf);

        // Render the hand panel
        let hand_panel_height = if area.height <= 17 {
            area.height - 3
        } else {
            14
        };
        if area.width > (player_panel_width + 2) {
            let hand_panel_width = area.width - player_panel_width;
            let hand_panel_area = Rect::new(
                area.x + player_panel_width,
                area.y + area.height - hand_panel_height,
                hand_panel_width,
                hand_panel_height,
            );
            Block::new()
                .borders(Borders::TOP)
                .border_type(BorderType::Thick)
                .render(hand_panel_area, buf);
            if hand_panel_area.width > 3 && hand_panel_area.height > 2 {
                self.player_hand.render(
                    Rect {
                        x: hand_panel_area.x,
                        y: hand_panel_area.y + 1,
                        width: hand_panel_area.width - 1,
                        height: hand_panel_area.height - 2,
                    },
                    buf,
                );
            }
            buf.get_mut(
                area.x + player_panel_width - 1,
                area.y + area.height - hand_panel_height,
            )
            .set_symbol("┣");
            buf.get_mut(
                area.x + area.width - 1,
                area.y + area.height - hand_panel_height,
            )
            .set_symbol("┫");

            // Top
            if hand_panel_area.height > 2 {
                buf.get_mut(area.x + player_panel_width - 1, hand_panel_area.y + 2)
                    .set_symbol("┠");
                buf.get_mut(area.x + area.width - 1, hand_panel_area.y + 2)
                    .set_symbol("┨");
            }

            // Bottom
            if hand_panel_area.height > 4 {
                buf.get_mut(area.x + player_panel_width - 1, area.y + area.height - 3)
                    .set_symbol("┠");
                buf.get_mut(area.x + area.width - 1, area.y + area.height - 3)
                    .set_symbol("┨");
            }
        }

        // Render the players
        self.players.render(
            Rect {
                x: player_panel_area.x + 1,
                y: player_panel_area.y,
                width: player_panel_area.width - 1,
                height: player_panel_area.height - 1,
            },
            buf,
        );
        buf.get_mut(area.x + player_panel_width - 1, area.y + area.height - 1)
            .set_symbol("┻");

        // Now, lets render the table and trick history
        let table_and_trick_history =
            crate::table::TableAndTrickHistory::new(self.table, self.trick_history);
        if area.height >= 3 + hand_panel_height {
            table_and_trick_history.render(
                Rect {
                    x: area.x + player_panel_width,
                    y: area.y + 3,
                    width: area.width - player_panel_width - 1,
                    height: area.height - (3 + hand_panel_height),
                },
                buf,
            );
        }
    }
}
