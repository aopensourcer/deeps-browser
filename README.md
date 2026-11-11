**English** · [Italiano](README.it.md)

# Deeps

A minimal desktop browser: native shell in **Rust**, UI in **Next.js**.

No Electron and no heavy frameworks on top of the engine: `tao` creates the window, `wry`
(WebView2 on Windows) creates the webviews. The browser chrome is an ordinary Next.js app
exported as static files and embedded in the binary.

```
+--------------------------------------------------+
|  "chrome" webview (88px)                         |  tab strip + toolbar + window controls
+--------------------------------------------------+
|  webview of the active tab (one per tab)         |  web page
+--------------------------------------------------+
```

The window is **frameless**: minimise / maximise / close are part of the UI, and edge
resizing is handled from JavaScript.

## Download

Windows builds are on the
[Releases](https://github.com/aopensourcer/deeps-browser/releases) page: download
`Deeps-<version>-win64.zip`, unpack it and run `Deeps.exe`.

Requires Windows 10/11 with the **WebView2 Runtime** (preinstalled on Windows 11 and with a
recent Edge). The executable is not signed: on first launch SmartScreen may ask you to pick
*More info → Run anyway*. The SHA-256 of the file is in the release notes.

**Status: beta (`0.1.0-beta.1`).** Everything listed below works, but installers,
auto-updates and code signing are still missing: see [Known limitations](#known-limitations).
Bug reports are welcome — read [CONTRIBUTING.md](CONTRIBUTING.md).

## Support Deeps

Deeps is free: no ads, no telemetry, no data sold. If it is useful to you, you can support
development on Ko-fi:

[![Support Deeps on Ko-fi](https://img.shields.io/badge/Ko--fi-Support%20Deeps-FF5E5B?logo=kofi&logoColor=white)](https://ko-fi.com/semfiku/goal?g=0)

Donations help cover the project's running costs and the next beta steps.

## Features

- Multiple tabs with real favicons, loading indicator, closing and drag reordering.
- **Vertical tabs** with an icon button: when enabled, the bar shrinks to favicons only as
  soon as the pointer leaves it and expands again after a 150 ms mouse dwell; opening and
  closing are animated at 180 ms.
- Tab shape (rounded, square, pill), accent colour and wallpaper backgrounds, including a
  **personal photo** used as background.
- **UI language** in English and Italian, with English as default.
- Smart address bar: URL, host, `localhost`, path, search.
- Search with **8 engines** and `!prefix` syntax (e.g. `!g rust webview`).
- **25 site bangs** (`!gh`, `!yt`, `!am`, …) which take precedence over single-letter bangs.
- **Built-in translator**: live popup on the current page, with source and target language,
  translation of visible and dynamically added text, and restore.
- **Free VPN** through Cloudflare WARP, with real state and instructions when it is missing.
- **History** page with search, single-item delete and clear-all.
- **Bookmarks** with a star in the toolbar and a dedicated page.
- **Downloads** with a quick panel from the toolbar: mini list with progress (white ring on
  the icon), delete and a shortcut to `deeps://downloads`.
- **Folder chosen on every download** in a Deeps window (the native WebView2 save dialog
  never appears), with an "Always save here" option and a configurable default folder. File
  names stay unique and notifications report start and completion.
- Local **password manager** with an encrypted vault.
- Login saving **on confirmation**: no credential is stored without a click.
- **Passkey** status based on the Windows/WebView2 authenticator.
- **Settings** (language, engine, shortcuts, adblock, theme, photo background, download
  folder, translator) with immediate effect.
- **Bookmark import** from Chrome, Edge, Brave, Vivaldi, Chromium, Opera and Opera GX.
- **Default browser**: registers Deeps in Windows and opens the default apps page.
- **Adblock** built in at DNS level, enabled from settings.
- **Extensions** menu with adblock status.
- Own context menu, with actions on links, images, selection and page.
- Image and file drag-and-drop onto the page with preview.
- Initial onboarding on first launch, with language choice, skipped when a profile already
  exists.
- Shortcuts that can be toggled, with a dot indicator.
- Automatic light/dark theme, restrained animations, vortex logo.
- Session, window size and position persisted.

## How it's built

| File | Role |
| --- | --- |
| `src/main.rs` | event loop, bootstrap, WebView2 flags |
| `src/app.rs` | state: tabs, activation, navigation, layout, overlays, IPC, favicons, window controls |
| `src/assets.rs` | serves the Next export to the chrome webview through the custom `app://` protocol |
| `src/internal.rs` | loopback HTTP server: internal pages, JSON APIs, search redirects |
| `src/data.rs` | settings, history, bookmarks, downloads, encrypted vault |
| `src/engines.rs` | search engine table and `!prefix` syntax |
| `src/importer.rs` | reads bookmarks from installed Chromium profiles |
| `src/defaults.rs` | registers Deeps as the default browser in Windows |
| `src/downloads.rs` | download destinations, unique names, folder picker, opening files and folders |
| `src/urlbar.rs` | normalizes input: URL, host, localhost, path, search, `deeps://` |
| `src/ipc.rs` | page bridge: per-tab token, shortcuts, resize, context menu, translator, drag |
| `src/adblock.rs` | host blocklist and WebView2 arguments |
| `src/logo.rs` | vortex SVG logo and procedural native icon |
| `src/store.rs` | session, size and position in `session.json` |
| `src/startpage.rs` | new tab page helpers |
| `internal/` | HTML/CSS/JS of the internal pages (translator and VPN included) |
| `chrome/` | Next.js 16 UI (App Router, static export) |

Choices worth knowing:

- **Two webviews in the same native window.** The chrome is not an overlay on the page: it
  is a separate webview, so a site cannot touch or read it.
- **Pages get no privileged APIs.** The only channel is `ipc` with a random per-tab token,
  injected before the page scripts: it can only forward whitelisted actions.
- **Internal pages on loopback.** A real origin (`http://127.0.0.1:<ephemeral port>`) avoids
  `about:blank` problems and enables APIs with token and Origin checks.
- **Push state.** Rust sends the whole state to the chrome with `evaluate_script`; commands
  come back as requests with an `id` and a response.
- **Lazy tabs.** A tab's webview is created only on first activation.
- **`CHROME_HEIGHT` (88) is duplicated in `src/app.rs` and in the CSS.** If you change the
  toolbar height, update both.

## Internal pages

| Address | Page |
| --- | --- |
| `deeps://start` | new tab |
| `deeps://history` | history |
| `deeps://bookmarks` | bookmarks |
| `deeps://downloads` | downloads |
| `deeps://passwords` | passwords |
| `deeps://translate` | full-page translator |
| `deeps://vpn` | free VPN (Cloudflare WARP) |
| `deeps://settings` | settings |

The address bar shows `deeps://...`, but the content lives on the local loopback server.
Internal pages cannot be bookmarked.

## Search and `!prefix`

| Prefix | Engine |
| --- | --- |
| `!d` | DuckDuckGo |
| `!g` | Google |
| `!b` | Bing |
| `!s` | Startpage |
| `!y` | Yahoo |
| `!e` | Ecosia |
| `!q` | Qwant |
| `!w` | Wikipedia |

`!g term` searches Google, `!w` alone opens the Wikipedia home page. If the prefix does not
exist, the whole text is searched with the default engine and a "we don't support this
engine" message appears.

### Site bangs

Site bangs are longer than one letter and are checked **before** engines, so `!gh rust` is
not parsed as `!g` followed by `h rust`.

| Bang | Site | Bang | Site | Bang | Site |
| --- | --- | --- | --- | --- | --- |
| `!gh` | GitHub | `!yt` | YouTube | `!rd` | Reddit |
| `!tw` | X | `!ig` | Instagram | `!so` | Stack Overflow |
| `!md` | MDN | `!npm` | npm | `!cr` | crates.io |
| `!am` | Amazon | `!wb` | Wikipedia IT | `!nyt` | New York Times |
| `!bbc` | BBC | `!cnn` | CNN | `!espn` | ESPN |
| `!imdb` | IMDb | `!sp` | Spotify | `!nf` | Netflix |
| `!tg` | Telegram | `!li` | LinkedIn | `!fb` | Facebook |
| `!pin` | Pinterest | `!maps` | Google Maps | `!drive` | Google Drive |
| `!gpt` | ChatGPT | | | | |

With the bang alone the site home page opens; with text after the bang the site's own search
opens (where available).

## Shortcuts

Shortcuts can be enabled from settings (and from the menu footer). With shortcuts disabled,
only `Esc` to close the menu and the explicit UI commands remain.

| Key | Action |
| --- | --- |
| `Ctrl+T` | new tab |
| `Ctrl+W` | close tab |
| `Ctrl+L` | address bar |
| `Ctrl+R` | reload |
| `Ctrl+K` | menu / extensions |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | next / previous tab |
| `Ctrl+Shift+I` | page DevTools |
| `Alt+Left` / `Alt+Right` | back / forward |

## Vertical tabs and appearance

Settings let you choose the **horizontal** or **vertical** layout. In vertical mode the
sidebar is 220px wide and the icon button next to the logo enables auto-hide: when the
pointer leaves the tab area the bar shrinks to 56px and only favicons remain; moving the
mouse back in expands it to 220px after 150 ms, so a quick pass does not open it. The
transition takes 180 ms and is animated both in the chrome (CSS) and in the native edge of
the page, which is resized by Rust.

The same panel changes tab shape (rounded, square, pill), accent colour and wallpaper.
Auto-hide is only a session state: it is not saved in the profile.

Wallpapers include **Personal photo**: pick an image from settings; it is resized in the
browser (max 1920px, JPEG) and stored as a data URL in the profile. The chrome and internal
pages show it behind a dark veil so text stays readable; the chrome loads it from the
internal `/wallpaper` route instead of carrying it in the state.

## Downloads

The toolbar download icon always opens a **quick panel** under the toolbar (not the full
page), both with tabs on top and with the vertical sidebar: it shows the latest files, the
percentage of running operations, a button to remove an entry and, at the bottom, **Show all
downloads**, which opens `deeps://downloads`.

Saving does not use the native WebView2 dialog: Deeps intercepts `DownloadStarting` and
shows a window with the file name, the proposed folder and the buttons Choose folder /
Cancel / Save. With "Always save here" the folder becomes the default and later requests
are not shown again (they can be re-enabled from settings). An empty folder means
`%USERPROFILE%\Downloads`.

Page `alert`, `confirm` and `prompt` no longer show the grey WebView2 window either: Deeps
disables the default dialogs, intercepts `ScriptDialogOpening` and draws them in the chrome
card (origin, message, text field for `prompt`). The page stays blocked until you choose OK
or Cancel; closing the tab cancels the pending dialog. `beforeunload` is accepted
automatically, because Deeps closes tabs without asking for confirmation.

## Language

The UI is available in **English** (default) and **Italian**. You choose it on first launch
or from `deeps://settings`; changing it reloads the internal pages to apply it immediately.
The Next.js chrome uses an explicit dictionary (`chrome/lib/i18n.ts`), while internal pages
start from Italian strings and are translated on the fly into English. A profile created
before the language feature uses English.

## Bookmark import and default browser

From `deeps://settings`, from the `Ctrl+K` menu or from the last onboarding step:

- **Import** reads the `Bookmarks` files of installed Chromium profiles (Chrome, Edge,
  Brave, Vivaldi, Chromium, Opera and Opera GX). Duplicates are skipped and the summary
  reports how many bookmarks were added. Firefox uses SQLite and is not read.
- **Make default** writes per-user keys under `HKCU` and opens the Windows *Default apps*
  page. Windows still asks for confirmation: no program can become the default silently.
  URLs passed on the command line are opened in a new tab.

## Translator

The toolbar and the menu (`Ctrl+K`) open **Translate page**: a popup inside the page with
the source language (default: detect) and the target language (default: Italian). Pressing
Translate:

- visible text is collected and translated in batches, without touching forms, code, inputs
  or editable areas;
- later page changes are observed and translated too;
- **Restore** puts the original text back.

Settings keep the default languages and `deeps://translate` offers the same feature
full-page. Requests go through the Rust process (no CORS/CSP in the page) and use the public
Google Translate service: an Internet connection is required and the page text is sent to
the service. Batches are spaced out to reduce rate limits; if the service returns 429 the
popup says so instead of presenting a partial translation as complete.

## Free VPN

`deeps://vpn` and the VPN panel show the real state of **Cloudflare WARP** and let you
connect and disconnect. Deeps does not wrap its own VPN: it uses the official `warp-cli`
client. If WARP is not installed the page says so and points to the download, without
pretending a connection is active.

## Passwords, logins and passkeys

Login saving is **opt-in**: when a form is submitted the page shows a prompt and credentials
are forwarded only after clicking Save. If the vault does not exist or is locked, the UI
explains what to do instead of saving in plain text.

Passkeys remain managed by the platform authenticator (Windows Hello/WebView2): Deeps does
not export or duplicate them, it only reports whether the authenticator is available.

## Data and privacy

Everything stays on the computer, in `%APPDATA%\Deeps\`:

| File | Contents |
| --- | --- |
| `space.json` | settings (including download folder and background photo), history (max 4000 entries), bookmarks, downloads |
| `vault.bin` | encrypted credentials |
| `session.json` | open tabs, window size and position |

The vault uses **Argon2id** (m=19456, t=2, p=1) to derive the key and
**XChaCha20-Poly1305** to encrypt. The password is never stored. Credentials are available
only while the vault is unlocked and the UI only allows copying, not revealing them in
clear text. Credentials collected from forms are sent to Deeps only after explicit
confirmation in the prompt.

The translator sends the page's visible text to Google Translate: it is a network call like
the site's own, but worth knowing. The VPN adds no telemetry of its own: it uses `warp-cli`
and only shows the state reported by the client.

The adblock turns a curated blocklist into WebView2 `--host-resolver-rules`: DNS for ad
requests is not resolved. That is why the toggle requires an app restart.

Data from the old `Deepy` installation is read as a fallback.

## Requirements

- Windows 10/11 with **WebView2 Runtime** (preinstalled on Windows 11 and with a recent Edge)
- **Rust** stable, MSVC toolchain
- **Node** 20.9+ to build the UI

## Commands

```powershell
# UI (once, and after every chrome change)
npm --prefix chrome install
npm --prefix chrome run build      # writes chrome/out, embedded at the next cargo build

# App
cargo run                          # debug
cargo run --release

# Tests
cargo test
```

UI development with hot reload, inside the real app:

```powershell
npm --prefix chrome run dev
$env:DEEPS_DEV_URL = "http://localhost:3000"
cargo run
```

Without Rust, `npm --prefix chrome run dev` opens the UI at `http://localhost:3000` with a
fake bridge (`chrome/lib/ipc.ts`): tabs, address bar, menu and pages are clickable with demo
data.

## Memory

Memory use is kept low with lazy tabs, `--process-per-site` and
`--renderer-process-limit=4`; closed tabs free the webview. The goal is to stay under
~500 MB with normal use of 5-10 tabs.

## Known limitations

- WebView2 does not support WebExtensions: the extensions menu shows the built-in adblock and
  says so explicitly.
- The adblock works at host level, not cosmetically: it does not remove the blank spaces left
  by ads.
- Resizing from the top edge is handled by the chrome; the left, right and bottom edges also
  by the pages. In vertical layout the left edge does not resize because it belongs to the
  sidebar.
- The translator needs Internet and depends on the public Google service: with many requests
  in a row it may answer 429. The popup then shows the error and you can retry.
- The VPN requires **Cloudflare WARP** installed (`warp-cli` in `PATH` or in the standard
  location). Without the client the VPN page stays honest and does not fake a connected
  state.
- Passkeys are delegated to Windows/WebView2; Deeps cannot import, export or sync them.
- Login saving does not intercept OAuth flows or forms inside cross-origin iframes.

## License

Deeps is released under the **GNU Affero General Public License v3.0**: see
[LICENSE](LICENSE). You can use, modify and redistribute it under the terms of the AGPL,
which requires making the source of distributed versions (including modified ones)
available.

If you want to use Deeps in a closed-source product or in a branded browser without the AGPL
obligations, you need a **commercial license**: see
[LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md).

## Contributing

Bugs, ideas and pull requests are welcome: read [CONTRIBUTING.md](CONTRIBUTING.md).
For security issues do not open a public issue: see [SECURITY.md](SECURITY.md).
