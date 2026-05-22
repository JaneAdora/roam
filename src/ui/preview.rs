use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

/// Markdown -> styled markdown; recognized code/markup -> syntax highlight;
/// anything else -> None (caller renders plain text).
fn styled_lines(content: &str, name: &str) -> Option<Vec<Line<'static>>> {
    if crate::ui::markdown::is_markdown(name) {
        return Some(crate::ui::markdown::to_lines(content));
    }
    crate::ui::highlight::language_for(name)
        .map(|lang| crate::ui::highlight::to_lines(content, lang))
}

/// The styled block for the right-hand preview pane: a single left rule, a dim
/// filename title, and a little inner padding so content stops hugging the border
/// and the top. Shared by the text and image panes so they read alike.
pub fn pane_block(name: Option<&str>) -> Block<'static> {
    let mut block = Block::default()
        .borders(Borders::LEFT)
        .border_style(theme::dim_footer())
        .padding(Padding::new(1, 1, 1, 0));
    if let Some(n) = name {
        block = block.title(Line::from(Span::styled(
            format!(" {n} "),
            theme::pane_header(),
        )));
    }
    block
}

pub fn render(f: &mut Frame, area: Rect, content: Option<&str>, name: Option<&str>) {
    let block = pane_block(name);
    let p = match content {
        Some(text) => match name.and_then(|n| styled_lines(text, n)) {
            Some(lines) => Paragraph::new(lines).block(block),
            None => Paragraph::new(text.to_string())
                .block(block)
                .style(theme::preview_text()),
        },
        None => Paragraph::new("(no preview)".to_string())
            .block(block)
            .style(theme::dim_footer()),
    };
    f.render_widget(p.wrap(Wrap { trim: false }), area);
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
    let p = match styled_lines(content, title) {
        Some(lines) => Paragraph::new(lines),
        None => Paragraph::new(content.to_string()).style(theme::preview_text()),
    };
    f.render_widget(p.wrap(Wrap { trim: false }).scroll((scroll, 0)), inner);
}
