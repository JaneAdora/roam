use crate::ui::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;

pub struct ActionMenu {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub selected: usize,
}

#[derive(Clone)]
pub struct MenuItem {
    pub key: Option<char>,
    pub label: String,
    pub action: MenuAction,
}

#[derive(Clone, Debug)]
pub enum MenuAction {
    CdAndExit,
    LaunchClaude,
    LaunchClaudeDanger,
    NewShellHere,
    NewTerminalTab,
    CopyPath,
    OpenEditor,
    CopyContents,
    Preview,
}

pub fn render_action_menu(f: &mut Frame, area: Rect, menu: &ActionMenu) {
    let items: Vec<ListItem> = menu
        .items
        .iter()
        .map(|m| {
            let key = m
                .key
                .map(|c| format!(" {c} "))
                .unwrap_or_else(|| "   ".to_string());
            ListItem::new(Line::from(vec![
                Span::styled(key, theme::pane_header_focused()),
                Span::raw(" "),
                Span::raw(m.label.clone()),
            ]))
        })
        .collect();

    let block = Block::default()
        .title(Line::from(Span::styled(
            format!(" {} ", menu.title),
            theme::pane_header_focused(),
        )))
        .borders(Borders::ALL)
        .border_style(theme::pane_header());

    f.render_widget(Clear, area);
    let list = List::new(items)
        .block(block)
        .highlight_style(theme::active_row())
        .highlight_symbol(theme::FOCUS_MARKER);
    let mut state = ListState::default();
    state.select(Some(menu.selected));
    f.render_stateful_widget(list, area, &mut state);
}

pub fn render_help(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            " help ",
            theme::pane_header_focused(),
        )))
        .borders(Borders::ALL)
        .border_style(theme::pane_header());
    f.render_widget(Clear, area);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = HELP_TEXT
        .lines()
        .map(|l| Line::from(Span::raw(l.to_string())))
        .collect();
    f.render_widget(ratatui::widgets::Paragraph::new(lines), inner);
}

const HELP_TEXT: &str = "\
NAVIGATION
  j/k or ↓↑    move          h/l or ←→   parent / enter
  g/G          top / bottom  1-9         jump bookmark
  Tab          swap focus

ACTIONS (Enter opens menu)
  o   cd-and-exit             c   launch claude
  D   claude danger (2-step)  s   new shell here
  t   new terminal tab        y   copy path
  e   open in $EDITOR         Y   copy contents (4 KiB cap)

VIEW
  .   toggle hidden           p   toggle preview pane
  r   refresh                 B   bookmark current

SEARCH
  /   filter current dir      R   recursive (depth 3)
  Esc clear / cancel

META
  ?   this help               q   quit  (Ctrl-C)
";
