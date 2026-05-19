mod actions;
mod app;
mod bookmarks;
mod config;
mod fs;
mod model;
mod preview;
mod ui;

use anyhow::Result;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
roam :: mobile-friendly file browser

USAGE:
  roam [PATH]      Open at PATH, $ROAM_ROOT, or $HOME (in that order).
  roam --resume    Open at the last-visited dir (persisted across runs).
  roam --help      Print this message.
  roam --version   Print version.

ENVIRONMENT:
  ROAM_ROOT        Default starting directory (used when no PATH arg).

KEYS: Enter opens a menu of actions on the focused entry. Press ? inside roam for the full keymap.
";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut want_resume = false;
    let mut path_arg: Option<PathBuf> = None;
    for a in args {
        match a.as_str() {
            "--help" | "-h" => {
                print!("{HELP}");
                return Ok(());
            }
            "--version" | "-V" => {
                println!("roam {VERSION}");
                return Ok(());
            }
            "--resume" => want_resume = true,
            other if other.starts_with("--") => {
                eprintln!("roam: unknown flag: {other}\n\nTry: roam --help");
                std::process::exit(2);
            }
            other => path_arg = Some(PathBuf::from(other)),
        }
    }

    let persisted = config::load();
    let bookmarks = bookmarks::load();
    let start = resolve_start(path_arg, want_resume, &persisted);

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::terminal::SetTitle("roam"),
    )?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut state = app::AppState::new(start, persisted, bookmarks)?;

    let result = app::run(&mut terminal, &mut state);

    let snapshot = config::State {
        last_dir: Some(state.cwd.clone()),
        show_hidden: state.show_hidden,
        preview_enabled: state.preview_enabled,
    };
    let _ = config::save(&snapshot);

    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    match result? {
        actions::RunOutcome::Quit => Ok(()),
        actions::RunOutcome::PrintAndExit(cmd) => {
            println!("{cmd}");
            Ok(())
        }
    }
}

fn resolve_start(
    arg: Option<PathBuf>,
    want_resume: bool,
    persisted: &config::State,
) -> PathBuf {
    if let Some(p) = arg {
        return p;
    }
    if want_resume {
        if let Some(p) = persisted.last_dir.clone() {
            if p.is_dir() {
                return p;
            }
        }
    }
    if let Ok(root) = std::env::var("ROAM_ROOT") {
        let p = PathBuf::from(root);
        if p.is_dir() {
            return p;
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
}
