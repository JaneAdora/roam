use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(f: &mut Frame, area: Rect, content: Option<&str>, markdown: bool) {
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(theme::dim_footer());
    let p = match content {
        Some(text) if markdown => {
            Paragraph::new(crate::ui::markdown::to_lines(text)).block(block)
        }
        Some(text) => Paragraph::new(text.to_string())
            .block(block)
            .style(theme::preview_text()),
        None => Paragraph::new("(no preview)".to_string())
            .block(block)
            .style(theme::dim_footer()),
    };
    f.render_widget(p.wrap(Wrap { trim: false }), area);
}

pub fn render_modal(
    f: &mut Frame,
    area: Rect,
    title: &str,
    content: &str,
    scroll: u16,
    markdown: bool,
) {
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
    let p = if markdown {
        Paragraph::new(crate::ui::markdown::to_lines(content))
    } else {
        Paragraph::new(content.to_string()).style(theme::preview_text())
    };
    f.render_widget(p.wrap(Wrap { trim: false }).scroll((scroll, 0)), inner);
}
