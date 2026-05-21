use crate::actions::{self, CopyResult, RunOutcome};
use crate::bookmarks;
use crate::config;
use crate::fs as roam_fs;
use crate::model::Entry;
use crate::preview;
use crate::ui::layout;
use crate::ui::modal::{ActionMenu, MenuAction, MenuItem};
use crate::ui::{self, entries as ui_entries, footer, header, modal, pinned, preview as ui_preview};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Terminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct AppState {
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub show_hidden: bool,
    pub preview_enabled: bool,
    pub preview_cache: Option<(usize, String)>,
    pub bookmarks: bookmarks::Loaded,
    pub transient: Option<(String, Instant)>,
    pub mode: InputMode,
    pub pending_danger: Option<Instant>,
}

pub enum InputMode {
    Normal,
    ActionMenu(ActionMenu),
    Help,
    PreviewModal {
        title: String,
        text: String,
        scroll: u16,
    },
    Search {
        query: String,
    },
}

impl AppState {
    pub fn new(cwd: PathBuf, persisted: config::State, bookmarks: bookmarks::Loaded) -> Result<Self> {
        let entries = roam_fs::list_dir(&cwd, persisted.show_hidden)?;
        Ok(Self {
            cwd,
            entries,
            selected: 0,
            show_hidden: persisted.show_hidden,
            preview_enabled: persisted.preview_enabled,
            preview_cache: None,
            bookmarks,
            transient: None,
            mode: InputMode::Normal,
            pending_danger: None,
        })
    }

    pub fn focused(&self) -> Option<&Entry> {
        self.entries.get(self.selected)
    }

    fn refresh_entries(&mut self) {
        self.preview_cache = None;
        match roam_fs::list_dir(&self.cwd, self.show_hidden) {
            Ok(v) => {
                self.entries = v;
                if self.selected >= self.entries.len() {
                    self.selected = self.entries.len().saturating_sub(1);
                }
            }
            Err(e) => self.toast(format!("error: {e}")),
        }
    }

    pub fn cd(&mut self, target: PathBuf) {
        self.cwd = target;
        self.selected = 0;
        self.preview_cache = None;
        self.refresh_entries();
    }

    pub fn parent(&mut self) {
        let prev = self.cwd.clone();
        if let Some(p) = roam_fs::parent_of(&self.cwd) {
            self.cwd = p;
            self.refresh_entries();
            if let Some(child_name) = prev.file_name() {
                if let Some(idx) = self
                    .entries
                    .iter()
                    .position(|e| e.name == *child_name)
                {
                    self.selected = idx;
                }
            }
        }
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.transient = Some((msg.into(), Instant::now()));
    }

    pub fn current_transient(&self) -> Option<&str> {
        let (msg, when) = self.transient.as_ref()?;
        if when.elapsed() < Duration::from_secs(3) {
            Some(msg.as_str())
        } else {
            None
        }
    }
}

