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
