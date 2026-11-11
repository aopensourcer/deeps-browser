//! The tiny loopback server behind the new-tab page and the internal pages
//! (history, bookmarks, passwords, settings).
//!
//! It serves real HTML on a real origin, so normal `fetch`, forms and links
//! work without any privileged bridge. Every `/api/*` call needs the random
//! per-run token embedded in the page, plus a same-origin request.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::data::{now_ms, Credential, Data};
use crate::{defaults, downloads, engines, importer};
use crate::logo;

const BASE_CSS: &str = include_str!("../internal/base.css");
const INTERNAL_JS: &str = include_str!("../internal/internal.js");
const START_HTML: &str = include_str!("../internal/start.html");
const HISTORY_HTML: &str = include_str!("../internal/history.html");
const BOOKMARKS_HTML: &str = include_str!("../internal/bookmarks.html");
const PASSWORDS_HTML: &str = include_str!("../internal/passwords.html");
const SETTINGS_HTML: &str = include_str!("../internal/settings.html");
const TRANSLATE_HTML: &str = include_str!("../internal/translate.html");
const VPN_HTML: &str = include_str!("../internal/vpn.html");
const DOWNLOADS_HTML: &str = include_str!("../internal/downloads.html");

const CSP: &str = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; \
                   script-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; \
                   form-action 'self' https:";

struct Request {
    method: String,
    target: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

pub fn serve(data: Arc<Mutex<Data>>) -> std::io::Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let data = data.clone();
            thread::spawn(move || {
                let _ = handle(stream, data, port);
            });
        }
    });

    Ok(port)
}

fn handle(mut stream: TcpStream, data: Arc<Mutex<Data>>, port: u16) -> std::io::Result<()> {
    let Some(request) = read_request(&mut stream)? else {
        return Ok(());
    };

    let origin = format!("http://127.0.0.1:{port}");
    let path = request.target.split('?').next().unwrap_or("/").to_string();
    let query = query_params(&request.target);

    if path.starts_with("/api/") {
        if !authorized(&request, &data, &origin) {
            return write_json(&mut stream, 403, &json!({ "error": "Non autorizzato" }));
        }
        let (status, body) = api(&request, &path, &query, &data);
        return write_json(&mut stream, status, &body);
    }

    match (request.method.as_str(), path.as_str()) {
        ("GET", "/") | ("GET", "/start") => {
            let html = start_page(&data);
            write_html(&mut stream, &html)
        }
        ("GET", "/history") => write_page(&mut stream, HISTORY_HTML, &data),
        ("GET", "/bookmarks") => write_page(&mut stream, BOOKMARKS_HTML, &data),
        ("GET", "/downloads") => write_page(&mut stream, DOWNLOADS_HTML, &data),
        ("GET", "/passwords") => write_page(&mut stream, PASSWORDS_HTML, &data),
        ("GET", "/settings") => write_page(&mut stream, SETTINGS_HTML, &data),
        ("GET", "/translate") => write_page(&mut stream, TRANSLATE_HTML, &data),
        ("GET", "/vpn") => write_page(&mut stream, VPN_HTML, &data),
        ("GET", "/wallpaper") => write_wallpaper(&mut stream, &data),
        ("GET", "/internal.css") => write_asset(&mut stream, "text/css; charset=utf-8", BASE_CSS.as_bytes()),
        ("GET", "/internal.js") => write_asset(
            &mut stream,
            "text/javascript; charset=utf-8",
            INTERNAL_JS.as_bytes(),
        ),
        ("GET", "/vortex.svg") | ("GET", "/favicon.svg") => {
            write_asset(&mut stream, "image/svg+xml", logo::SVG.as_bytes())
        }
        ("GET", "/search") => {
            let text = query.get("q").map(String::as_str).unwrap_or("").trim();
            if text.is_empty() {
                return redirect(&mut stream, "/");
            }
            let engine = data.lock().map(|state| state.settings.engine.clone()).unwrap_or_else(|_| "ddg".to_string());
            let url = if text.starts_with('!') {
                engines::bang(text, &engine).0
            } else {
                engines::query(&engine, text)
            };
            redirect(&mut stream, &url)
        }
        _ => write_status(&mut stream, 404, "text/plain; charset=utf-8", b"not found"),
    }
}

