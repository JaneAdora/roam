use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::path::Path;

pub fn render(f: &mut Frame, area: Rect, cwd: &Path, transient: Option<&str>) {
    let path = cwd.to_string_lossy();
    let title = Line::from(vec![
        Span::styled("roam ", theme::pane_header_focused()),
        Span::styled(path.into_owned(), theme::pane_header()),
    ]);

    let mut lines = vec![title];
    if let Some(msg) = transient {
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            theme::status_line(),
        )));
    }

    f.render_widget(Paragraph::new(lines), area);
}
