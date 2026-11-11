//! Search engines and the `!initial` syntax.
//!
//! The same table drives the address bar, the new-tab page and the settings
//! page, so an engine is described in exactly one place.

pub struct Engine {
    pub key: &'static str,
    pub label: &'static str,
    pub initial: char,
    pub home: &'static str,
    pub search: &'static str,
}

pub const ENGINES: &[Engine] = &[
    Engine {
        key: "ddg",
        label: "DuckDuckGo",
        initial: 'd',
        home: "https://duckduckgo.com/",
        search: "https://duckduckgo.com/?q={q}",
    },
    Engine {
        key: "google",
        label: "Google",
        initial: 'g',
        home: "https://www.google.com/",
        search: "https://www.google.com/search?q={q}",
    },
    Engine {
        key: "bing",
        label: "Bing",
        initial: 'b',
        home: "https://www.bing.com/",
        search: "https://www.bing.com/search?q={q}",
    },
    Engine {
        key: "startpage",
        label: "Startpage",
        initial: 's',
        home: "https://www.startpage.com/",
        search: "https://www.startpage.com/sp/search?query={q}",
    },
    Engine {
        key: "yahoo",
        label: "Yahoo",
        initial: 'y',
        home: "https://search.yahoo.com/",
        search: "https://search.yahoo.com/search?p={q}",
    },
    Engine {
        key: "ecosia",
        label: "Ecosia",
        initial: 'e',
        home: "https://www.ecosia.org/",
        search: "https://www.ecosia.org/search?q={q}",
    },
    Engine {
        key: "qwant",
        label: "Qwant",
        initial: 'q',
        home: "https://www.qwant.com/",
        search: "https://www.qwant.com/?q={q}",
    },
    Engine {
        key: "wikipedia",
        label: "Wikipedia",
        initial: 'w',
        home: "https://en.wikipedia.org/",
        search: "https://en.wikipedia.org/w/index.php?search={q}",
    },
];

/// Site-specific bangs, e.g. `!gh rust` searches GitHub and `!gh` opens it.
pub struct Site {
    pub key: &'static str,
    pub label: &'static str,
    pub home: &'static str,
    pub search: &'static str,
}

pub const SITES: &[Site] = &[
    Site {
        key: "gh",
        label: "GitHub",
        home: "https://github.com/",
        search: "https://github.com/search?q={q}&type=repositories",
    },
    Site {
        key: "yt",
        label: "YouTube",
        home: "https://www.youtube.com/",
        search: "https://www.youtube.com/results?search_query={q}",
    },
    Site {
        key: "rd",
        label: "Reddit",
        home: "https://www.reddit.com/",
        search: "https://www.reddit.com/search/?q={q}",
    },
    Site {
        key: "tw",
        label: "X",
        home: "https://x.com/",
        search: "https://x.com/search?q={q}&f=live",
    },
    Site {
        key: "ig",
        label: "Instagram",
        home: "https://www.instagram.com/",
        search: "https://www.instagram.com/explore/search/keyword/?q={q}",
    },
    Site {
        key: "so",
        label: "Stack Overflow",
        home: "https://stackoverflow.com/",
        search: "https://stackoverflow.com/search?q={q}",
    },
    Site {
        key: "md",
        label: "MDN",
        home: "https://developer.mozilla.org/",
        search: "https://developer.mozilla.org/en-US/search?q={q}",
    },
    Site {
        key: "npm",
        label: "npm",
        home: "https://www.npmjs.com/",
        search: "https://www.npmjs.com/search?q={q}",
    },
    Site {
        key: "cr",
        label: "crates.io",
        home: "https://crates.io/",
        search: "https://crates.io/search?q={q}",
    },
    Site {
        key: "am",
        label: "Amazon",
        home: "https://www.amazon.it/",
        search: "https://www.amazon.it/s?k={q}",
    },
    Site {
        key: "wb",
        label: "Wikipedia IT",
        home: "https://it.wikipedia.org/",
        search: "https://it.wikipedia.org/w/index.php?search={q}",
    },
    Site {
        key: "nyt",
        label: "New York Times",
        home: "https://www.nytimes.com/",
        search: "https://www.nytimes.com/search?query={q}",
    },
    Site {
        key: "bbc",
        label: "BBC",
        home: "https://www.bbc.com/",
        search: "https://www.bbc.com/search?q={q}",
    },
    Site {
        key: "cnn",
        label: "CNN",
        home: "https://edition.cnn.com/",
        search: "https://edition.cnn.com/search?q={q}",
    },
    Site {
        key: "espn",
        label: "ESPN",
        home: "https://www.espn.com/",
        search: "https://www.espn.com/search/_/q/{q}",
    },
    Site {
        key: "imdb",
        label: "IMDb",
        home: "https://www.imdb.com/",
        search: "https://www.imdb.com/find/?q={q}",
    },
    Site {
        key: "sp",
        label: "Spotify",
        home: "https://open.spotify.com/",
        search: "https://open.spotify.com/search/{q}",
    },
    Site {
        key: "nf",
        label: "Netflix",
        home: "https://www.netflix.com/",
        search: "https://www.netflix.com/search?q={q}",
    },
    Site {
        key: "tg",
        label: "Telegram",
        home: "https://web.telegram.org/",
        search: "https://t.me/{q}",
    },
    Site {
        key: "li",
        label: "LinkedIn",
        home: "https://www.linkedin.com/",
        search: "https://www.linkedin.com/search/results/all/?keywords={q}",
    },
    Site {
        key: "fb",
        label: "Facebook",
        home: "https://www.facebook.com/",
        search: "https://www.facebook.com/search/top?q={q}",
    },
    Site {
        key: "pin",
        label: "Pinterest",
        home: "https://www.pinterest.com/",
        search: "https://www.pinterest.com/search/pins/?q={q}",
    },
    Site {
        key: "maps",
        label: "Google Maps",
        home: "https://maps.google.com/",
        search: "https://www.google.com/maps/search/{q}",
    },
    Site {
        key: "drive",
        label: "Google Drive",
        home: "https://drive.google.com/",
        search: "https://drive.google.com/drive/search?q={q}",
    },
    Site {
        key: "gpt",
        label: "ChatGPT",
        home: "https://chatgpt.com/",
        search: "https://chatgpt.com/?q={q}",
    },
];