/* ------------------------------------------------------------------ */
/* pages                                                               */
/* ------------------------------------------------------------------ */

fn start_page(data: &Arc<Mutex<Data>>) -> String {
    let (token, engine_key, bookmarks, language) = {
        let state = match data.lock() {
            Ok(state) => state,
            Err(_) => return START_HTML.replace("__DEEPS_TOKEN__", ""),
        };
        (
            state.token.clone(),
            state.settings.engine.clone(),
            state.bookmarks(),
            state.settings.language.clone(),
        )
    };
    let english = language != "it";

    let engine = engines::by_key(&engine_key).unwrap_or(&engines::ENGINES[0]);
    let engine_hint = engines::ENGINES
        .iter()
        .map(|item| format!("!{} {}", item.initial, item.label))
        .collect::<Vec<_>>()
        .join(" · ");
    let site_hint = engines::SITES
        .iter()
        .take(12)
        .map(|item| format!("!{}", item.key))
        .collect::<Vec<_>>()
        .join(" ");
    let hint = if english {
        format!("{engine_hint}  ·  sites: {site_hint} …")
    } else {
        format!("{engine_hint}  ·  siti: {site_hint} …")
    };

    let mut links = String::new();
    if bookmarks.is_empty() {
        for (title, url) in [
            ("GitHub", "https://github.com"),
            ("Wikipedia", "https://en.wikipedia.org"),
            ("MDN", "https://developer.mozilla.org"),
            ("Hacker News", "https://news.ycombinator.com"),
        ] {
            links.push_str(&quick_link(title, url));
        }
    } else {
        for bookmark in bookmarks.iter().take(8) {
            links.push_str(&quick_link(&bookmark.title, &bookmark.url));
        }
    }

    let title = if english { "New tab" } else { "Nuova tab" };
    let placeholder = if english {
        format!("Search with {} or type an address", engine.label)
    } else {
        format!("Cerca con {} o digita un indirizzo", engine.label)
    };
    let footer = if english {
        "Ctrl+T new tab · Ctrl+K menu · Ctrl+L address · !g for Google"
    } else {
        "Ctrl+T nuova tab · Ctrl+K menu · Ctrl+L indirizzo · !g per Google"
    };
    let submit = if english { "Enter" } else { "Invio" };

    START_HTML
        .replace("__DEEPS_TOKEN__", &token)
        .replace("__DEEPS_LANG__", &language)
        .replace("__QUICKLINKS__", &links)
        .replace("__TITLE__", title)
        .replace("__PLACEHOLDER__", &escape_html(&placeholder))
        .replace("__SUBMIT__", submit)
        .replace("__FOOTER__", footer)
        .replace("__ENGINE_LABEL__", &escape_html(engine.label))
        .replace("__ENGINE_HINT__", &escape_html(&hint))
}

fn quick_link(title: &str, url: &str) -> String {
    let label = if title.trim().is_empty() {
        host_of(url).unwrap_or_else(|| url.to_string())
    } else {
        title.to_string()
    };
    let hue = hue_of(&label);
    let letter = label
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());
    format!(
        r#"<a href="{}"><i style="--h:{}">{}</i><span>{}</span></a>"#,
        escape_html(url),
        hue,
        escape_html(&letter),
        escape_html(&label)
    )
}

fn write_page(stream: &mut TcpStream, template: &str, data: &Arc<Mutex<Data>>) -> std::io::Result<()> {
    let (token, language) = data
        .lock()
        .map(|state| (state.token.clone(), state.settings.language.clone()))
        .unwrap_or_else(|_| (String::new(), "en".to_string()));
    let html = template
        .replace("__DEEPS_TOKEN__", &token)
        .replace("__DEEPS_LANG__", &language);
    write_html(stream, &html)
}

