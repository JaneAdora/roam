use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct Bookmark {
    pub key: char,
    pub label: String,
}

pub fn render(f: &mut Frame, area: Rect, bookmarks: &[Bookmark]) {
    if bookmarks.is_empty() {
        return;
    }
    let mut spans: Vec<Span> = Vec::new();
    for (i, bm) in bookmarks.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!("[{}]", bm.key),
            theme::pane_header_focused(),
        ));
        spans.push(Span::raw(format!(" {}", bm.label)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
