//! Everything the browser remembers: settings, history, bookmarks and the
//! encrypted password vault.
//!
//! Shared between the window (which records visits) and the loopback page
//! server (which answers `/api/*`), so it lives behind one mutex.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::engines;

const HISTORY_CAP: usize = 4000;
const DOWNLOADS_CAP: usize = 200;
const VAULT_MAGIC: &[u8; 5] = b"DPWV1";

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Settings {
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default = "yes")]
    pub shortcuts: bool,
    #[serde(default = "yes")]
    pub adblock: bool,
    /// `top` keeps the classic horizontal strip, `vertical` moves tabs into a
    /// full-height sidebar on the left.
    #[serde(default = "default_tab_layout", rename = "tabLayout")]
    pub tab_layout: String,
    /// `rounded`, `square` or `pill`.
    #[serde(default = "default_tab_shape", rename = "tabShape")]
    pub tab_shape: String,
    /// Custom accent colour in `#rrggbb` form.
    #[serde(default = "default_accent")]
    pub accent: String,
    /// One of `none`, `aurora`, `mesh`, `grid`, `dunes`, `stars`.
    #[serde(default = "default_wallpaper")]
    pub wallpaper: String,
    /// Ask before sending a detected login to the password vault.
    #[serde(default = "yes", rename = "saveLogins")]
    pub save_logins: bool,
    /// Interface language: `en` or `it`. English is the default.
    #[serde(default = "default_language")]
    pub language: String,
    /// Language the in-page translator starts from. `auto` detects it.
    #[serde(default = "default_translate_from", rename = "translateFrom")]
    pub translate_from: String,
    /// Language the in-page translator translates to.
    #[serde(default = "default_translate_to", rename = "translateTo")]
    pub translate_to: String,
    /// False until the first-run setup has been completed.
    #[serde(default)]
    pub onboarded: bool,
    /// Folder where downloads are saved. Empty means the Windows Downloads
    /// folder.
    #[serde(default, rename = "downloadDir")]
    pub download_dir: String,
    /// Ask where to save every download before it starts.
    #[serde(default = "yes", rename = "askDownloadDir")]
    pub ask_download_dir: bool,
    /// Custom wallpaper photo as a `data:image/...;base64,...` URL. Empty when
    /// the user has not picked one.
    #[serde(default)]
    pub photo: Arc<str>,
}

fn default_engine() -> String {
    "ddg".to_string()
}

fn default_tab_layout() -> String {
    "top".to_string()
}

fn default_tab_shape() -> String {
    "rounded".to_string()
}

fn default_accent() -> String {
    "#6366f1".to_string()
}

fn default_wallpaper() -> String {
    "none".to_string()
}