/// Serves the user's wallpaper photo stored in the profile as a data URL.
/// Chrome and internal pages both load it from here, so the (potentially
/// large) data URL never travels in state snapshots.
fn write_wallpaper(stream: &mut TcpStream, data: &Arc<Mutex<Data>>) -> std::io::Result<()> {
    let photo = data
        .lock()
        .map(|state| state.settings.photo.clone())
        .unwrap_or_default();
    if photo.is_empty() {
        return write_status(stream, 404, "text/plain; charset=utf-8", b"no wallpaper");
    }
    let Some(rest) = photo.strip_prefix("data:") else {
        return write_status(stream, 404, "text/plain; charset=utf-8", b"no wallpaper");
    };
    let Some((meta, payload)) = rest.split_once(',') else {
        return write_status(stream, 404, "text/plain; charset=utf-8", b"no wallpaper");
    };
    let Some(mime) = meta.strip_suffix(";base64") else {
        return write_status(stream, 404, "text/plain; charset=utf-8", b"no wallpaper");
    };
    use base64::Engine;
    let bytes = match base64::engine::general_purpose::STANDARD.decode(payload.trim()) {
        Ok(bytes) => bytes,
        Err(_) => {
            return write_status(stream, 404, "text/plain; charset=utf-8", b"no wallpaper")
        }
    };
    write_status(stream, 200, mime, &bytes)
}

/* ------------------------------------------------------------------ */
/* api                                                                 */
/* ------------------------------------------------------------------ */

