use crate::ui::pinned::Bookmark;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct File {
    #[serde(default)]
    bookmark: Vec<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    key: String,
    label: String,
    path: String,
}

pub struct Loaded {
    pub display: Vec<Bookmark>,
    pub paths: std::collections::HashMap<char, PathBuf>,
}

pub fn load() -> Loaded {
    let path = match dirs::config_dir() {
        Some(d) => d.join("roam").join("bookmarks.toml"),
        None => return Loaded {
            display: Vec::new(),
            paths: Default::default(),
        },
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Loaded {
            display: Vec::new(),
            paths: Default::default(),
        },
    };
    let parsed: File = toml::from_str(&content).unwrap_or_default();
    let mut display = Vec::new();
    let mut paths = std::collections::HashMap::new();
    for raw in parsed.bookmark {
        if let Some(c) = raw.key.chars().next() {
            paths.insert(c, expand(&raw.path));
            display.push(Bookmark {
                key: c,
                label: raw.label,
            });
        }
    }
    Loaded { display, paths }
}

fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(p)
}

/// Lowest unused single-digit slot (1-9), or None when all nine are taken.
pub fn next_free_key(taken: &std::collections::HashMap<char, PathBuf>) -> Option<char> {
    ('1'..='9').find(|c| !taken.contains_key(c))
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Append one `[[bookmark]]` block to existing TOML text. Pure: preserves prior
/// content (and any comments) and escapes the label/path for a TOML basic string.
fn append_entry(existing: &str, key: char, label: &str, path: &str) -> String {
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!(
        "\n[[bookmark]]\nkey = \"{}\"\nlabel = \"{}\"\npath = \"{}\"\n",
        key,
        escape(label),
        escape(path)
    ));
    out
}

/// Persist a new bookmark to `~/.config/roam/bookmarks.toml`, creating the file
/// (and its directory) if needed and preserving any existing entries.
pub fn add(key: char, label: &str, path: &std::path::Path) -> std::io::Result<()> {
    let dir = dirs::config_dir()
        .map(|d| d.join("roam"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no config dir"))?;
    std::fs::create_dir_all(&dir)?;
    let file = dir.join("bookmarks.toml");
    let existing = std::fs::read_to_string(&file).unwrap_or_default();
    let updated = append_entry(&existing, key, label, &path.to_string_lossy());
    std::fs::write(&file, updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn next_free_key_skips_taken() {
        let mut taken = HashMap::new();
        taken.insert('1', PathBuf::from("/a"));
        taken.insert('2', PathBuf::from("/b"));
        assert_eq!(next_free_key(&taken), Some('3'));
    }

    #[test]
    fn next_free_key_none_when_full() {
        let mut taken = HashMap::new();
        for c in '1'..='9' {
            taken.insert(c, PathBuf::from("/x"));
        }
        assert_eq!(next_free_key(&taken), None);
    }

    #[test]
    fn append_entry_parses_back_via_loader() {
        let toml = append_entry("", '3', "my proj", "/home/jane/projects");
        let parsed: File = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.bookmark.len(), 1);
        assert_eq!(parsed.bookmark[0].key, "3");
        assert_eq!(parsed.bookmark[0].label, "my proj");
        assert_eq!(parsed.bookmark[0].path, "/home/jane/projects");
    }

    #[test]
    fn append_entry_preserves_existing_and_escapes_quotes() {
        let existing = "[[bookmark]]\nkey = \"1\"\nlabel = \"a\"\npath = \"/a\"\n";
        let toml = append_entry(existing, '2', "wei\"rd", "/p");
        let parsed: File = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.bookmark.len(), 2);
        assert_eq!(parsed.bookmark[1].label, "wei\"rd");
        assert_eq!(parsed.bookmark[1].key, "2");
    }
}