fn default_translate_from() -> String {
    "auto".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_translate_to() -> String {
    "it".to_string()
}

fn yes() -> bool {
    true
}

pub const TAB_LAYOUTS: &[&str] = &["top", "vertical"];
pub const TAB_SHAPES: &[&str] = &["rounded", "square", "pill"];
pub const WALLPAPERS: &[&str] = &["none", "aurora", "mesh", "grid", "dunes", "stars"];
pub const LANGUAGES: &[&str] = &["en", "it"];

impl Default for Settings {
    fn default() -> Self {
        Settings {
            engine: default_engine(),
            shortcuts: true,
            adblock: true,
            tab_layout: default_tab_layout(),
            tab_shape: default_tab_shape(),
            accent: default_accent(),
            wallpaper: default_wallpaper(),
            save_logins: true,
            language: default_language(),
            translate_from: default_translate_from(),
            translate_to: default_translate_to(),
            onboarded: false,
            download_dir: String::new(),
            ask_download_dir: true,
            photo: Arc::from(""),
        }
    }
}

impl Settings {
    /// Applies a partial JSON patch coming from either the chrome webview or
    /// the internal settings page. Unknown keys are ignored, invalid values
    /// are rejected so a bad client can never persist nonsense.
    pub fn patch(&self, body: &Value) -> Result<Settings, String> {
        let mut next = self.clone();

        if let Some(engine) = body.get("engine").and_then(Value::as_str) {
            if engines::by_key(engine).is_none() {
                return Err("Motore non supportato".to_string());
            }
            next.engine = engine.to_string();
        }
        if let Some(shortcuts) = body.get("shortcuts").and_then(Value::as_bool) {
            next.shortcuts = shortcuts;
        }
        if let Some(adblock) = body.get("adblock").and_then(Value::as_bool) {
            next.adblock = adblock;
        }
        if let Some(save_logins) = body.get("saveLogins").and_then(Value::as_bool) {
            next.save_logins = save_logins;
        }
        if let Some(onboarded) = body.get("onboarded").and_then(Value::as_bool) {
            next.onboarded = onboarded;
        }
        if let Some(layout) = body.get("tabLayout").and_then(Value::as_str) {
            let layout = layout.trim().to_lowercase();
            if !TAB_LAYOUTS.contains(&layout.as_str()) {
                return Err("Disposizione schede non supportata".to_string());
            }
            next.tab_layout = layout;
        }
        if let Some(shape) = body.get("tabShape").and_then(Value::as_str) {
            let shape = shape.trim().to_lowercase();
            if !TAB_SHAPES.contains(&shape.as_str()) {
                return Err("Forma schede non supportata".to_string());
            }
            next.tab_shape = shape;
        }
        if let Some(wallpaper) = body.get("wallpaper").and_then(Value::as_str) {
            let wallpaper = wallpaper.trim().to_lowercase();
            if !WALLPAPERS.contains(&wallpaper.as_str()) {
                return Err("Sfondo non supportato".to_string());
            }
            next.wallpaper = wallpaper;
        }
        if let Some(accent) = body.get("accent").and_then(Value::as_str) {
            next.accent = norm_accent(accent)?;
        }
        if let Some(language) = body.get("language").and_then(Value::as_str) {
            let language = language.trim().to_lowercase();
            if !LANGUAGES.contains(&language.as_str()) {
                return Err("Lingua non supportata".to_string());
            }
            next.language = language;
        }
        if let Some(from) = body.get("translateFrom").and_then(Value::as_str) {
            next.translate_from = normalize_language(from, true)?;
        }
        if let Some(to) = body.get("translateTo").and_then(Value::as_str) {
            next.translate_to = normalize_language(to, false)?;
        }
        if let Some(dir) = body.get("downloadDir").and_then(Value::as_str) {
            next.download_dir = norm_download_dir(dir)?;
        }
        if let Some(ask) = body.get("askDownloadDir").and_then(Value::as_bool) {
            next.ask_download_dir = ask;
        }
        if let Some(photo) = body.get("photo").and_then(Value::as_str) {
            next.photo = norm_photo(photo)?;
        }

        Ok(next)
    }

    pub fn has_photo(&self) -> bool {
        !self.photo.is_empty()
    }
}

/// Keeps translator language codes in the `it`, `en`, `zh-CN` shape used by
/// Google Translate. `auto` is only accepted for the source language.
pub fn normalize_language(raw: &str, allow_auto: bool) -> Result<String, String> {
    let value = raw.trim().to_lowercase();
    if value == "auto" {
        return if allow_auto {
            Ok(value)
        } else {
            Err("Lingua non supportata".to_string())
        };
    }
    let ok = (2..=8).contains(&value.len())
        && value
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '-');
    if !ok {
        return Err("Lingua non supportata".to_string());
    }
    Ok(value)
}

fn norm_accent(raw: &str) -> Result<String, String> {
    let text = raw.trim();
    let hex = text
        .strip_prefix('#')
        .ok_or_else(|| "Colore non valido".to_string())?;
    if !(hex.len() == 3 || hex.len() == 6) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Colore non valido".to_string());
    }
    let expanded = if hex.len() == 3 {
        hex.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        hex.to_string()
    };
    Ok(format!("#{}", expanded.to_lowercase()))
}

/// Largest photo data URL we are willing to store, in bytes.
const PHOTO_MAX: usize = 8_000_000;

/// Downloads are stored next to the profile, so only accept a sane folder
/// string. The path is not required to exist yet; the download code creates it.
fn norm_download_dir(raw: &str) -> Result<String, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Ok(String::new());
    }
    if text.chars().count() > 400 || text.contains('\0') {
        return Err("Cartella non valida".to_string());
    }
    Ok(text.to_string())
}