fn api(
    request: &Request,
    path: &str,
    query: &HashMap<String, String>,
    data: &Arc<Mutex<Data>>,
) -> (u16, Value) {
    let method = request.method.as_str();
    let rest = path.trim_start_matches("/api/");
    let parts: Vec<&str> = rest.split('/').filter(|part| !part.is_empty()).collect();

    let body: Value = if request.body.is_empty() {
        Value::Null
    } else {
        match serde_json::from_slice(&request.body) {
            Ok(value) => value,
            Err(_) => return (400, json!({ "error": "Richiesta non valida" })),
        }
    };

    match (method, parts.as_slice()) {
        ("GET", ["settings"]) => settings_payload(data),
        ("PUT", ["settings"]) => {
            let mut state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            match state.settings.patch(&body) {
                Ok(next) => state.set_settings(next),
                Err(error) => return (400, json!({ "error": error })),
            }
            drop(state);
            settings_payload(data)
        }
        ("POST", ["settings", "folder"]) => match downloads::pick_folder() {
            Ok(Some(path)) => (200, json!({ "path": path })),
            Ok(None) => (200, json!({ "path": Value::Null })),
            Err(error) => (500, json!({ "error": error })),
        },

        ("GET", ["history"]) => {
            let text = query.get("q").cloned().unwrap_or_default();
            let limit = query
                .get("limit")
                .and_then(|raw| raw.parse::<usize>().ok())
                .unwrap_or(500)
                .clamp(1, 2000);
            let entries = data
                .lock()
                .map(|state| state.history(&text, limit))
                .unwrap_or_default();
            (200, json!({ "entries": entries }))
        }
        ("DELETE", ["history"]) => {
            if let Ok(mut state) = data.lock() {
                state.clear_history();
            }
            (200, json!({ "ok": true }))
        }
        ("DELETE", ["history", id]) => {
            let id = parse_id(id);
            if let Ok(mut state) = data.lock() {
                state.delete_history(id);
            }
            (200, json!({ "ok": true }))
        }

        ("GET", ["bookmarks"]) => {
            let entries = data
                .lock()
                .map(|state| state.bookmarks())
                .unwrap_or_default();
            (200, json!({ "entries": entries }))
        }
        ("POST", ["bookmarks"]) => {
            let url = body.get("url").and_then(Value::as_str).unwrap_or("").trim();
            let title = body.get("title").and_then(Value::as_str).unwrap_or("");
            let mut state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            match state.add_bookmark(url, title) {
                Ok(entry) => (200, json!({ "entry": entry })),
                Err(error) => (400, json!({ "error": error })),
            }
        }
        ("DELETE", ["bookmarks", id]) => {
            let id = parse_id(id);
            if let Ok(mut state) = data.lock() {
                state.delete_bookmark(id);
            }
            (200, json!({ "ok": true }))
        }

        ("GET", ["downloads"]) => {
            let entries = data
                .lock()
                .map(|state| state.downloads())
                .unwrap_or_default();
            (200, json!({ "entries": entries }))
        }
        ("DELETE", ["downloads"]) => {
            if let Ok(mut state) = data.lock() {
                state.clear_downloads();
            }
            (200, json!({ "ok": true }))
        }
        ("DELETE", ["downloads", id]) => {
            let id = parse_id(id);
            if let Ok(mut state) = data.lock() {
                state.delete_download(id);
            }
            (200, json!({ "ok": true }))
        }
        ("POST", ["downloads", id, "open"]) => {
            let id = parse_id(id);
            let path = data
                .lock()
                .ok()
                .and_then(|state| state.download_path(id));
            match path {
                Some(path) => match downloads::open_file(&path.to_string_lossy()) {
                    Ok(()) => (200, json!({ "ok": true })),
                    Err(error) => (400, json!({ "error": error })),
                },
                None => (404, json!({ "error": "Download non trovato" })),
            }
        }
        ("POST", ["downloads", id, "folder"]) => {
            let id = parse_id(id);
            let path = data
                .lock()
                .ok()
                .and_then(|state| state.download_path(id));
            match path {
                Some(path) => match downloads::reveal(&path.to_string_lossy()) {
                    Ok(()) => (200, json!({ "ok": true })),
                    Err(error) => (400, json!({ "error": error })),
                },
                None => (404, json!({ "error": "Download non trovato" })),
            }
        }

        ("GET", ["vault", "status"]) => {
            let state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            (
                200,
                json!({
                    "exists": state.vault_exists(),
                    "unlocked": state.vault_unlocked(),
                    "count": state.vault_count(),
                }),
            )
        }
        ("POST", ["vault", "create"]) => {
            let password = body.get("password").and_then(Value::as_str).unwrap_or("");
            let mut state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            match state.create_vault(password) {
                Ok(()) => (200, json!({ "ok": true })),
                Err(error) => (400, json!({ "error": error })),
            }
        }
        ("POST", ["vault", "unlock"]) => {
            let password = body.get("password").and_then(Value::as_str).unwrap_or("");
            let mut state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            match state.unlock_vault(password) {
                Ok(()) => (200, json!({ "ok": true })),
                Err(error) => (401, json!({ "error": error })),
            }
        }
        ("POST", ["vault", "lock"]) => {
            if let Ok(mut state) = data.lock() {
                state.lock_vault();
            }
            (200, json!({ "ok": true }))
        }
        ("GET", ["vault", "entries"]) => {
            let state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            match state.credentials() {
                Ok(entries) => (200, json!({ "entries": entries })),
                Err(error) => (401, json!({ "error": error })),
            }
        }
        ("POST", ["vault", "entries"]) => {
            let entry: Credential = match serde_json::from_value(body) {
                Ok(entry) => entry,
                Err(_) => return (400, json!({ "error": "Voce non valida" })),
            };
            let mut state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            match state.save_credential(entry) {
                Ok(entry) => (200, json!({ "entry": entry })),
                Err(error) => (400, json!({ "error": error })),
            }
        }
        ("DELETE", ["vault", "entries", id]) => {
            let id = parse_id(id);
            let mut state = match data.lock() {
                Ok(state) => state,
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            match state.delete_credential(id) {
                Ok(()) => (200, json!({ "ok": true })),
                Err(error) => (400, json!({ "error": error })),
            }
        }

        ("POST", ["translate"]) => translate_api(&body),
        ("POST", ["translate", "batch"]) => translate_batch_api(&body),

        ("POST", ["import"]) => {
            let report = match data.lock() {
                Ok(mut state) => importer::import(&mut state),
                Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
            };
            (
                200,
                serde_json::to_value(report).unwrap_or_else(|_| json!({ "added": 0, "skipped": 0 })),
            )
        }
        ("GET", ["default"]) => (200, json!({ "registered": defaults::is_registered() })),
        ("POST", ["default"]) => match defaults::register_and_open() {
            Ok(path) => (200, json!({ "ok": true, "path": path })),
            Err(error) => (500, json!({ "error": error })),
        },

        ("GET", ["vpn", "status"]) => (200, vpn_status()),
        ("POST", ["vpn", "connect"]) => match run_warp(&["connect"]) {
            Ok(output) => {
                let mut value = vpn_status();
                if let Some(object) = value.as_object_mut() {
                    object.insert("output".to_string(), json!(output));
                }
                (200, value)
            }
            Err(error) => (500, json!({ "error": error })),
        },
        ("POST", ["vpn", "disconnect"]) => match run_warp(&["disconnect"]) {
            Ok(output) => {
                let mut value = vpn_status();
                if let Some(object) = value.as_object_mut() {
                    object.insert("output".to_string(), json!(output));
                }
                (200, value)
            }
            Err(error) => (500, json!({ "error": error })),
        },

        ("GET", ["quicklinks"]) => {
            let entries = data
                .lock()
                .map(|state| state.bookmarks().into_iter().take(8).collect::<Vec<_>>())
                .unwrap_or_default();
            (200, json!({ "entries": entries }))
        }

        _ => (404, json!({ "error": "Endpoint sconosciuto" })),
    }
}

fn settings_payload(data: &Arc<Mutex<Data>>) -> (u16, Value) {
    let state = match data.lock() {
        Ok(state) => state,
        Err(_) => return (500, json!({ "error": "Stato non disponibile" })),
    };
    let engines = engines::ENGINES
        .iter()
        .map(|engine| {
            json!({
                "key": engine.key,
                "label": engine.label,
                "initial": engine.initial.to_string(),
            })
        })
        .collect::<Vec<_>>();
    let sites = engines::SITES
        .iter()
        .map(|site| {
            json!({
                "key": site.key,
                "label": site.label,
            })
        })
        .collect::<Vec<_>>();
    let settings = &state.settings;
    let wallpaper = if settings.has_photo() {
        format!("/wallpaper?t={}", photo_version(&settings.photo))
    } else {
        String::new()
    };
    (
        200,
        json!({
            // The photo itself is deliberately omitted: the page loads it from
            // `/wallpaper` instead of carrying megabytes in this payload.
            "settings": {
                "engine": settings.engine,
                "shortcuts": settings.shortcuts,
                "adblock": settings.adblock,
                "tabLayout": settings.tab_layout,
                "tabShape": settings.tab_shape,
                "accent": settings.accent,
                "wallpaper": settings.wallpaper,
                "saveLogins": settings.save_logins,
                "language": settings.language,
                "translateFrom": settings.translate_from,
                "translateTo": settings.translate_to,
                "onboarded": settings.onboarded,
                "downloadDir": settings.download_dir,
                "askDownloadDir": settings.ask_download_dir,
                "hasPhoto": settings.has_photo(),
            },
            "photoUrl": wallpaper,
            "defaultDownloadDir": downloads::download_dir().to_string_lossy().into_owned(),
            "engines": engines,
            "sites": sites,
            "layouts": crate::data::TAB_LAYOUTS,
            "shapes": crate::data::TAB_SHAPES,
            "wallpapers": crate::data::WALLPAPERS,
            "languages": crate::data::LANGUAGES,
            "registered": defaults::is_registered(),
            "version": env!("CARGO_PKG_VERSION"),
        }),
    )
}

/// Stable cache-busting version for the wallpaper URL.
fn photo_version(photo: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in photo.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

/* ------------------------------------------------------------------ */
/* translator                                                          */
/* ------------------------------------------------------------------ */

fn translate_api(body: &Value) -> (u16, Value) {
    let text = body
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if text.is_empty() {
        return (400, json!({ "error": "Scrivi qualcosa da tradurre" }));
    }
    if text.chars().count() > 5000 {
        return (400, json!({ "error": "Testo troppo lungo (massimo 5000 caratteri)" }));
    }

    let from = language(body.get("from").and_then(Value::as_str).unwrap_or("auto"), true);
    let to = language(body.get("to").and_then(Value::as_str).unwrap_or("it"), false);
    let (Some(from), Some(to)) = (from, to) else {
        return (400, json!({ "error": "Lingua non supportata" }));
    };

    let url = format!(
        "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
        from,
        to,
        engines::encode(text)
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .build();
    let response = match agent
        .get(&url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Deeps/0.1")
        .call()
    {
        Ok(response) => response,
        Err(error) => {
            return (
                502,
                json!({ "error": format!("Traduttore non raggiungibile: {error}") }),
            )
        }
    };

    let raw = match response.into_string() {
        Ok(raw) => raw,
        Err(error) => {
            return (
                502,
                json!({ "error": format!("Risposta non leggibile: {error}") }),
            )
        }
    };
    let value: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return (502, json!({ "error": "Risposta del traduttore non valida" })),
    };

    let mut translated = String::new();
    if let Some(chunks) = value.get(0).and_then(Value::as_array) {
        for chunk in chunks {
            if let Some(part) = chunk.get(0).and_then(Value::as_str) {
                translated.push_str(part);
            }
        }
    }
    if translated.trim().is_empty() {
        return (502, json!({ "error": "Il traduttore non ha restituito testo" }));
    }

    let detected = value
        .get(2)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    (
        200,
        json!({
            "text": translated,
            "detected": detected,
            "from": from,
            "to": to,
        }),
    )
}

fn language(raw: &str, allow_auto: bool) -> Option<String> {
    crate::data::normalize_language(raw, allow_auto).ok()
}

/// Translates many short strings in a single request. The in-page translator
/// sends visible text nodes in small batches so a live page does not turn into
/// hundreds of HTTP round-trips.
pub fn translate_batch(texts: &[String], from: &str, to: &str) -> Result<Vec<String>, String> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    if texts.len() > 60 {
        return Err("Troppi testi in una sola richiesta".to_string());
    }
    let total: usize = texts.iter().map(|text| text.chars().count()).sum();
    if total > 6000 {
        return Err("Testo troppo lungo per il traduttore".to_string());
    }
    let from = language(from, true).ok_or_else(|| "Lingua non supportata".to_string())?;
    let to = language(to, false).ok_or_else(|| "Lingua non supportata".to_string())?;

    translate_gate();

    const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
        AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build();

    let endpoints = [
        "https://translate.googleapis.com/translate_a/t?client=gtx",
        "https://clients5.google.com/translate_a/t?client=dict-chrome-ex",
    ];
    let mut last_error = "Traduttore non raggiungibile".to_string();
    let mut limited = false;

    for base in endpoints {
        let mut url = format!("{base}&sl={from}&tl={to}");
        for text in texts {
            url.push_str("&q=");
            url.push_str(&engines::encode(text));
        }
        for attempt in 0..2 {
            match agent.get(&url).set("User-Agent", BROWSER_USER_AGENT).call() {
                Ok(response) => match response.into_string() {
                    Ok(raw) => match parse_translation(&raw, texts.len()) {
                        Ok(out) => return Ok(out),
                        Err(message) => {
                            last_error = message;
                            break;
                        }
                    },
                    Err(error) => {
                        last_error = format!("Risposta non leggibile: {error}");
                        break;
                    }
                },
                Err(ureq::Error::Status(429, _)) => {
                    limited = true;
                    last_error =
                        "Il traduttore è momentaneamente limitato, riprova tra poco".to_string();
                    if attempt == 0 {
                        thread::sleep(Duration::from_millis(700));
                    }
                }
                Err(ureq::Error::Status(code, _)) if code >= 500 => {
                    last_error = format!("Traduttore non disponibile ({code})");
                    if attempt == 0 {
                        thread::sleep(Duration::from_millis(350));
                    }
                }
                Err(error) => {
                    last_error = format!("Traduttore non raggiungibile: {error}");
                    break;
                }
            }
        }
    }

    if limited && !last_error.contains("limitato") {
        last_error = "Il traduttore è momentaneamente limitato, riprova tra poco".to_string();
    }
    Err(last_error)
}

/// Google throttles bursts from a single client, so sequential batches are
/// spaced a little instead of all firing at the same moment.
fn translate_gate() {
    static GATE: OnceLock<Mutex<Instant>> = OnceLock::new();
    let gate = GATE.get_or_init(|| Mutex::new(Instant::now()));
    let mut last = gate.lock().unwrap_or_else(|error| error.into_inner());
    let minimum = Duration::from_millis(220);
    let elapsed = last.elapsed();
    if elapsed < minimum {
        thread::sleep(minimum - elapsed);
    }
    *last = Instant::now();
}

fn parse_translation(raw: &str, expected: usize) -> Result<Vec<String>, String> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|_| "Risposta del traduttore non valida".to_string())?;
    let items = value
        .as_array()
        .ok_or_else(|| "Risposta del traduttore non valida".to_string())?;

    let mut out = Vec::with_capacity(expected);
    for (index, item) in items.iter().enumerate() {
        if index >= expected {
            break;
        }
        let translated = first_string(item)
            .ok_or_else(|| "Il traduttore non ha restituito testo".to_string())?;
        out.push(translated);
    }
    if out.len() != expected {
        return Err("Il traduttore ha restituito un numero di testi diverso".to_string());
    }
    Ok(out)
}

