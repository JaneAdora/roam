use std::path::Path;

use suite_term::clipboard::emit_osc52;
use suite_term::quote::quote_path;

#[derive(Debug)]
pub enum RunOutcome {
    Quit,
    PrintAndExit(String),
}

pub use suite_term::clipboard::Osc52;

/// Emit the OSC 52 clipboard sequence for `s` (capped at a char boundary).
/// Returns an `Osc52` whose `.truncated()` / `.sent_bytes` / `.total_bytes`
/// describe what was actually copied.
pub fn copy_to_clipboard(s: &str) -> Osc52 {
    emit_osc52(s)
}

pub fn cd_command(path: &Path) -> String {
    format!("cd {}", quote_path(path))
}

pub fn claude_command(path: &Path, dangerous: bool) -> String {
    let flag = if dangerous {
        " --dangerously-skip-permissions"
    } else {
        ""
    };
    format!("cd {} && claude{flag}", quote_path(path))
}

pub fn editor_command(path: &Path) -> String {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    format!("{editor} {}", quote_path(path))
}

pub fn shell_command(path: &Path) -> String {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    format!("cd {} && exec {shell}", quote_path(path))
}

pub fn tmux_new_window(path: &Path) -> String {
    format!("tmux new-window -c {}", quote_path(path))
}

pub fn in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn osc52_envelope() {
        let out = suite_term::clipboard::osc52_sequence("hi").sequence;
        assert!(out.starts_with("\x1b]52;c;"));
        assert!(out.ends_with('\x07'));
    }

    #[test]
    fn osc52_truncation_reports_exact_sent_bytes() {
        let mut s = "a".repeat(suite_term::clipboard::OSC52_CAP - 1);
        s.push('\u{e9}'); // 2-byte char straddles the cap boundary
        let r = suite_term::clipboard::osc52_sequence(&s);
        assert!(r.truncated());
        assert_eq!(r.sent_bytes, suite_term::clipboard::OSC52_CAP - 1);
        assert_eq!(r.total_bytes, suite_term::clipboard::OSC52_CAP + 1);
    }

    #[test]
    fn quote_simple_path_unquoted() {
        assert_eq!(quote_path(&PathBuf::from("/tmp/projects")), "/tmp/projects");
    }

    #[test]
    fn quote_path_with_space_is_quoted() {
        assert_eq!(
            quote_path(&PathBuf::from("/path with space/x")),
            "'/path with space/x'"
        );
    }

    #[test]
    fn quote_path_with_apostrophe_escapes() {
        let q = quote_path(&PathBuf::from("/a'b/c"));
        assert_eq!(q, "'/a'\\''b/c'");
    }

    #[test]
    fn cd_command_simple() {
        assert_eq!(cd_command(&PathBuf::from("/x")), "cd /x");
    }

    #[test]
    fn claude_command_dangerous() {
        assert_eq!(
            claude_command(&PathBuf::from("/x"), true),
            "cd /x && claude --dangerously-skip-permissions"
        );
    }

    #[test]
    fn tmux_new_window_quotes() {
        assert_eq!(tmux_new_window(&PathBuf::from("/x")), "tmux new-window -c /x");
    }
}
