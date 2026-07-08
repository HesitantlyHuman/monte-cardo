use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

use crate::{cards::Card, settings::GameMode};

const MENU_TITLE: &[&str] = &[
    "┏┳┓┏━┓┏┓╻╺┳╸┏━╸   ┏━╸┏━┓┏━┓╺┳┓┏━┓",
    "┃┃┃┃ ┃┃┗┫ ┃ ┣╸    ┃  ┣━┫┣┳┛ ┃┃┃ ┃",
    "╹ ╹┗━┛╹ ╹ ╹ ┗━╸   ┗━╸╹ ╹╹┗╸╺┻┛┗━┛",
];

const TITLE_WIDTH: u16 = 40;
const MENU_CARDS_WIDTH: u16 = 18;
const MENU_CARDS_HEIGHT: u16 = 6;
const TITLE_CARD_GAP: u16 = 3;

const MENU_CONTENT_WIDTH: u16 = TITLE_WIDTH + TITLE_CARD_GAP + MENU_CARDS_WIDTH;
const MENU_BUTTON_WIDTH: u16 = 28;

const MENU_CARD_SPACING: u16 = 4;
const MENU_CARD_RAISE: u16 = 1;

pub struct MainMenu {
    selected_index: usize,
}

impl MainMenu {
    pub fn new(selected_index: usize) -> Self {
        Self { selected_index }
    }
}

impl Widget for MainMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 8 || area.height < 8 {
            return;
        }

        Block::new()
            .title(" Monte Cardo ")
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .render(area, buf);

        let inner = inner_rect(area, 2, 1);

        if inner.height < 10 {
            Paragraph::new("Terminal is too small for the menu.")
                .alignment(Alignment::Center)
                .render(inner, buf);
            return;
        }

        let title_area = Rect::new(inner.x, inner.y + 1, inner.width, 6);
        render_title_and_cards(title_area, buf);

        let menu_y = title_area.y + title_area.height + 1;
        let menu_height = 5.min(inner.height.saturating_sub(8));
        let menu_area = Rect::new(inner.x, menu_y, inner.width, menu_height);
        render_menu_options(menu_area, buf, self.selected_index);

        let controls_y = inner.y + inner.height.saturating_sub(3);
        render_controls(Rect::new(inner.x, controls_y, inner.width, 1), buf);

        let credit_y = inner.y + inner.height.saturating_sub(1);
        render_credit(Rect::new(inner.x, credit_y, inner.width, 1), buf);
    }
}

fn render_title_and_cards(area: Rect, buf: &mut Buffer) {
    if area.width >= MENU_CONTENT_WIDTH {
        let start_x = content_start_x(area);

        let title_rect = Rect::new(start_x, area.y + 1, TITLE_WIDTH, MENU_TITLE.len() as u16);

        let cards_rect = Rect::new(
            start_x + TITLE_WIDTH + TITLE_CARD_GAP,
            area.y,
            MENU_CARDS_WIDTH,
            MENU_CARDS_HEIGHT,
        );

        Paragraph::new(MENU_TITLE.join("\n"))
            .alignment(Alignment::Left)
            .render(title_rect, buf);

        render_menu_cards(cards_rect, buf);
    } else {
        Paragraph::new(MENU_TITLE.join("\n"))
            .alignment(Alignment::Center)
            .render(
                Rect::new(area.x, area.y + 1, area.width, MENU_TITLE.len() as u16),
                buf,
            );
    }
}
fn render_menu_cards(area: Rect, buf: &mut Buffer) {
    if area.width < 7 || area.height < 5 {
        return;
    }

    let render_width = MENU_CARD_SPACING * 3 + 3;

    let start_x = if area.width >= render_width {
        area.x + area.width.saturating_sub(render_width) / 2
    } else {
        area.x
    };

    let base_y = if area.height >= MENU_CARDS_HEIGHT {
        area.y + area.height.saturating_sub(MENU_CARDS_HEIGHT) / 2 + MENU_CARD_RAISE
    } else {
        area.y
    };

    let wild_x = start_x;
    let two_x = start_x + MENU_CARD_SPACING;
    let seven_x = start_x + MENU_CARD_SPACING * 2;

    let lower_y = base_y;
    let raised_y = base_y.saturating_sub(MENU_CARD_RAISE);

    Card::new(0, false).render(
        Rect::new(
            wild_x,
            lower_y,
            area.width.saturating_sub(wild_x - area.x),
            5,
        ),
        buf,
    );

    // The 2 is raised, but not topmost.
    Card::new(2, false).render(
        Rect::new(
            two_x,
            raised_y,
            area.width.saturating_sub(two_x - area.x),
            5,
        ),
        buf,
    );

    // Render 7 last so it sits visually on top of the 2.
    Card::new(7, false).render(
        Rect::new(
            seven_x,
            lower_y,
            area.width.saturating_sub(seven_x - area.x),
            5,
        ),
        buf,
    );
}

fn render_menu_options(area: Rect, buf: &mut Buffer, selected_index: usize) {
    if area.height == 0 {
        return;
    }

    let options = [GameMode::PlayComputers, GameMode::PlayLive];

    let button_width = MENU_BUTTON_WIDTH.min(area.width);

    let button_x = if area.width >= MENU_CONTENT_WIDTH {
        content_start_x(area)
    } else {
        area.x + area.width.saturating_sub(button_width) / 2
    };

    for (index, mode) in options.into_iter().enumerate() {
        let y = area.y + index as u16 * 2;

        if y >= area.y + area.height {
            break;
        }

        let selected = index == selected_index;

        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let arrow = if selected { "➤" } else { " " };

        let line = Line::from(vec![
            Span::styled(format!("{:<2}", arrow), style),
            Span::styled(format!(" {:<24}", mode.label()), style),
        ]);

        Paragraph::new(line)
            .alignment(Alignment::Left)
            .render(Rect::new(button_x, y, button_width, 1), buf);
    }
}

fn render_controls(area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(Span::styled(
        "↑/↓ Select     Enter Continue     Ctrl+C Quit",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center)
    .render(area, buf);
}

fn render_credit(area: Rect, buf: &mut Buffer) {
    Paragraph::new(Line::from(Span::styled(
        "by Tanner Sims",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Right)
    .render(area, buf);
}

fn inner_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x + horizontal,
        y: area.y + vertical,
        width: area.width.saturating_sub(horizontal * 2),
        height: area.height.saturating_sub(vertical * 2),
    }
}

fn content_start_x(area: Rect) -> u16 {
    area.x + area.width.saturating_sub(MENU_CONTENT_WIDTH) / 2
}