fn first_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => items.iter().find_map(first_string),
        Value::Object(map) => map
            .get("trans")
            .or_else(|| map.get("translation"))
            .and_then(first_string)
            .or_else(|| map.values().find_map(first_string)),
        _ => None,
    }
}

fn translate_batch_api(body: &Value) -> (u16, Value) {
    let texts = body
        .get("texts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|text| text.to_string())
                .filter(|text| !text.trim().is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if texts.is_empty() {
        return (400, json!({ "error": "Nessun testo da tradurre" }));
    }
    let from = body
        .get("from")
        .and_then(Value::as_str)
        .unwrap_or("auto");
    let to = body.get("to").and_then(Value::as_str).unwrap_or("it");
    match translate_batch(&texts, from, to) {
        Ok(translations) => (200, json!({ "translations": translations })),
        Err(error) => (502, json!({ "error": error })),
    }
}

/* ------------------------------------------------------------------ */
/* free vpn (Cloudflare WARP)                                          */
/* ------------------------------------------------------------------ */

fn warp_binary() -> PathBuf {
    if let Ok(path) = std::env::var("WARP_CLI") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    for candidate in [
        r"C:\Program Files\Cloudflare\Cloudflare WARP\warp-cli.exe",
        r"C:\Program Files (x86)\Cloudflare\Cloudflare WARP\warp-cli.exe",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return path;
        }
    }
    PathBuf::from("warp-cli")
}

fn run_warp(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new(warp_binary());
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command
        .output()
        .map_err(|_| "Cloudflare WARP non è installato".to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let errors = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !errors.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&errors);
    }
    if output.status.success() {
        Ok(text)
    } else {
        Err(if text.is_empty() {
            "warp-cli non ha risposto".to_string()
        } else {
            text
        })
    }
}