/// Validates an uploaded wallpaper photo. The chrome already downscales the
/// image, so we only check that it is a small, real base64 image data URL.
fn norm_photo(raw: &str) -> Result<Arc<str>, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Ok(Arc::from(""));
    }
    if text.len() > PHOTO_MAX {
        return Err("Immagine troppo grande".to_string());
    }
    let rest = text
        .strip_prefix("data:")
        .ok_or_else(|| "Immagine non valida".to_string())?;
    let (meta, payload) = rest
        .split_once(',')
        .ok_or_else(|| "Immagine non valida".to_string())?;
    let mime = meta
        .strip_suffix(";base64")
        .ok_or_else(|| "Immagine non valida".to_string())?;
    if !matches!(mime, "image/jpeg" | "image/png" | "image/webp") {
        return Err("Formato immagine non supportato".to_string());
    }
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|_| "Immagine non valida".to_string())?;
    if decoded.is_empty() {
        return Err("Immagine non valida".to_string());
    }
    if decoded.len() > 6_000_000 {
        return Err("Immagine troppo grande".to_string());
    }
    Ok(Arc::from(text))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub visited: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub added: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadEntry {
    pub id: u64,
    pub url: String,
    pub name: String,
    pub path: String,
    /// `running`, `done` or `failed`.
    pub state: String,
    pub started: u64,
    pub finished: u64,
    /// Bytes received so far and the total reported by the server (0 when
    /// unknown).
    #[serde(default)]
    pub received: i64,
    #[serde(default)]
    pub total: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Credential {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub site: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Space {
    #[serde(default)]
    settings: Settings,
    #[serde(default)]
    history: Vec<HistoryEntry>,
    #[serde(default)]
    bookmarks: Vec<Bookmark>,
    #[serde(default)]
    downloads: Vec<DownloadEntry>,
    #[serde(default)]
    next_id: u64,
}

struct Unlocked {
    key: [u8; 32],
    salt: [u8; 16],
    credentials: Vec<Credential>,
}

pub struct Data {
    pub settings: Settings,
    pub token: String,
    history: Vec<HistoryEntry>,
    bookmarks: Vec<Bookmark>,
    downloads: Vec<DownloadEntry>,
    next_id: u64,
    dir: PathBuf,
    unlocked: Option<Unlocked>,
}

impl Data {
    pub fn load() -> Data {
        Data::load_at(app_dir(), Some(legacy_dir()))
    }

    fn load_at(dir: PathBuf, legacy: Option<PathBuf>) -> Data {
        let source = if dir.join("space.json").exists() {
            dir.join("space.json")
        } else if let Some(legacy) = legacy {
            legacy.join("space.json")
        } else {
            dir.join("space.json")
        };
        let mut space: Space = std::fs::read_to_string(&source)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        // An existing `space.json` means this is not a first run: older
        // profiles should never be dragged through the setup wizard again.
        if source.exists() {
            space.settings.onboarded = true;
        }

        let mut max_id = 0;
        for entry in &space.history {
            max_id = max_id.max(entry.id);
        }
        for entry in &space.bookmarks {
            max_id = max_id.max(entry.id);
        }
        for entry in &space.downloads {
            max_id = max_id.max(entry.id);
        }
        if space.next_id <= max_id {
            space.next_id = max_id + 1;
        }
        if space.next_id == 0 {
            space.next_id = 1;
        }

        Data {
            settings: space.settings,
            token: random_hex(24),
            history: space.history,
            bookmarks: space.bookmarks,
            downloads: {
                // A download cannot survive a restart: entries left running
                // belong to a previous session.
                let mut entries = space.downloads;
                for entry in entries.iter_mut().filter(|entry| entry.state == "running") {
                    entry.state = "failed".to_string();
                }
                entries
            },
            next_id: space.next_id,
            dir,
            unlocked: None,
        }
    }

    /* ---------------------------------------------------------------- */
    /* settings                                                           */
    /* ---------------------------------------------------------------- */

    pub fn set_settings(&mut self, next: Settings) {
        self.settings = next;
        self.save_space();
    }

    /* ---------------------------------------------------------------- */
    /* history                                                            */
    /* ---------------------------------------------------------------- */

    pub fn record_history(&mut self, url: &str, title: &str) {
        if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
            return;
        }
        let now = now_ms();
        if let Some(last) = self.history.last_mut() {
            if last.url == url && now.saturating_sub(last.visited) < 30_000 {
                last.visited = now;
                if !title.trim().is_empty() {
                    last.title = clean(title);
                }
                self.save_space();
                return;
            }
        }

        let id = self.take_id();
        self.history.push(HistoryEntry {
            id,
            url: url.to_string(),
            title: clean(title),
            visited: now,
        });
        if self.history.len() > HISTORY_CAP {
            let overflow = self.history.len() - HISTORY_CAP;
            self.history.drain(0..overflow);
        }
        self.save_space();
    }

    /// A page can send its title after the load finished; keep the newest
    /// history row for that URL in sync.
    pub fn touch_history_title(&mut self, url: &str, title: &str) {
        if title.trim().is_empty() {
            return;
        }
        for entry in self.history.iter_mut().rev().take(8) {
            if entry.url == url {
                entry.title = clean(title);
                self.save_space();
                return;
            }
        }
    }

    pub fn history(&self, query: &str, limit: usize) -> Vec<HistoryEntry> {
        let needle = query.trim().to_lowercase();
        let mut out: Vec<HistoryEntry> = self
            .history
            .iter()
            .rev()
            .filter(|entry| {
                needle.is_empty()
                    || entry.url.to_lowercase().contains(&needle)
                    || entry.title.to_lowercase().contains(&needle)
            })
            .take(limit.max(1))
            .cloned()
            .collect();
        out.shrink_to_fit();
        out
    }

    pub fn delete_history(&mut self, id: u64) {
        self.history.retain(|entry| entry.id != id);
        self.save_space();
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.save_space();
    }

    /* ---------------------------------------------------------------- */
    /* bookmarks                                                          */
    /* ---------------------------------------------------------------- */

    pub fn bookmarks(&self) -> Vec<Bookmark> {
        let mut out = self.bookmarks.clone();
        out.sort_by(|a, b| b.added.cmp(&a.added));
        out
    }

    pub fn is_bookmarked(&self, url: &str) -> bool {
        let key = normalize_url(url);
        self.bookmarks
            .iter()
            .any(|bookmark| normalize_url(&bookmark.url) == key)
    }

    pub fn add_bookmark(&mut self, url: &str, title: &str) -> Result<Bookmark, String> {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("Questa pagina non può essere salvata".to_string());
        }
        if let Some(existing) = self
            .bookmarks
            .iter()
            .find(|bookmark| normalize_url(&bookmark.url) == normalize_url(url))
        {
            return Ok(existing.clone());
        }
        let id = self.take_id();
        let bookmark = Bookmark {
            id,
            url: url.to_string(),
            title: if title.trim().is_empty() {
                host_of(url).unwrap_or_else(|| url.to_string())
            } else {
                clean(title)
            },
            added: now_ms(),
        };
        self.bookmarks.push(bookmark.clone());
        self.save_space();
        Ok(bookmark)
    }

    /// Imports a batch of bookmarks, saving the profile once at the end.
    /// Returns `(added, skipped)`.
    pub fn import_bookmarks(&mut self, items: &[(String, String)]) -> (usize, usize) {
        let mut added = 0;
        let mut skipped = 0;
        for (title, url) in items {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                skipped += 1;
                continue;
            }
            if self.is_bookmarked(url) {
                skipped += 1;
                continue;
            }
            let id = self.take_id();
            self.bookmarks.push(Bookmark {
                id,
                url: url.clone(),
                title: if title.trim().is_empty() {
                    host_of(url).unwrap_or_else(|| url.clone())
                } else {
                    clean(title)
                },
                added: now_ms(),
            });
            added += 1;
        }
        if added > 0 {
            self.save_space();
        }
        (added, skipped)
    }

    pub fn remove_bookmark(&mut self, url: &str) -> bool {
        let key = normalize_url(url);
        let before = self.bookmarks.len();
        self.bookmarks
            .retain(|bookmark| normalize_url(&bookmark.url) != key);
        let removed = self.bookmarks.len() != before;
        if removed {
            self.save_space();
        }
        removed
    }

    pub fn delete_bookmark(&mut self, id: u64) {
        self.bookmarks.retain(|bookmark| bookmark.id != id);
        self.save_space();
    }

    /* ---------------------------------------------------------------- */
    /* downloads                                                          */
    /* ---------------------------------------------------------------- */

    pub fn downloads(&self) -> Vec<DownloadEntry> {
        let mut out = self.downloads.clone();
        out.sort_by(|a, b| b.started.cmp(&a.started));
        out
    }

    /// Records a download that just started and returns its id.
    pub fn start_download(&mut self, url: &str, path: &Path) -> u64 {
        let id = self.take_id();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("download")
            .to_string();
        self.downloads.push(DownloadEntry {
            id,
            url: url.to_string(),
            name,
            path: path.to_string_lossy().into_owned(),
            state: "running".to_string(),
            started: now_ms(),
            finished: 0,
            received: 0,
            total: 0,
        });
        if self.downloads.len() > DOWNLOADS_CAP {
            let overflow = self.downloads.len() - DOWNLOADS_CAP;
            self.downloads.drain(0..overflow);
        }
        self.save_space();
        id
    }

    /// Updates the live progress of a running download. Called for every byte
    /// event, so it deliberately avoids touching `space.json`.
    pub fn progress_download(&mut self, id: u64, received: i64, total: i64) {
        let Some(entry) = self.downloads.iter_mut().find(|entry| entry.id == id) else {
            return;
        };
        if received >= 0 {
            entry.received = received;
        }
        if total > 0 {
            entry.total = total;
        }
    }

    /// Marks a download as finished. `id` comes from `start_download`.
    pub fn finish_download(&mut self, id: u64, ok: bool) {
        let Some(entry) = self.downloads.iter_mut().find(|entry| entry.id == id) else {
            return;
        };
        entry.state = if ok { "done" } else { "failed" }.to_string();
        entry.finished = now_ms();
        if ok && entry.total > 0 {
            entry.received = entry.total;
        }
        self.save_space();
    }

    pub fn download_path(&self, id: u64) -> Option<PathBuf> {
        self.downloads
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| PathBuf::from(&entry.path))
    }

    pub fn delete_download(&mut self, id: u64) {
        self.downloads.retain(|entry| entry.id != id);
        self.save_space();
    }

    pub fn clear_downloads(&mut self) {
        self.downloads.retain(|entry| entry.state == "running");
        self.save_space();
    }

    /* ---------------------------------------------------------------- */
    /* password vault                                                     */
    /* ---------------------------------------------------------------- */

    pub fn vault_exists(&self) -> bool {
        self.dir.join("vault.bin").exists()
    }

    pub fn vault_unlocked(&self) -> bool {
        self.unlocked.is_some()
    }

    pub fn vault_count(&self) -> usize {
        self.unlocked
            .as_ref()
            .map(|state| state.credentials.len())
            .unwrap_or(0)
    }

    pub fn create_vault(&mut self, password: &str) -> Result<(), String> {
        if self.vault_exists() {
            return Err("Esiste già una cassaforte".to_string());
        }
        if password.chars().count() < 6 {
            return Err("Usa almeno 6 caratteri".to_string());
        }
        let mut salt = [0u8; 16];
        random_bytes(&mut salt)?;
        let key = derive_key(password, &salt)?;
        self.unlocked = Some(Unlocked {
            key,
            salt,
            credentials: Vec::new(),
        });
        self.save_vault()
    }

    pub fn unlock_vault(&mut self, password: &str) -> Result<(), String> {
        let raw = std::fs::read(self.dir.join("vault.bin"))
            .map_err(|_| "Cassaforte non trovata".to_string())?;
        let (salt, nonce, cipher) = parse_vault(&raw)?;
        let key = derive_key(password, &salt)?;
        let ciphertext = XChaCha20Poly1305::new(Key::from_slice(&key))
            .decrypt(XNonce::from_slice(&nonce), cipher)
            .map_err(|_| "Password errata".to_string())?;
        let credentials: Vec<Credential> = serde_json::from_slice(&ciphertext)
            .map_err(|_| "Cassaforte danneggiata".to_string())?;
        self.unlocked = Some(Unlocked {
            key,
            salt,
            credentials,
        });
        Ok(())
    }

    pub fn lock_vault(&mut self) {
        self.unlocked = None;
    }

    pub fn credentials(&self) -> Result<Vec<Credential>, String> {
        let state = self
            .unlocked
            .as_ref()
            .ok_or_else(|| "Cassaforte bloccata".to_string())?;
        let mut out = state.credentials.clone();
        out.sort_by(|a, b| a.site.to_lowercase().cmp(&b.site.to_lowercase()));
        Ok(out)
    }

    pub fn save_credential(&mut self, mut entry: Credential) -> Result<Credential, String> {
        let state = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "Cassaforte bloccata".to_string())?;
        if entry.site.trim().is_empty() {
            return Err("Indica un sito o un nome".to_string());
        }
        if entry.password.is_empty() {
            return Err("La password è vuota".to_string());
        }
        entry.site = clean(&entry.site);
        entry.username = entry.username.trim().to_string();
        entry.note = entry.note.trim().to_string();

        if entry.id == 0 {
            entry.id = self.next_id;
            self.next_id += 1;
            state.credentials.push(entry.clone());
        } else {
            let existing = state
                .credentials
                .iter_mut()
                .find(|item| item.id == entry.id)
                .ok_or_else(|| "Voce non trovata".to_string())?;
            *existing = entry.clone();
        }
        self.save_vault()?;
        Ok(entry)
    }

    pub fn delete_credential(&mut self, id: u64) -> Result<(), String> {
        let state = self
            .unlocked
            .as_mut()
            .ok_or_else(|| "Cassaforte bloccata".to_string())?;
        state.credentials.retain(|item| item.id != id);
        self.save_vault()
    }

    /// Saves a login captured from a page. Updating an existing site/user
    /// pair replaces the password instead of piling up duplicates.
    pub fn upsert_login(&mut self, site: &str, username: &str, password: &str) -> Result<(), String> {
        if password.is_empty() {
            return Err("La password è vuota".to_string());
        }
        let site = clean(site);
        if site.is_empty() {
            return Err("Sito non valido".to_string());
        }
        let username = username.trim().chars().take(200).collect::<String>();
        let password = password.chars().take(500).collect::<String>();

        {
            let state = self
                .unlocked
                .as_mut()
                .ok_or_else(|| "Cassaforte bloccata".to_string())?;
            if let Some(existing) = state.credentials.iter_mut().find(|item| {
                item.site.eq_ignore_ascii_case(&site) && item.username == username
            }) {
                existing.password = password;
            } else {
                let id = self.next_id;
                self.next_id += 1;
                state.credentials.push(Credential {
                    id,
                    site,
                    username,
                    password,
                    note: String::new(),
                });
            }
        }

        self.save_vault()
    }

    fn save_vault(&mut self) -> Result<(), String> {
        let state = self
            .unlocked
            .as_ref()
            .ok_or_else(|| "Cassaforte bloccata".to_string())?;
        let plain = serde_json::to_vec(&state.credentials)
            .map_err(|error| format!("Errore cassaforte: {error}"))?;
        let mut nonce = [0u8; 24];
        random_bytes(&mut nonce)?;
        let ciphertext = XChaCha20Poly1305::new(Key::from_slice(&state.key))
            .encrypt(XNonce::from_slice(&nonce), plain.as_slice())
            .map_err(|_| "Cifratura fallita".to_string())?;

        let mut blob = Vec::with_capacity(5 + 16 + 24 + ciphertext.len());
        blob.extend_from_slice(VAULT_MAGIC);
        blob.extend_from_slice(&state.salt);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ciphertext);
        write_atomic(&self.dir.join("vault.bin"), &blob)
    }

    /* ---------------------------------------------------------------- */
    /* internals                                                          */
    /* ---------------------------------------------------------------- */

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn save_space(&self) {
        let space = Space {
            settings: self.settings.clone(),
            history: self.history.clone(),
            bookmarks: self.bookmarks.clone(),
            downloads: self.downloads.clone(),
            next_id: self.next_id,
        };
        if let Ok(text) = serde_json::to_vec(&space) {
            let _ = write_atomic(&self.dir.join("space.json"), &text);
        }
    }
}

