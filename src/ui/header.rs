use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::path::Path;

pub fn render(f: &mut Frame, area: Rect, cwd: &Path, transient: Option<&str>, find: Option<(&str, usize)>) {
    let path = cwd.to_string_lossy();
    let mut title_spans = vec![
        Span::styled("roam ", theme::pane_header_focused()),
        Span::styled(path.into_owned(), theme::pane_header()),
    ];
    if let Some((q, n)) = find {
        title_spans.push(Span::styled(
            format!("  [find '{q}': {n}  Esc to clear]"),
            theme::status_line(),
        ));
    }
    let title = Line::from(title_spans);

    let mut lines = vec![title];
    if let Some(msg) = transient {
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            theme::status_line(),
        )));
    }

    f.render_widget(Paragraph::new(lines), area);
}
