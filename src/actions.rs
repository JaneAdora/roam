use anyhow::Result;
use base64::Engine;
use std::path::Path;

pub const OSC52_RAW_CAP: usize = 4096;

#[derive(Debug)]
pub enum RunOutcome {
    Quit,
    PrintAndExit(String),
}

pub fn osc52_encode(s: &str) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(s);
    format!("\x1b]52;c;{b64}\x07")
}

pub enum CopyResult {
    Full,
    Truncated { sent: usize, total: usize },
}

pub fn copy_to_clipboard(s: &str) -> Result<CopyResult> {
    use std::io::Write;
    let total = s.len();
    let (payload, outcome) = if total > OSC52_RAW_CAP {
        // Back off to a UTF-8 char boundary so slicing can't panic mid-codepoint
        // (a multibyte char straddling byte OSC52_RAW_CAP would otherwise crash,
        // corrupting the terminal while the TUI owns the alt-screen).
        let mut end = OSC52_RAW_CAP;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        (&s[..end], CopyResult::Truncated { sent: end, total })
    } else {
        (s, CopyResult::Full)
    };
    let seq = osc52_encode(payload);
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(seq.as_bytes())?;
    stdout.flush()?;
    Ok(outcome)
}

fn quote_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    if s.chars().all(|c| {
        c.is_alphanumeric() || matches!(c, '/' | '_' | '-' | '.' | '+' | '~' | ',')
    }) {
        s.into_owned()
    } else {
        let escaped = s.replace('\'', "'\\''");
        format!("'{escaped}'")
    }
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
        let out = osc52_encode("hi");
        assert!(out.starts_with("\x1b]52;c;"));
        assert!(out.ends_with('\x07'));
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
