//! WebView2 launch arguments: ad blocking and a hard cap on renderer memory.

/// Curated host blocklist. Host resolver rules reject the lookup, so the
/// request never leaves the machine.
pub const HOSTS: &[&str] = &[
    "2mdn.net",
    "adcolony.com",
    "adform.net",
    "adnxs.com",
    "adsrvr.org",
    "advertising.com",
    "amazon-adsystem.com",
    "applovin.com",
    "appnexus.com",
    "bidswitch.net",
    "casalemedia.com",
    "criteo.com",
    "criteo.net",
    "doubleclick.net",
    "facebook.net",
    "google-analytics.com",
    "googleadservices.com",
    "googlesyndication.com",
    "googletagmanager.com",
    "googletagservices.com",
    "hotjar.com",
    "imasdk.googleapis.com",
    "inmobi.com",
    "ironsrc.com",
    "lijit.com",
    "media.net",
    "mixpanel.com",
    "moatads.com",
    "mopub.com",
    "openx.net",
    "outbrain.com",
    "pubmatic.com",
    "quantserve.com",
    "rubiconproject.com",
    "scorecardresearch.com",
    "segment.io",
    "serving-sys.com",
    "sharethrough.com",
    "smartadserver.com",
    "taboola.com",
    "teads.tv",
    "tremorhub.com",
    "unityads.unity3d.com",
    "vungle.com",
    "yieldmo.com",
];

pub fn browser_args(adblock: bool) -> String {
    // WebView2 already keeps sites isolated; a renderer cap and process-per-site
    // keep a dozen tabs from turning into a dozen Chromium trees.
    let mut args = String::from("--process-per-site --renderer-process-limit=4");

    if adblock {
        let rules: Vec<String> = HOSTS
            .iter()
            .flat_map(|host| {
                [
                    format!("MAP {host} ~NOTFOUND"),
                    format!("MAP *.{host} ~NOTFOUND"),
                ]
            })
            .collect();
        args.push_str(" --host-resolver-rules=\"");
        args.push_str(&rules.join(", "));
        args.push('"');
    }

    if let Ok(existing) = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS") {
        if !existing.trim().is_empty() {
            args.push(' ');
            args.push_str(existing.trim());
        }
    }

    args
}

/// Must be called before the first webview is created.
pub fn apply(adblock: bool) {
    let args = browser_args(adblock);
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", args);
}