fn parse_vault(raw: &[u8]) -> Result<([u8; 16], [u8; 24], &[u8]), String> {
    if raw.len() < 5 + 16 + 24 || &raw[..5] != VAULT_MAGIC {
        return Err("Cassaforte danneggiata".to_string());
    }
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&raw[5..21]);
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&raw[21..45]);
    Ok((salt, nonce, &raw[45..]))
}

fn derive_key(password: &str, salt: &[u8; 16]) -> Result<[u8; 32], String> {
    let params = Params::new(19_456, 2, 1, Some(32))
        .map_err(|_| "Parametri di cifratura non validi".to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| "Derivazione della chiave fallita".to_string())?;
    Ok(key)
}

fn random_bytes(buffer: &mut [u8]) -> Result<(), String> {
    getrandom::fill(buffer).map_err(|_| "Sorgente casuale non disponibile".to_string())
}

fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    if random_bytes(&mut buffer).is_err() {
        for (index, byte) in buffer.iter_mut().enumerate() {
            *byte = (now_ms() as usize >> (index % 8)) as u8;
        }
    }
    buffer.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn clean(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect()
}

fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_lowercase()
}

fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|host| host.trim_start_matches("www.").to_string()))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, bytes).map_err(|error| error.to_string())?;
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&temp, path).map_err(|error| error.to_string())
}