pub fn by_site_key(key: &str) -> Option<&'static Site> {
    SITES.iter().find(|site| site.key.eq_ignore_ascii_case(key))
}

pub fn by_key(key: &str) -> Option<&'static Engine> {
    ENGINES.iter().find(|engine| engine.key.eq_ignore_ascii_case(key))
}

pub fn by_initial(initial: char) -> Option<&'static Engine> {
    ENGINES
        .iter()
        .find(|engine| engine.initial.eq_ignore_ascii_case(&initial))
}

pub fn encode(text: &str) -> String {
    url::form_urlencoded::byte_serialize(text.as_bytes()).collect()
}

/// Search URL for an engine key. Unknown keys fall back to DuckDuckGo so a
/// stale setting can never break the address bar.
pub fn query(engine: &str, text: &str) -> String {
    let engine = by_key(engine).unwrap_or(&ENGINES[0]);
    engine.search.replace("{q}", &encode(text))
}

/// Handles the `!x query` syntax.
///
/// Returns the URL to open plus `false` when the initial does not belong to a
/// supported engine. In that case the whole text is searched with the default
/// engine, which is what the user asked for.
pub fn bang(text: &str, default_engine: &str) -> (String, bool) {
    let trimmed = text.trim();
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let head = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();

    if let Some(initial) = head.strip_prefix('!') {
        // Multi-letter site bangs are checked first so `!gh` cannot be
        // mistaken for the single-letter `!g` engine.
        if let Some(site) = by_site_key(initial) {
            if rest.is_empty() {
                return (site.home.to_string(), true);
            }
            return (site.search.replace("{q}", &encode(rest)), true);
        }

        let mut chars = initial.chars();
        if let (Some(first), None) = (chars.next(), chars.next()) {
            if let Some(engine) = by_initial(first) {
                if rest.is_empty() {
                    return (engine.home.to_string(), true);
                }
                return (
                    engine.search.replace("{q}", &encode(rest)),
                    true,
                );
            }
            return (query(default_engine, trimmed), false);
        }
    }

    (query(default_engine, trimmed), true)
}

#[cfg(test)]
mod tests {
    use super::bang;

    #[test]
    fn selects_engine_by_initial() {
        let (url, ok) = bang("!g rust webview", "ddg");
        assert!(ok);
        assert!(url.starts_with("https://www.google.com/search?q=rust+webview"));
    }

    #[test]
    fn unknown_initial_searches_everything() {
        let (url, ok) = bang("!z strange query", "ddg");
        assert!(!ok);
        assert!(url.starts_with("https://duckduckgo.com/?q=%21z+strange+query"));
    }

    #[test]
    fn bare_initial_opens_home() {
        let (url, ok) = bang("!w", "ddg");
        assert!(ok);
        assert_eq!(url, "https://en.wikipedia.org/");
    }

    #[test]
    fn site_bang_wins_over_engine_initial() {
        let (url, ok) = bang("!gh rust webview", "ddg");
        assert!(ok);
        assert!(url.starts_with("https://github.com/search?q=rust+webview"));
        assert!(url.contains("type=repositories"));
    }

    #[test]
    fn bare_site_bang_opens_home() {
        let (url, ok) = bang("!yt", "ddg");
        assert!(ok);
        assert_eq!(url, "https://www.youtube.com/");
    }

    #[test]
    fn site_table_is_complete_and_unique() {
        assert_eq!(super::SITES.len(), 25);
        for (index, site) in super::SITES.iter().enumerate() {
            assert!(!site.key.is_empty());
            assert!(site.search.contains("{q}"));
            for other in &super::SITES[index + 1..] {
                assert_ne!(site.key, other.key);
            }
        }
    }
}
