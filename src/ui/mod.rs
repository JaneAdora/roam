pub mod entries;
pub mod footer;
pub mod header;
pub mod layout;
pub mod highlight;
pub mod icons;
pub mod image;
pub mod markdown;
pub mod modal;
pub mod pinned;
pub mod preview;
pub mod theme;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn centered_rect(parent: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(parent);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(v[1])[1]
}
