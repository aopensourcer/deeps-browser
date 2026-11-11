use url::Url;

use crate::engines;

pub struct Resolved {
    pub url: String,
    /// Set when the user typed `!x` for an engine we do not support.
    pub notice: Option<String>,
}

/// Turns whatever the user typed in the address bar into a URL.
/// Anything that is not clearly a host, a path or a `deeps://` page becomes a
/// web search with the configured engine.
pub fn resolve(input: &str, start: &str, engine: &str) -> Resolved {
    let text = input.trim();

    if text.is_empty() || text.eq_ignore_ascii_case("deeps://newtab") || text.eq_ignore_ascii_case("about:newtab") {
        return plain(start.to_string());
    }

    if let Some(rest) = strip_scheme(text, "deeps://") {
        let rest = rest.trim_start_matches('/');
        if rest.is_empty() {
            return plain(start.to_string());
        }
        return plain(format!("{}{}", start.trim_end_matches('/'), format!("/{rest}")));
    }

    if looks_like_path(text) {
        if let Ok(url) = Url::from_file_path(text) {
            return plain(url.to_string());
        }
    }

    if let Ok(url) = Url::parse(text) {
        if matches!(url.scheme(), "http" | "https" | "file") {
            return plain(url.to_string());
        }
    }

    if looks_like_host(text) {
        let scheme = if is_local(text) { "http" } else { "https" };
        return plain(format!("{scheme}://{text}"));
    }

    if text.starts_with('!') {
        let (url, supported) = engines::bang(text, engine);
        return Resolved {
            url,
            notice: if supported {
                None
            } else {
                Some("Non supportiamo questo motore".to_string())
            },
        };
    }

    plain(engines::query(engine, text))
}

fn plain(url: String) -> Resolved {
    Resolved { url, notice: None }
}

fn strip_scheme<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&text[prefix.len()..])
    } else {
        None
    }
}

fn host_part(text: &str) -> &str {
    text.split(['/', '?', '#']).next().unwrap_or("")
}

fn is_local(text: &str) -> bool {
    let host = host_part(text).split(':').next().unwrap_or("");
    host == "localhost"
        || host.starts_with("127.")
        || host == "[::1]"
        || host.ends_with(".local")
        || host.ends_with(".localhost")
}

fn looks_like_host(text: &str) -> bool {
    if text.chars().any(char::is_whitespace) || text.contains('@') {
        return false;
    }

    let host = host_part(text);
    if host.is_empty() || host.starts_with('[') {
        return false;
    }

    let without_port = host.split(':').next().unwrap_or(host);
    if without_port == "localhost" {
        return true;
    }
    if without_port.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }

    let mut labels = without_port.rsplit('.');
    match (labels.next(), labels.next()) {
        (Some(tld), Some(_)) => tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()),
        _ => false,
    }
}

fn looks_like_path(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() > 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

#[cfg(test)]
mod tests {
    use super::resolve;

    const START: &str = "http://127.0.0.1:9999/";

    #[test]
    fn searches_plain_words() {
        let resolved = resolve("rust webview", START, "ddg");
        assert!(resolved
            .url
            .starts_with("https://duckduckgo.com/?q=rust+webview"));
        assert!(resolved.notice.is_none());
    }

    #[test]
    fn keeps_urls() {
        assert_eq!(
            resolve("https://example.com/a?b=1", START, "ddg").url,
            "https://example.com/a?b=1"
        );
    }

    #[test]
    fn adds_https_to_domains() {
        assert_eq!(
            resolve("example.com/docs", START, "ddg").url,
            "https://example.com/docs"
        );
    }

    #[test]
    fn adds_http_to_localhost() {
        assert_eq!(
            resolve("localhost:3000", START, "ddg").url,
            "http://localhost:3000"
        );
    }

    #[test]
    fn treats_versions_as_searches() {
        assert!(resolve("1.5", START, "ddg")
            .url
            .starts_with("https://duckduckgo.com/"));
    }

    #[test]
    fn empty_goes_to_new_tab() {
        assert_eq!(resolve("", START, "ddg").url, START);
    }

    #[test]
    fn deeps_pages_map_to_loopback() {
        assert_eq!(
            resolve("deeps://history", START, "ddg").url,
            "http://127.0.0.1:9999/history"
        );
        assert_eq!(resolve("deeps://", START, "ddg").url, START);
    }

    #[test]
    fn uses_selected_engine() {
        let resolved = resolve("rust", START, "google");
        assert!(resolved
            .url
            .starts_with("https://www.google.com/search?q=rust"));
    }

    #[test]
    fn bang_syntax_and_notice() {
        let google = resolve("!g rust", START, "ddg");
        assert!(google.url.starts_with("https://www.google.com/search"));
        assert!(google.notice.is_none());

        let unknown = resolve("!z rust", START, "ddg");
        assert!(unknown.url.starts_with("https://duckduckgo.com/"));
        assert!(unknown.notice.is_some());
    }
}
