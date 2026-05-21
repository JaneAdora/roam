//! Tiny, dependency-free markdown styler for the preview pane and modal. This is
//! NOT a full CommonMark parser: it handles the common block forms (ATX
//! headings, fenced code, blockquotes, lists, horizontal rules) and inline
//! emphasis/code, which is enough to make `.md` files easier to read in the TUI.
use crate::ui::theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// True for filenames/relative paths that look like markdown.
pub fn is_markdown(name: &str) -> bool {
    let l = name.to_lowercase();
    l.ends_with(".md") || l.ends_with(".markdown") || l.ends_with(".mdown") || l.ends_with(".mkd")
}

fn base() -> Style {
    theme::preview_text()
}
fn code() -> Style {
    Style::default().fg(theme::LAVENDER)
}
fn dim() -> Style {
    Style::default().fg(theme::LAVENDER).add_modifier(Modifier::DIM)
}

/// Render markdown text into styled lines.
pub fn to_lines(text: &str) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    let mut in_fence = false;
    for raw in text.lines() {
        let trimmed = raw.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            out.push(Line::from(Span::styled(raw.to_string(), dim())));
            continue;
        }
        if in_fence {
            out.push(Line::from(Span::styled(raw.to_string(), code())));
            continue;
        }
        if let Some((level, htext)) = heading(trimmed) {
            let style = match level {
                1 => Style::default()
                    .fg(theme::MAGENTA)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                2 => Style::default()
                    .fg(theme::LAVENDER)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                _ => Style::default().fg(theme::LAVENDER).add_modifier(Modifier::BOLD),
            };
            out.push(Line::from(Span::styled(htext.to_string(), style)));
            continue;
        }
        if is_hr(trimmed) {
            out.push(Line::from(Span::styled("────────────────────".to_string(), dim())));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('>') {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            let mut spans = vec![Span::styled("│ ".to_string(), dim())];
            spans.extend(inline(rest, base().add_modifier(Modifier::ITALIC | Modifier::DIM)));
            out.push(Line::from(spans));
            continue;
        }
        if let Some((indent, marker, rest)) = list_item(raw) {
            let mut spans = vec![
                Span::raw(indent.to_string()),
                Span::styled(format!("{marker} "), Style::default().fg(theme::MAGENTA)),
            ];
            spans.extend(inline(rest, base()));
            out.push(Line::from(spans));
            continue;
        }
        out.push(Line::from(inline(raw, base())));
    }
    out
}

fn heading(t: &str) -> Option<(usize, &str)> {
    if !t.starts_with('#') {
        return None;
    }
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &t[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some((hashes, rest.trim_start()))
}

fn is_hr(t: &str) -> bool {
    let t = t.trim();
    t.len() >= 3 && ['-', '*', '_'].iter().any(|&ch| t.chars().all(|c| c == ch))
}

fn list_item(raw: &str) -> Option<(&str, String, &str)> {
    let indent_len = raw.len() - raw.trim_start().len();
    let (indent, body) = raw.split_at(indent_len);
    for m in ['-', '*', '+'] {
        if let Some(rest) = body.strip_prefix(m).and_then(|r| r.strip_prefix(' ')) {
            return Some((indent, "•".to_string(), rest));
        }
    }
    let digits: String = body.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        if let Some(rest) = body[digits.len()..].strip_prefix(". ") {
            return Some((indent, format!("{digits}."), rest));
        }
    }
    None
}

/// Inline emphasis: `code`, **bold**, *italic* / _italic_. Falls back to literal
/// text when a marker is unclosed; `_` only emphasizes at word boundaries to
/// avoid snake_case false positives.
fn inline(text: &str, base: Style) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            if let Some(end) = (i + 1..chars.len()).find(|&j| chars[j] == '`') {
                flush(&mut spans, &mut buf, base);
                spans.push(Span::styled(chars[i + 1..end].iter().collect::<String>(), code()));
                i = end + 1;
                continue;
            }
        }
        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if let Some(end) = find_double_star(&chars, i + 2) {
                flush(&mut spans, &mut buf, base);
                spans.push(Span::styled(
                    chars[i + 2..end].iter().collect::<String>(),
                    base.add_modifier(Modifier::BOLD),
                ));
                i = end + 2;
                continue;
            }
        }
        if (c == '*' || c == '_') && (c == '*' || i == 0 || chars[i - 1].is_whitespace()) {
            if let Some(end) = (i + 1..chars.len()).find(|&j| chars[j] == c) {
                if end > i + 1 {
                    flush(&mut spans, &mut buf, base);
                    spans.push(Span::styled(
                        chars[i + 1..end].iter().collect::<String>(),
                        base.add_modifier(Modifier::ITALIC),
                    ));
                    i = end + 1;
                    continue;
                }
            }
        }
        buf.push(c);
        i += 1;
    }
    flush(&mut spans, &mut buf, base);
    if spans.is_empty() {
        spans.push(Span::styled(String::new(), base));
    }
    spans
}

fn flush(spans: &mut Vec<Span<'static>>, buf: &mut String, base: Style) {
    if !buf.is_empty() {
        spans.push(Span::styled(std::mem::take(buf), base));
    }
}

fn find_double_star(chars: &[char], from: usize) -> Option<usize> {
    (from..chars.len().saturating_sub(1)).find(|&j| chars[j] == '*' && chars[j + 1] == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_md_extensions() {
        assert!(is_markdown("README.md"));
        assert!(is_markdown("notes.MARKDOWN"));
        assert!(!is_markdown("main.rs"));
    }

    #[test]
    fn heading_strips_hashes() {
        let lines = to_lines("# Title\n");
        let h: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(h, "Title");
    }

    #[test]
    fn inline_splits_bold_and_code() {
        let lines = to_lines("some **bold** and `code` here\n");
        assert!(lines[0].spans.len() >= 4);
        assert!(lines[0].spans.iter().any(|s| s.content.as_ref() == "bold"));
        assert!(lines[0].spans.iter().any(|s| s.content.as_ref() == "code"));
    }

    #[test]
    fn list_bullet_and_fence() {
        let lines = to_lines("- item\n```\nlet x=1;\n```\n");
        assert!(lines.iter().any(|l| l.spans.iter().any(|s| s.content.contains('•'))));
        assert!(lines.iter().any(|l| l.spans.iter().any(|s| s.content.contains("let x=1;"))));
    }

    #[test]
    fn snake_case_not_italicized() {
        let lines = to_lines("call file_name_here now\n");
        // single span -> no emphasis split happened
        assert_eq!(lines[0].spans.len(), 1);
    }
}