pub fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut AppState,
) -> Result<RunOutcome> {
    loop {
        terminal.draw(|f| render(f, state))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Release {
                        if let Some(outcome) = handle_key(state, key)? {
                            return Ok(outcome);
                        }
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}

fn render(f: &mut ratatui::Frame, state: &mut AppState) {
    let area = f.area();
    let cols = layout::choose_columns(area.width);

    let pinned_height = if cols.show_pinned && !state.bookmarks.display.is_empty() {
        1
    } else {
        0
    };
    let transient_lines = if state.current_transient().is_some() { 1 } else { 0 };
    let footer_height = 1 + transient_lines;
    let header_height = 1 + transient_lines;

    let mut constraints = vec![Constraint::Length(header_height)];
    if pinned_height > 0 {
        constraints.push(Constraint::Length(pinned_height));
    }
    constraints.push(Constraint::Min(3));
    constraints.push(Constraint::Length(footer_height));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;
    header::render(f, chunks[idx], &state.cwd, state.current_transient());
    idx += 1;
    if pinned_height > 0 {
        pinned::render(f, chunks[idx], &state.bookmarks.display);
        idx += 1;
    }

    let body = chunks[idx];
    idx += 1;

    let loading = false;

    if cols.show_preview && state.preview_enabled {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(body);
        ui_entries::render(f, split[0], &state.entries, state.selected, cols, loading);
        let preview_text = focused_preview_text(state);
        ui_preview::render(f, split[1], preview_text.as_deref());
    } else {
        ui_entries::render(f, body, &state.entries, state.selected, cols, loading);
    }

    footer::render(f, chunks[idx], state.current_transient());

    match &state.mode {
        InputMode::Normal => {}
        InputMode::ActionMenu(menu) => {
            let rect = ui::centered_rect(area, 60, 60);
            modal::render_action_menu(f, rect, menu);
        }
        InputMode::Help => {
            let rect = ui::centered_rect(area, 80, 80);
            modal::render_help(f, rect);
        }
        InputMode::PreviewModal { title, text, scroll } => {
            let rect = ui::centered_rect(area, 100, 90);
            ui_preview::render_modal(f, rect, title, text, *scroll);
        }
        InputMode::Search { query } => {
            let rect = ui::centered_rect(area, 60, 20);
            let block = ratatui::widgets::Block::default()
                .title(" filter ")
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(ui::theme::pane_header());
            f.render_widget(ratatui::widgets::Clear, rect);
            let inner = block.inner(rect);
            f.render_widget(block, rect);
            let p = ratatui::widgets::Paragraph::new(format!("/ {query}"));
            f.render_widget(p, inner);
        }
    }
}

fn focused_preview_text(state: &mut AppState) -> Option<String> {
    let idx = state.selected;
    if let Some((cached_idx, ref text)) = state.preview_cache {
        if cached_idx == idx {
            return Some(text.clone());
        }
    }
    let entry = state.focused()?;
    let text = if entry.is_dir_like() {
        format!("(directory: {})", entry.display_name())
    } else {
        match preview::read(&entry.path) {
            preview::Preview::Text(s) => s,
            preview::Preview::Binary { size } => {
                format!("(binary, {})", roam_fs::human_size(size))
            }
            preview::Preview::Unreadable => "(unreadable)".to_string(),
        }
    };
    state.preview_cache = Some((idx, text.clone()));
    Some(text)
}

fn handle_key(state: &mut AppState, key: KeyEvent) -> Result<Option<RunOutcome>> {
    if matches!(state.mode, InputMode::ActionMenu(_)) {
        return handle_menu_key(state, key);
    }
    if matches!(state.mode, InputMode::Help) {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) {
            state.mode = InputMode::Normal;
        }
        return Ok(None);
    }
    if matches!(state.mode, InputMode::PreviewModal { .. }) {
        return handle_preview_modal_key(state, key);
    }
    if matches!(state.mode, InputMode::Search { .. }) {
        return handle_search_key(state, key);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return Ok(Some(RunOutcome::Quit));
    }

    match key.code {
        KeyCode::Char('q') => return Ok(Some(RunOutcome::Quit)),
        KeyCode::Char('?') => state.mode = InputMode::Help,

        KeyCode::Char('j') | KeyCode::Down => move_down(state),
        KeyCode::Char('k') | KeyCode::Up => move_up(state),
        KeyCode::Char('g') => state.selected = 0,
        KeyCode::Char('G') => state.selected = state.entries.len().saturating_sub(1),
        KeyCode::Char('h') | KeyCode::Left => state.parent(),
        KeyCode::Char('l') | KeyCode::Right => enter_dir(state),

        KeyCode::Char('r') => state.refresh_entries(),
        KeyCode::Char('.') => {
            state.show_hidden = !state.show_hidden;
            state.refresh_entries();
            state.toast(if state.show_hidden {
                "hidden: on"
            } else {
                "hidden: off"
            });
        }
        KeyCode::Char('p') => {
            state.preview_enabled = !state.preview_enabled;
            state.toast(if state.preview_enabled {
                "preview: on"
            } else {
                "preview: off"
            });
        }

        KeyCode::Char('/') => state.mode = InputMode::Search { query: String::new() },

        KeyCode::Char('o') => return Ok(action_cd_exit(state)),
        KeyCode::Char('c') => return Ok(action_claude(state, false)),
        KeyCode::Char('D') => return Ok(action_claude_danger(state)),
        KeyCode::Char('s') => return Ok(action_shell(state)),
        KeyCode::Char('t') => return Ok(action_new_tab(state)),
        KeyCode::Char('y') => action_copy_path(state),
        KeyCode::Char('e') => return Ok(action_edit(state)),
        KeyCode::Char('Y') => action_copy_contents(state),

        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            jump_bookmark(state, c);
        }

        KeyCode::Enter => open_action_menu(state),

        _ => {}
    }
    Ok(None)
}

