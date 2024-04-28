use ratatui::{
    buffer::Buffer,
    layout::{Layout, Rect},
    style::Color,
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

pub struct GameState {}

impl GameState {
    pub fn new() -> Self {
        GameState {}
    }
}

impl Widget for GameState {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Block::new()
            .borders(Borders::ALL)
            .title(Span::raw("ladder-shedding"))
            .border_type(BorderType::Thick)
            .render(area, buf);

        let header_area = Rect::new(area.x, area.y, area.width, 3);
        Block::new()
            .borders(Borders::BOTTOM)
            .border_type(BorderType::Thick)
            .render(header_area, buf);

        // 28 chars, 25 percent
        let player_panel_width = ((area.width as f32 * 0.25) as u16).max(28);
        let player_panel_height = area.height - 3;
        let player_panel_area =
            Rect::new(area.x, area.y + 3, player_panel_width, player_panel_height);
        Block::new()
            .borders(Borders::RIGHT)
            .border_type(BorderType::Thick)
            .render(player_panel_area, buf);

        // 14 Height
        let hand_panel_height = 14;
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

        // Now, we want to fix the intersections
        buf.get_mut(area.x, area.y + 2).set_symbol("┣");
        buf.get_mut(area.x + area.width - 1, area.y + 2)
            .set_symbol("┫");
        buf.get_mut(area.x + player_panel_width - 1, area.y + 2)
            .set_symbol("┳");
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
        buf.get_mut(area.x + player_panel_width - 1, area.y + area.height - 1)
            .set_symbol("┻");
    }
}
