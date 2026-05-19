use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub last_dir: Option<PathBuf>,
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default = "default_preview")]
    pub preview_enabled: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_dir: None,
            show_hidden: false,
            preview_enabled: true,
        }
    }
}

fn default_preview() -> bool {
    true
}

fn state_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("roam").join("state.json"))
}

pub fn load() -> State {
    let Some(p) = state_path() else {
        return State::default();
    };
    let Ok(s) = std::fs::read_to_string(&p) else {
        return State::default();
    };
    serde_json::from_str(&s).unwrap_or_default()
}

pub fn save(state: &State) -> Result<()> {
    let Some(p) = state_path() else {
        return Ok(());
    };
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let s = serde_json::to_string_pretty(state)?;
    std::fs::write(p, s)?;
    Ok(())
}