fn app_dir() -> PathBuf {
    base_dir().join("Deeps")
}

fn legacy_dir() -> PathBuf {
    base_dir().join("Deepy")
}

fn base_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::{normalize_url, Data};

    fn scratch() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        std::env::temp_dir().join(format!(
            "deeps-test-{}-{}-{}",
            std::process::id(),
            super::now_ms(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn bookmark_toggle_roundtrip() {
        let dir = scratch();
        let mut data = Data::load_at(dir.clone(), None);
        let url = "https://deeps.invalid/bookmark-test";
        data.add_bookmark(url, "Test").unwrap();
        assert!(data.is_bookmarked(url));
        assert!(data.remove_bookmark(url));
        assert!(!data.is_bookmarked(url));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn trailing_slash_is_ignored() {
        assert_eq!(
            normalize_url("https://Example.com/Path/"),
            normalize_url("https://example.com/path")
        );
    }

    #[test]
    fn settings_patch_validates_and_normalizes() {
        let base = super::Settings::default();
        let next = base
            .patch(&serde_json::json!({
                "tabLayout": "vertical",
                "tabShape": "pill",
                "wallpaper": "aurora",
                "accent": "#ABC",
                "translateFrom": "EN",
                "translateTo": "fr",
            }))
            .unwrap();
        assert_eq!(next.tab_layout, "vertical");
        assert_eq!(next.tab_shape, "pill");
        assert_eq!(next.wallpaper, "aurora");
        assert_eq!(next.accent, "#aabbcc");
        assert_eq!(next.translate_from, "en");
        assert_eq!(next.translate_to, "fr");
        assert!(base.patch(&serde_json::json!({ "accent": "red" })).is_err());
        assert!(base.patch(&serde_json::json!({ "tabLayout": "diagonal" })).is_err());
        assert!(base.patch(&serde_json::json!({ "translateTo": "auto" })).is_err());
    }

    #[test]
    fn existing_profile_skips_onboarding() {
        let dir = scratch();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("space.json"), b"{}").unwrap();
        let data = Data::load_at(dir.clone(), None);
        assert!(data.settings.onboarded);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn fresh_profile_keeps_onboarding() {
        let dir = scratch();
        let data = Data::load_at(dir.clone(), None);
        assert!(!data.settings.onboarded);
        let _ = std::fs::remove_dir_all(dir);
    }
}
