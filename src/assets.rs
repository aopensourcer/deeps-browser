use std::borrow::Cow;

use include_dir::{include_dir, Dir};
use wry::http::{Request, Response};

static CHROME: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/chrome/out");

pub fn respond(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    match lookup(request.uri().path()) {
        Some((bytes, mime)) => Response::builder()
            .status(200)
            .header("Content-Type", mime)
            .header("Cache-Control", "no-store")
            .body(Cow::Borrowed(bytes))
            .expect("valid response"),
        None => Response::builder()
            .status(404)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Cow::Borrowed(&b"not found"[..]))
            .expect("valid response"),
    }
}

fn lookup(path: &str) -> Option<(&'static [u8], &'static str)> {
    let mut clean = path.trim_start_matches('/');
    if clean.is_empty() {
        clean = "index.html";
    }

    for candidate in [clean.to_string(), format!("{clean}.html"), format!("{clean}/index.html")] {
        if let Some(file) = CHROME.get_file(&candidate) {
            return Some((file.contents(), mime(&candidate)));
        }
    }

    None
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