fn handle_menu_key(state: &mut AppState, key: KeyEvent) -> Result<Option<RunOutcome>> {
    let InputMode::ActionMenu(menu) = &mut state.mode else {
        return Ok(None);
    };
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            state.mode = InputMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if menu.selected + 1 < menu.items.len() {
                menu.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            menu.selected = menu.selected.saturating_sub(1);
        }
        KeyCode::Enter => {
            let action = menu.items[menu.selected].action.clone();
            state.mode = InputMode::Normal;
            return Ok(fire_action(state, action));
        }
        KeyCode::Char(c) => {
            let key_match = menu.items.iter().position(|m| m.key == Some(c));
            if let Some(i) = key_match {
                let action = menu.items[i].action.clone();
                state.mode = InputMode::Normal;
                return Ok(fire_action(state, action));
            }
        }
        _ => {}
    }
    Ok(None)
}

fn handle_preview_modal_key(state: &mut AppState, key: KeyEvent) -> Result<Option<RunOutcome>> {
    let InputMode::PreviewModal { scroll, .. } = &mut state.mode else {
        return Ok(None);
    };
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => state.mode = InputMode::Normal,
        KeyCode::Char('j') | KeyCode::Down => *scroll = scroll.saturating_add(1),
        KeyCode::Char('k') | KeyCode::Up => *scroll = scroll.saturating_sub(1),
        KeyCode::PageDown | KeyCode::Char(' ') => *scroll = scroll.saturating_add(10),
        KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
        _ => {}
    }
    Ok(None)
}

fn handle_search_key(state: &mut AppState, key: KeyEvent) -> Result<Option<RunOutcome>> {
    let InputMode::Search { query } = &mut state.mode else {
        return Ok(None);
    };
    match key.code {
        KeyCode::Esc => state.mode = InputMode::Normal,
        KeyCode::Enter => {
            let q = query.clone();
            state.mode = InputMode::Normal;
            apply_filter(state, &q);
        }
        KeyCode::Backspace => {
            query.pop();
        }
        KeyCode::Char(c) => query.push(c),
        _ => {}
    }
    Ok(None)
}

fn apply_filter(state: &mut AppState, query: &str) {
    if query.is_empty() {
        return;
    }
    let q = query.to_lowercase();
    if let Some(idx) = state
        .entries
        .iter()
        .position(|e| e.display_name().to_lowercase().contains(&q))
    {
        state.selected = idx;
        state.toast(format!("found '{query}'"));
    } else {
        state.toast(format!("no match for '{query}'"));
    }
}

fn move_down(state: &mut AppState) {
    if state.entries.is_empty() {
        return;
    }
    state.selected = (state.selected + 1).min(state.entries.len() - 1);
    state.preview_cache = None;
}

fn move_up(state: &mut AppState) {
    state.selected = state.selected.saturating_sub(1);
    state.preview_cache = None;
}

fn enter_dir(state: &mut AppState) {
    let Some(entry) = state.focused() else { return };
    if !entry.is_dir_like() {
        return;
    }
    if entry.is_broken_symlink() {
        state.toast("broken symlink, can't navigate");
        return;
    }
    let target = entry.path.clone();
    state.cd(target);
}

fn jump_bookmark(state: &mut AppState, key: char) {
    if let Some(path) = state.bookmarks.paths.get(&key).cloned() {
        if path.is_dir() {
            state.cd(path);
        } else {
            state.toast(format!("bookmark {key}: path missing"));
        }
    }
}

fn open_action_menu(state: &mut AppState) {
    let Some(entry) = state.focused() else { return };
    if entry.is_dir_like() && !entry.is_broken_symlink() {
        let menu = ActionMenu {
            title: format!("dir: {}", entry.display_name()),
            selected: 0,
            items: vec![
                item(Some('o'), "cd-and-exit", MenuAction::CdAndExit),
                item(Some('c'), "launch claude", MenuAction::LaunchClaude),
                item(Some('s'), "new shell here", MenuAction::NewShellHere),
                item(Some('t'), "new terminal tab", MenuAction::NewTerminalTab),
                item(Some('y'), "copy path", MenuAction::CopyPath),
                item(Some('D'), "claude --danger (2-step)", MenuAction::LaunchClaudeDanger),
            ],
        };
        state.mode = InputMode::ActionMenu(menu);
    } else if entry.is_broken_symlink() {
        state.toast("broken symlink");
    } else {
        let menu = ActionMenu {
            title: format!("file: {}", entry.display_name()),
            selected: 0,
            items: vec![
                item(None, "preview", MenuAction::Preview),
                item(Some('e'), "open in $EDITOR", MenuAction::OpenEditor),
                item(Some('y'), "copy path", MenuAction::CopyPath),
                item(Some('Y'), "copy contents (4K cap)", MenuAction::CopyContents),
            ],
        };
        state.mode = InputMode::ActionMenu(menu);
    }
}

fn item(key: Option<char>, label: &str, action: MenuAction) -> MenuItem {
    MenuItem {
        key,
        label: label.to_string(),
        action,
    }
}