fn vpn_status() -> Value {
    if run_warp(&["--version"]).is_err() {
        return json!({
            "installed": false,
            "state": "missing",
            "detail": "Cloudflare WARP non è installato su questo computer.",
            "download": "https://1.1.1.1/",
        });
    }

    let raw = run_warp(&["status"]).unwrap_or_default();
    let lower = raw.to_lowercase();
    let state = if lower.contains("disconnected") {
        "disconnected"
    } else if lower.contains("connecting") {
        "connecting"
    } else if lower.contains("connected") {
        "connected"
    } else if lower.contains("registration") || lower.contains("missing") {
        "unregistered"
    } else if lower.contains("disabled") {
        "disabled"
    } else {
        "unknown"
    };
    json!({
        "installed": true,
        "state": state,
        "detail": raw.trim(),
        "download": "https://1.1.1.1/",
    })
}

/* ------------------------------------------------------------------ */
/* http helpers                                                        */
/* ------------------------------------------------------------------ */

fn authorized(request: &Request, data: &Arc<Mutex<Data>>, origin: &str) -> bool {
    let token = data.lock().map(|state| state.token.clone()).unwrap_or_default();
    let token_ok = request
        .headers
        .get("x-deeps-token")
        .map(|value| value == &token)
        .unwrap_or(false);
    if !token_ok {
        return false;
    }
    match request.headers.get("origin") {
        Some(value) => value == origin,
        None => true,
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request>> {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 8192];

    let header_end = loop {
        if let Some(index) = find_header_end(&raw) {
            break index;
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(None);
        }
        raw.extend_from_slice(&chunk[..read]);
        if raw.len() > 2_000_000 {
            return Ok(None);
        }
    };

    let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split_whitespace();
    let method = first.next().unwrap_or("").to_string();
    let target = first.next().unwrap_or("/").to_string();
    if method.is_empty() || target.is_empty() {
        return Ok(None);
    }

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_lowercase(), value.trim().to_string());
        }
    }

    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
        .min(9_000_000);
    let mut body = raw[header_end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);

    Ok(Some(Request {
        method,
        target,
        headers,
        body,
    }))
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|window| window == b"\r\n\r\n")
}

