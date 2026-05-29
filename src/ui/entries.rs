use crate::fs::{human_mtime, human_size};
use crate::model::Entry;
use crate::ui::icons::{self, IconStyle};
use crate::ui::layout::Columns;
use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

pub fn render(
    f: &mut Frame,
    area: Rect,
    entries: &[Entry],
    selected: usize,
    cols: Columns,
    loading: bool,
    icon_style: IconStyle,
) {
    if loading && entries.is_empty() {
        let p = ratatui::widgets::Paragraph::new(Line::from(Span::styled(
            "loading…",
            theme::dim_footer(),
        )));
        f.render_widget(p, area);
        return;
    }

    if entries.is_empty() {
        let p = ratatui::widgets::Paragraph::new(Line::from(Span::styled(
            "(empty)",
            theme::dim_footer(),
        )));
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| make_item(e, i == selected, cols, icon_style))
        .collect();

    let list = List::new(items);
    let mut state = ListState::default();
    state.select(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn make_item<'a>(entry: &'a Entry, focused: bool, cols: Columns, icon_style: IconStyle) -> ListItem<'a> {
    let marker = if focused {
        theme::FOCUS_MARKER
    } else {
        theme::UNFOCUSED_PREFIX
    };
    let icon = icons::icon_for(entry, icon_style, cols.compact_icons);
    let name = truncate(&entry.display_name(), cols.name_max);

    let mut spans = vec![
        Span::raw(marker.to_string()),
        Span::raw(icon.to_string()),
    ];

    let name_style = name_style(entry, focused);
    spans.push(Span::styled(name, name_style));

    if let Some(target) = &entry.symlink_target {
        let arrow = if entry.is_broken_symlink() {
            format!(" → [broken: {}]", target.to_string_lossy())
        } else {
            format!(" → {}", target.to_string_lossy())
        };
        spans.push(Span::styled(
            arrow,
            if entry.is_broken_symlink() {
                theme::broken_style()
            } else {
                theme::dim_footer()
            },
        ));
    }

    if cols.show_size {
        if let Some(s) = entry.size {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("{:>6}", human_size(s)),
                theme::dim_footer(),
            ));
        }
    }

    if cols.show_mtime {
        if let Some(m) = entry.mtime {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("{:>4}", human_mtime(m)),
                theme::dim_footer(),
            ));
        }
    }

    ListItem::new(Line::from(spans))
}

fn name_style(entry: &Entry, focused: bool) -> Style {
    if focused {
        theme::active_row()
    } else if entry.is_broken_symlink() {
        theme::broken_style()
    } else if entry.is_hidden {
        theme::hidden_style()
    } else if entry.is_dir_like() {
        theme::dir_style()
    } else {
        Style::default()
    }
}

fn truncate(s: &str, max: u16) -> String {
    let max = max as usize;
    let len = s.chars().count();
    if len <= max {
        return s.to_string();
    }
    // Relative paths (recursive-find results) keep the tail so the basename stays
    // visible; plain names keep the head.
    if s.contains('/') {
        let take = max.saturating_sub(1);
        let mut out = String::from("…");
        out.extend(s.chars().skip(len - take));
        out
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_plain_name_keeps_head() {
        let t = truncate("verylongfilename.txt", 8);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() <= 8);
    }

    #[test]
    fn truncate_path_keeps_basename_tail() {
        let t = truncate("aaa/bbb/ccc/the_file.txt", 12);
        assert!(t.starts_with('…'));
        assert!(t.ends_with(".txt"));
        assert!(t.chars().count() <= 12);
    }
}