fn fire_action(state: &mut AppState, action: MenuAction) -> Option<RunOutcome> {
    match action {
        MenuAction::CdAndExit => action_cd_exit(state),
        MenuAction::LaunchClaude => action_claude(state, false),
        MenuAction::LaunchClaudeDanger => action_claude_danger(state),
        MenuAction::NewShellHere => action_shell(state),
        MenuAction::NewTerminalTab => action_new_tab(state),
        MenuAction::CopyPath => {
            action_copy_path(state);
            None
        }
        MenuAction::OpenEditor => action_edit(state),
        MenuAction::CopyContents => {
            action_copy_contents(state);
            None
        }
        MenuAction::Preview => {
            open_preview_modal(state);
            None
        }
    }
}

fn action_target(state: &AppState) -> Option<PathBuf> {
    state.focused().map(|e| e.path.clone())
}

fn action_cd_exit(state: &AppState) -> Option<RunOutcome> {
    let path = action_target(state)?;
    let target = resolve_target_dir(&path);
    exit_with(actions::cd_command(&target))
}

fn action_claude(state: &AppState, danger: bool) -> Option<RunOutcome> {
    let path = action_target(state)?;
    let target = resolve_target_dir(&path);
    exit_with(actions::claude_command(&target, danger))
}

fn action_claude_danger(state: &mut AppState) -> Option<RunOutcome> {
    let now = Instant::now();
    let armed = match state.pending_danger {
        Some(when) if now.duration_since(when) < Duration::from_secs(3) => true,
        _ => false,
    };
    if !armed {
        state.pending_danger = Some(now);
        state.toast("press D again to confirm DANGER");
        return None;
    }
    state.pending_danger = None;
    action_claude(state, true)
}

fn action_shell(state: &AppState) -> Option<RunOutcome> {
    let path = action_target(state)?;
    let target = resolve_target_dir(&path);
    exit_with(actions::shell_command(&target))
}

fn action_new_tab(state: &mut AppState) -> Option<RunOutcome> {
    let path = action_target(state)?;
    let target = resolve_target_dir(&path);
    if actions::in_tmux() {
        exit_with(actions::tmux_new_window(&target))
    } else {
        state.toast("no tmux: falling back to shell-in-place");
        exit_with(actions::shell_command(&target))
    }
}

fn action_copy_path(state: &mut AppState) {
    let Some(path) = action_target(state) else { return };
    let s = path.to_string_lossy().into_owned();
    match actions::copy_to_clipboard(&s) {
        Ok(CopyResult::Full) => state.toast(format!("copied: {s}")),
        Ok(CopyResult::Truncated { sent, total }) => {
            state.toast(format!("copied (truncated {sent}/{total}B)"))
        }
        Err(e) => state.toast(format!("copy failed: {e}")),
    }
}

fn action_edit(state: &AppState) -> Option<RunOutcome> {
    let entry = state.focused()?;
    if entry.is_dir_like() {
        return None;
    }
    exit_with(actions::editor_command(&entry.path))
}

fn action_copy_contents(state: &mut AppState) {
    let Some(entry) = state.focused() else { return };
    if entry.is_dir_like() {
        state.toast("not a file");
        return;
    }
    let path = entry.path.clone();
    let text = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            state.toast(format!("read failed: {e}"));
            return;
        }
    };
    match actions::copy_to_clipboard(&text) {
        Ok(CopyResult::Full) => state.toast(format!("copied {}B", text.len())),
        Ok(CopyResult::Truncated { sent, total }) => {
            state.toast(format!("copied (truncated {sent}/{total}B)"))
        }
        Err(e) => state.toast(format!("copy failed: {e}")),
    }
}

fn open_preview_modal(state: &mut AppState) {
    let Some(entry) = state.focused().cloned() else { return };
    if entry.is_dir_like() {
        return;
    }
    let title = entry.display_name();
    let text = match preview::read(&entry.path) {
        preview::Preview::Text(s) => s,
        preview::Preview::Binary { size } => format!("(binary file, {})", roam_fs::human_size(size)),
        preview::Preview::Unreadable => "(unreadable)".to_string(),
    };
    state.mode = InputMode::PreviewModal { title, text, scroll: 0 };
}

fn exit_with(cmd: String) -> Option<RunOutcome> {
    let _ = actions::copy_to_clipboard(&cmd);
    Some(RunOutcome::PrintAndExit(cmd))
}

fn resolve_target_dir(path: &Path) -> PathBuf {
    if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().map(PathBuf::from).unwrap_or_else(|| path.to_path_buf())
    }
}
