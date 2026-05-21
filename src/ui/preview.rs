use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, content: Option<&str>) {
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(theme::dim_footer());
    let body = match content {
        Some(text) => text.to_string(),
        None => "(no preview)".to_string(),
    };
    let p = Paragraph::new(body).block(block).style(theme::preview_text());
    f.render_widget(p, area);
}

pub fn render_modal(f: &mut Frame, area: Rect, title: &str, content: &str, scroll: u16) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            format!(" {title} "),
            theme::pane_header_focused(),
        )))
        .borders(Borders::ALL)
        .border_style(theme::pane_header());
    f.render_widget(ratatui::widgets::Clear, area);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let p = Paragraph::new(content.to_string())
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .style(theme::preview_text());
    f.render_widget(p, inner);
}
