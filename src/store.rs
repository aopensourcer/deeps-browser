use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Session {
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub active: usize,
    #[serde(default = "default_window")]
    pub window: (f64, f64),
    /// Last outer position in logical pixels.
    #[serde(default)]
    pub position: Option<(f64, f64)>,
}

fn default_window() -> (f64, f64) {
    (1280.0, 820.0)
}

fn base_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

fn new_path() -> Option<PathBuf> {
    Some(base_dir()?.join("Deeps").join("session.json"))
}

fn old_path() -> Option<PathBuf> {
    Some(base_dir()?.join("Deepy").join("session.json"))
}

pub fn load() -> Option<Session> {
    let text = std::fs::read_to_string(new_path()?)
        .ok()
        .or_else(|| std::fs::read_to_string(old_path()?).ok())?;
    serde_json::from_str(&text).ok()
}

pub fn save(session: &Session) {
    let Some(file) = new_path() else {
        return;
    };
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(session) {
        let temp = file.with_extension("tmp");
        if std::fs::write(&temp, json).is_ok() {
            let _ = std::fs::remove_file(&file);
            let _ = std::fs::rename(&temp, &file);
        }
    }
}
