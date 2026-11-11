//! Bookmark import from the browsers already installed on this PC.
//!
//! Chromium-based browsers keep their bookmarks in a plain JSON file, so no
//! extra dependency is needed. Firefox stores them in SQLite; we simply report
//! the sources we could read instead of pretending to support it.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

use crate::data::Data;

#[derive(Serialize, Clone)]
pub struct ImportSource {
    pub name: String,
    pub found: usize,
    pub added: usize,
}

#[derive(Serialize, Clone)]
pub struct ImportReport {
    pub added: usize,
    pub skipped: usize,
    pub sources: Vec<ImportSource>,
}

pub struct Source {
    name: String,
    items: Vec<(String, String)>,
}

/// Imports every bookmark we can find and returns an honest per-browser report.
pub fn import(data: &mut Data) -> ImportReport {
    let sources = collect();
    let mut report = ImportReport {
        added: 0,
        skipped: 0,
        sources: Vec::new(),
    };

    for source in sources {
        let found = source.items.len();
        let (added, skipped) = data.import_bookmarks(&source.items);
        report.added += added;
        report.skipped += skipped;
        if found > 0 {
            report.sources.push(ImportSource {
                name: source.name,
                found,
                added,
            });
        }
    }

    report
}

fn collect() -> Vec<Source> {
    let mut sources = Vec::new();

    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let base = PathBuf::from(local);
        for (browser, relative) in [
            ("Google Chrome", r"Google\Chrome\User Data"),
            ("Microsoft Edge", r"Microsoft\Edge\User Data"),
            ("Brave", r"BraveSoftware\Brave-Browser\User Data"),
            ("Vivaldi", r"Vivaldi\User Data"),
            ("Chromium", r"Chromium\User Data"),
        ] {
            scan_user_data(&base.join(relative), browser, &mut sources);
        }
    }

    if let Some(roaming) = std::env::var_os("APPDATA") {
        let base = PathBuf::from(roaming);
        for (browser, relative) in [
            ("Opera", r"Opera Software\Opera Stable"),
            ("Opera GX", r"Opera Software\Opera GX Stable"),
        ] {
            scan_profile(&base.join(relative), browser, &mut sources);
        }
    }

    sources
}

fn scan_user_data(root: &Path, browser: &str, out: &mut Vec<Source>) {
    if !root.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "Default" || name.starts_with("Profile") {
                let label = if name == "Default" {
                    browser.to_string()
                } else {
                    format!("{browser} · {name}")
                };
                scan_profile(&path, &label, out);
            }
        }
    }
}

fn scan_profile(profile: &Path, label: &str, out: &mut Vec<Source>) {
    let file = profile.join("Bookmarks");
    if let Some(items) = parse_bookmarks(&file) {
        out.push(Source {
            name: label.to_string(),
            items,
        });
    }
}

fn parse_bookmarks(path: &Path) -> Option<Vec<(String, String)>> {
    let raw = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    let mut items = Vec::new();
    if let Some(roots) = value.get("roots").and_then(Value::as_object) {
        for root in roots.values() {
            collect_node(root, &mut items);
        }
    }
    Some(items)
}

fn collect_node(node: &Value, out: &mut Vec<(String, String)>) {
    if node.get("type").and_then(Value::as_str) == Some("url") {
        let url = node.get("url").and_then(Value::as_str).unwrap_or("");
        let title = node.get("name").and_then(Value::as_str).unwrap_or("");
        if url.starts_with("http://") || url.starts_with("https://") {
            out.push((title.to_string(), url.to_string()));
        }
    }
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        for child in children {
            collect_node(child, out);
        }
    }
}
