use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct Bookmark {
    pub key: char,
    pub label: String,
}

/// Render the one-line pinned bookmarks bar. When `selected` is `Some(i)` the
/// pane is focused and bookmark `i` is highlighted (reversed).
pub fn render(f: &mut Frame, area: Rect, bookmarks: &[Bookmark], selected: Option<usize>) {
    if bookmarks.is_empty() {
        return;
    }
    let mut spans: Vec<Span> = Vec::new();
    for (i, bm) in bookmarks.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let focused = selected == Some(i);
        let key_style = if focused {
            theme::pane_header_focused().add_modifier(Modifier::REVERSED)
        } else {
            theme::pane_header_focused()
        };
        let label_style = if focused {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        spans.push(Span::styled(format!("[{}]", bm.key), key_style));
        spans.push(Span::styled(format!(" {}", bm.label), label_style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