fn query_params(target: &str) -> HashMap<String, String> {
    let Some((_, query)) = target.split_once('?') else {
        return HashMap::new();
    };
    url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}

fn parse_id(raw: &str) -> u64 {
    raw.parse::<u64>().unwrap_or(0)
}

fn write_html(stream: &mut TcpStream, html: &str) -> std::io::Result<()> {
    write_status(stream, 200, "text/html; charset=utf-8", html.as_bytes())
}

fn write_asset(stream: &mut TcpStream, kind: &str, bytes: &[u8]) -> std::io::Result<()> {
    write_status(stream, 200, kind, bytes)
}

fn write_status(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Content-Security-Policy: {CSP}\r\n\
         X-Frame-Options: DENY\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

fn write_json(stream: &mut TcpStream, status: u16, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    write_status(stream, status, "application/json; charset=utf-8", &body)
}

fn redirect(stream: &mut TcpStream, location: &str) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 302 Found\r\n\
         Location: {location}\r\n\
         Content-Length: 0\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n"
    )
}

fn escape_html(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            other => other.to_string(),
        })
        .collect()
}

fn hue_of(seed: &str) -> u32 {
    let mut hue: u32 = 0;
    for byte in seed.bytes() {
        hue = (hue * 31 + byte as u32) % 360;
    }
    hue
}

fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|host| host.trim_start_matches("www.").to_string()))
}

/// Kept next to the server because the same timestamp format is used by the
/// pages when they render history rows.
#[allow(dead_code)]
fn tonight() -> u64 {
    now_ms()
}

#[cfg(test)]
mod tests {
    use super::{first_string, parse_translation};
    use serde_json::json;

    #[test]
    fn parses_flat_translation_arrays() {
        let raw = r#"["Ciao mondo","Questo dominio è per esempi"]"#;
        assert_eq!(
            parse_translation(raw, 2).expect("flat array"),
            vec![
                "Ciao mondo".to_string(),
                "Questo dominio è per esempi".to_string()
            ]
        );
    }

    #[test]
    fn parses_objects_from_alternate_endpoint() {
        let raw = json!([
            { "trans": "Ciao", "orig": "Hello" },
            { "trans": "Mondo", "orig": "World" }
        ])
        .to_string();
        assert_eq!(
            parse_translation(&raw, 2).expect("object entries"),
            vec!["Ciao".to_string(), "Mondo".to_string()]
        );
    }

    #[test]
    fn rejects_mismatched_counts() {
        assert!(parse_translation(r#"["solo uno"]"#, 2).is_err());
        assert!(parse_translation("not json", 1).is_err());
    }

    #[test]
    fn first_string_walks_nested_arrays() {
        let value = json!([["Ciao", "Hello", null]]);
        assert_eq!(first_string(&value), Some("Ciao".to_string()));
    }
}
