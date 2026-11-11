use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
#[cfg(windows)]
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tao::dpi::{LogicalPosition, LogicalSize};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Icon, ResizeDirection, Theme, Window, WindowBuilder};
use wry::dpi::{LogicalPosition as RectPosition, LogicalSize as RectSize};
use wry::{NewWindowResponse, PageLoadEvent, Rect, WebView, WebViewBuilder};

#[cfg(windows)]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2_4, ICoreWebView2Deferral, ICoreWebView2DownloadOperation,
    ICoreWebView2DownloadStartingEventArgs, ICoreWebView2ScriptDialogOpeningEventArgs,
    COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED, COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED,
    COREWEBVIEW2_SCRIPT_DIALOG_KIND_ALERT, COREWEBVIEW2_SCRIPT_DIALOG_KIND_BEFOREUNLOAD,
    COREWEBVIEW2_SCRIPT_DIALOG_KIND_CONFIRM, COREWEBVIEW2_SCRIPT_DIALOG_KIND_PROMPT,
};
#[cfg(windows)]
use webview2_com::{
    BytesReceivedChangedEventHandler, DownloadStartingEventHandler, ScriptDialogOpeningEventHandler,
    StateChangedEventHandler,
};
#[cfg(windows)]
use windows::core::{Interface, PCWSTR, PWSTR};
#[cfg(windows)]
use windows::Win32::System::Com::CoTaskMemFree;
#[cfg(windows)]
use wry::WebViewExtWindows;
#[cfg(windows)]
use tao::platform::windows::WindowExtWindows;

use crate::data::{Data, Settings};
use crate::{adblock, defaults, engines, importer, internal, ipc, logo, store, urlbar};

/// Height of the chrome webview: 40px tab strip + 48px toolbar.
pub const CHROME_HEIGHT: f64 = 88.0;
/// Width of the vertical tab sidebar. Kept in sync with `--sidebar` in the
/// chrome CSS.
pub const SIDEBAR_WIDTH: f64 = 220.0;
/// Width of the vertical tab sidebar when it auto-collapses to favicons.
pub const SIDEBAR_COLLAPSED_WIDTH: f64 = 56.0;
/// Extra chrome height used by the minimized downloads panel. Kept in sync
/// with `--panel-height` in the chrome CSS.
pub const DOWNLOADS_PANEL_HEIGHT: f64 = 312.0;
const CHROME_URL: &str = "http://app.localhost/index.html";
const SYNC_INTERVAL: Duration = Duration::from_millis(350);
/// How long the vertical sidebar takes to slide between favicons and labels.
const SIDEBAR_ANIM: Duration = Duration::from_millis(180);
/// Frame budget while the sidebar is moving. The native page edge cannot be
/// animated with CSS, so Rust drives the bounds and the chrome fades in step.
const ANIM_FRAME: Duration = Duration::from_millis(16);
const FAVICON_SCRIPT: &str = r#"(function () {
  try {
    var best = "";
    var links = document.querySelectorAll('link[rel~="icon" i], link[rel="shortcut icon" i], link[rel="apple-touch-icon" i], link[rel="mask-icon" i]');
    for (var i = 0; i < links.length; i++) {
      var raw = links[i].getAttribute("href");
      if (!raw) continue;
      var href;
      try { href = new URL(raw, location.href).href; } catch (error) { continue; }
      if (/^data:image\//i.test(href)) { best = href; break; }
      if (/^https?:/i.test(href) && !best) best = href;
    }
    if (!best && /^https?:/i.test(location.href)) best = location.origin + "/favicon.ico";
    return { page: location.href, icon: best };
  } catch (error) {
    return { page: location.href, icon: "" };
  }
})()"#;

#[derive(Clone, Debug)]
pub enum AppEvent {
    Ipc(String),
    Shortcut {
        tab: u64,
        action: String,
    },
    Resize {
        tab: u64,
        direction: String,
    },
    NewWindow {
        url: String,
    },
    Title {
        tab: u64,
        title: String,
    },
    Page {
        tab: u64,
        url: String,
        finished: bool,
    },
    Favicon {
        tab: u64,
        page: String,
        icon: String,
    },
    OpenUrl {
        url: String,
        new_tab: bool,
    },
    Search(String),
    Translate(String),
    PageTranslate {
        tab: u64,
        id: u64,
        from: String,
        to: String,
        texts: Vec<String>,
    },
    Translated {
        tab: u64,
        id: u64,
        translations: Vec<String>,
        error: Option<String>,
    },
    TranslatePrefs {
        from: String,
        to: String,
    },
    SaveLogin {
        tab: u64,
        site: String,
        username: String,
        password: String,
    },
    /// A native download is waiting for the chrome to choose a folder.
    DownloadPrompt {
        id: u64,
        url: String,
        name: String,
        folder: String,
    },
    /// A page opened `alert`, `confirm` or `prompt`: WebView2's own dialog was
    /// suppressed and the chrome shows the Deeps card instead.
    ScriptDialog,
    /// Live byte counters changed (memory only, no persistence).
    DownloadProgress,
    /// A download finished, failed or was cancelled.
    DownloadState,
}

/// Next identifier handed to a pending (unconfirmed) download.
static NEXT_PENDING: AtomicU64 = AtomicU64::new(1);

/// A download paused on `ICoreWebView2DownloadStartingEventArgs::GetDeferral`
/// while the user picks a folder in the chrome UI. WebView2 callbacks run on
/// the UI thread and COM interfaces are not `Send`, so the queue is
/// thread-local.
#[cfg(windows)]
struct PendingDownload {
    id: u64,
    url: String,
    name: String,
    folder: String,
    args: ICoreWebView2DownloadStartingEventArgs,
    deferral: ICoreWebView2Deferral,
}

#[cfg(windows)]
thread_local! {
    static PENDING_DOWNLOADS: RefCell<VecDeque<PendingDownload>> = RefCell::new(VecDeque::new());
    static PENDING_DIALOGS: RefCell<VecDeque<PendingDialog>> = RefCell::new(VecDeque::new());
    static LAST_PROGRESS: Cell<Option<Instant>> = Cell::new(None);
}

/// Next identifier handed to a pending page dialog.
static NEXT_DIALOG: AtomicU64 = AtomicU64::new(1);

/// A page dialog (`alert` / `confirm` / `prompt`) paused while the chrome
/// shows its own card. The page's JavaScript stays blocked until the user
/// answers, so the WebView2 event arguments are kept alive in a thread-local
/// queue, exactly like downloads.
#[cfg(windows)]
struct PendingDialog {
    id: u64,
    tab: u64,
    /// `alert`, `confirm` or `prompt`.
    kind: String,
    message: String,
    default_text: String,
    origin: String,
    args: ICoreWebView2ScriptDialogOpeningEventArgs,
    deferral: ICoreWebView2Deferral,
}

/// Attaches the native `ScriptDialogOpening` handler to a page webview, so
/// `alert`, `confirm` and `prompt` never show the gray WebView2 window.
#[cfg(windows)]
fn attach_dialogs(view: &WebView, tab: u64, proxy: EventLoopProxy<AppEvent>) {
    // WebView2 raises `ScriptDialogOpening` only once its own gray dialogs are
    // turned off; otherwise it draws them and never tells the host.
    if let Ok(core) = unsafe { view.controller().CoreWebView2() } {
        if let Ok(settings) = unsafe { core.Settings() } {
            unsafe {
                let _ = settings.SetAreDefaultScriptDialogsEnabled(false);
            }
        }
    }
    let core = view.webview();
    let handler = ScriptDialogOpeningEventHandler::create(Box::new(move |_sender, args| {
        if let Some(args) = args {
            begin_dialog(&args, tab, &proxy);
        }
        Ok(())
    }));
    let mut token = 0i64;
    unsafe {
        let _ = core.add_ScriptDialogOpening(&handler, &mut token);
    }
}

/// Runs when a page wants to show a script dialog. The deferral keeps
/// WebView2's gray window hidden while the chrome shows the Deeps card;
/// `beforeunload` prompts are accepted automatically (Deeps closes tabs
/// without asking).
#[cfg(windows)]
fn begin_dialog(
    args: &ICoreWebView2ScriptDialogOpeningEventArgs,
    tab: u64,
    proxy: &EventLoopProxy<AppEvent>,
) {
    let Ok(deferral) = (unsafe { args.GetDeferral() }) else {
        return;
    };

    let mut raw_kind = COREWEBVIEW2_SCRIPT_DIALOG_KIND_ALERT;
    unsafe {
        let _ = args.Kind(&mut raw_kind);
    }

    if raw_kind == COREWEBVIEW2_SCRIPT_DIALOG_KIND_BEFOREUNLOAD {
        unsafe {
            let _ = args.Accept();
            let _ = deferral.Complete();
        }
        return;
    }

    let kind = if raw_kind == COREWEBVIEW2_SCRIPT_DIALOG_KIND_CONFIRM {
        "confirm"
    } else if raw_kind == COREWEBVIEW2_SCRIPT_DIALOG_KIND_PROMPT {
        "prompt"
    } else {
        "alert"
    };

    let message = unsafe {
        let mut raw = PWSTR::null();
        args.Message(&mut raw).ok().map(|_| take_pwstr(raw))
    }
    .unwrap_or_default();
    let default_text = unsafe {
        let mut raw = PWSTR::null();
        args.DefaultText(&mut raw).ok().map(|_| take_pwstr(raw))
    }
    .unwrap_or_default();
    let origin = unsafe {
        let mut raw = PWSTR::null();
        args.Uri(&mut raw).ok().map(|_| take_pwstr(raw))
    }
    .unwrap_or_default();

    let id = NEXT_DIALOG.fetch_add(1, Ordering::Relaxed);
    PENDING_DIALOGS.with(|queue| {
        queue.borrow_mut().push_back(PendingDialog {
            id,
            tab,
            kind: kind.to_string(),
            message,
            default_text,
            origin,
            args: args.clone(),
            deferral,
        });
    });
    let _ = proxy.send_event(AppEvent::ScriptDialog);
}

/// Cancels every dialog opened by a tab that is going away; completing the
/// deferral makes the blocked JavaScript continue as if cancelled.
#[cfg(windows)]
fn dismiss_dialogs(tab: u64) {
    PENDING_DIALOGS.with(|queue| {
        let mut queue = queue.borrow_mut();
        let mut index = 0;
        while index < queue.len() {
            if queue[index].tab == tab {
                let dialog = queue.remove(index);
                if let Some(dialog) = dialog {
                    unsafe {
                        let _ = dialog.deferral.Complete();
                    }
                }
            } else {
                index += 1;
            }
        }
    });
}


/// Attaches the native `DownloadStarting` handler to a page webview.
#[cfg(windows)]
fn attach_downloads(view: &WebView, data: Arc<Mutex<Data>>, proxy: EventLoopProxy<AppEvent>) {
    let Ok(core) = view.webview().cast::<ICoreWebView2_4>() else {
        return;
    };
    let handler = DownloadStartingEventHandler::create(Box::new(move |_sender, args| {
        if let Some(args) = args {
            begin_download(&args, &data, &proxy);
        }
        Ok(())
    }));
    let mut token = 0i64;
    unsafe {
        let _ = core.add_DownloadStarting(&handler, &mut token);
    }
}

/// Runs when a page starts a download. Always suppresses WebView2's own save
/// dialog; either starts the file immediately in the configured folder or
/// defers the operation until the chrome picks one.
#[cfg(windows)]
fn begin_download(
    args: &ICoreWebView2DownloadStartingEventArgs,
    data: &Arc<Mutex<Data>>,
    proxy: &EventLoopProxy<AppEvent>,
) {
    unsafe {
        let _ = args.SetHandled(true);
    }

    let operation = unsafe { args.DownloadOperation().ok() };
    let url = operation
        .as_ref()
        .and_then(|operation| unsafe {
            let mut raw = PWSTR::null();
            operation.Uri(&mut raw).ok().map(|_| take_pwstr(raw))
        })
        .unwrap_or_default();
    let proposed = operation
        .as_ref()
        .and_then(|operation| unsafe {
            let mut raw = PWSTR::null();
            operation.ResultFilePath(&mut raw).ok().map(|_| take_pwstr(raw))
        })
        .unwrap_or_default();
    let name = Path::new(&proposed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download")
        .to_string();

    let (ask, configured) = data
        .lock()
        .map(|state| (state.settings.ask_download_dir, state.settings.download_dir.clone()))
        .unwrap_or((true, String::new()));
    let folder = crate::downloads::destination_dir(&configured);

    if ask {
        let Ok(deferral) = (unsafe { args.GetDeferral() }) else {
            return;
        };
        let id = NEXT_PENDING.fetch_add(1, Ordering::Relaxed);
        let folder_text = folder.to_string_lossy().into_owned();
        PENDING_DOWNLOADS.with(|queue| {
            queue.borrow_mut().push_back(PendingDownload {
                id,
                url: url.clone(),
                name: name.clone(),
                folder: folder.to_string_lossy().into_owned(),
                args: args.clone(),
                deferral,
            });
        });
        let _ = proxy.send_event(AppEvent::DownloadPrompt {
            id,
            url,
            name,
            folder: folder_text,
        });
    } else {
        let destination = crate::downloads::prepare_in(&folder, &name);
        set_result_path(args, &destination);
        let mut id = 0;
        if let Ok(mut state) = data.lock() {
            id = state.start_download(&url, &destination);
        }
        if let Some(operation) = operation {
            attach_operation(&operation, id, data.clone(), proxy.clone());
        }
        let _ = proxy.send_event(AppEvent::DownloadState);
    }
}

#[cfg(windows)]
fn set_result_path(args: &ICoreWebView2DownloadStartingEventArgs, path: &Path) {
    let text = path.to_string_lossy();
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = args.SetResultFilePath(PCWSTR::from_raw(wide.as_ptr()));
    }
}

/// Watches byte and state changes for one download.
#[cfg(windows)]
fn attach_operation(
    operation: &ICoreWebView2DownloadOperation,
    id: u64,
    data: Arc<Mutex<Data>>,
    proxy: EventLoopProxy<AppEvent>,
) {
    let progress_data = data.clone();
    let progress_proxy = proxy.clone();
    let bytes = BytesReceivedChangedEventHandler::create(Box::new(move |operation, _| {
        if let Some(operation) = operation {
            let (mut received, mut total) = (0i64, 0i64);
            unsafe {
                let _ = operation.BytesReceived(&mut received);
                let _ = operation.TotalBytesToReceive(&mut total);
            }
            if let Ok(mut state) = progress_data.lock() {
                state.progress_download(id, received, total);
            }
            let due = LAST_PROGRESS.with(|last| {
                let now = Instant::now();
                match last.get() {
                    Some(previous) if now.duration_since(previous) < Duration::from_millis(120) => {
                        false
                    }
                    _ => {
                        last.set(Some(now));
                        true
                    }
                }
            });
            if due {
                let _ = progress_proxy.send_event(AppEvent::DownloadProgress);
            }
        }
        Ok(())
    }));
    let mut token = 0i64;
    unsafe {
        let _ = operation.add_BytesReceivedChanged(&bytes, &mut token);
    }

    let state_data = data;
    let state_proxy = proxy;
    let changed = StateChangedEventHandler::create(Box::new(move |operation, _| {
        let Some(operation) = operation else {
            return Ok(());
        };
        let mut state = Default::default();
        unsafe {
            let _ = operation.State(&mut state);
        }
        if state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED
            || state == COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED
        {
            let ok = state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED;
            if let Ok(mut data) = state_data.lock() {
                data.finish_download(id, ok);
            }
            let _ = state_proxy.send_event(AppEvent::DownloadState);
        }
        Ok(())
    }));
    let mut token = 0i64;
    unsafe {
        let _ = operation.add_StateChanged(&changed, &mut token);
    }
}

/// Takes ownership of a COM-allocated string, copying it and freeing the
/// original buffer.
#[cfg(windows)]
fn take_pwstr(raw: PWSTR) -> String {
    if raw.is_null() {
        return String::new();
    }
    let text = unsafe { raw.to_string() }.unwrap_or_default();
    unsafe {
        CoTaskMemFree(Some(raw.0 as *const _));
    }
    text
}

struct Tab {
    id: u64,
    webview: Option<WebView>,
    url: String,
    title: String,
    icon: Option<String>,
    loading: bool,
    /// Set until the first page load finishes, so keyboard focus lands on the
    /// page instead of staying in the chrome.
    needs_focus: bool,
}

struct SidebarAnim {
    from: f64,
    to: f64,
    start: Instant,
}

pub struct App {
    window: Window,
    chrome: WebView,
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    proxy: EventLoopProxy<AppEvent>,
    start_url: String,
    data: Arc<Mutex<Data>>,
    cached_settings: Settings,
    overlay: bool,
    last_maximized: bool,
    last_state: String,
    dirty: bool,
    deadline: Instant,
    sidebar_collapsed: bool,
    sidebar_width: f64,
    sidebar_anim: Option<SidebarAnim>,
    /// Minimized download panel under the toolbar.
    downloads_panel: bool,
}

impl App {
    pub fn new(
        target: &EventLoopWindowTarget<AppEvent>,
        proxy: EventLoopProxy<AppEvent>,
        initial_url: Option<String>,
    ) -> Result<Self, String> {
        let data = Arc::new(Mutex::new(Data::load()));
        let settings = data
            .lock()
            .map(|state| state.settings.clone())
            .unwrap_or_default();

        // Must happen before the first WebView2 controller exists.
        adblock::apply(settings.adblock);

        let session = store::load();
        let (width, height) = session
            .as_ref()
            .map(|saved| saved.window)
            .unwrap_or((1280.0, 820.0));
        let position = session
            .as_ref()
            .and_then(|saved| saved.position)
            .filter(|(x, y)| *x > -4000.0 && *y > -4000.0 && *x < 20_000.0 && *y < 20_000.0);

        let mut builder = WindowBuilder::new()
            .with_title("Deeps")
            .with_decorations(false)
            .with_resizable(true)
            .with_background_color((13, 13, 15, 255))
            .with_inner_size(LogicalSize::new(
                width.clamp(760.0, 3200.0),
                height.clamp(520.0, 2200.0),
            ))
            .with_min_inner_size(LogicalSize::new(760.0, 520.0));

        if let Some((x, y)) = position {
            builder = builder.with_position(LogicalPosition::new(x, y));
        }
        if let Ok(icon) = Icon::from_rgba(logo::icon_rgba(64), 64, 64) {
            builder = builder.with_window_icon(Some(icon));
        }

        let window = builder.build(target).map_err(|error| error.to_string())?;
        #[cfg(windows)]
        crate::downloads::set_owner_hwnd(window.hwnd() as isize);
        let port = internal::serve(data.clone()).map_err(|error| error.to_string())?;
        let start_url = format!("http://127.0.0.1:{port}/");

        let chrome_proxy = proxy.clone();
        let chrome = WebViewBuilder::new()
            .with_custom_protocol("app".to_string(), |_id, request| {
                crate::assets::respond(request)
            })
            .with_ipc_handler(move |request| {
                let _ = chrome_proxy.send_event(AppEvent::Ipc(request.body().clone()));
            })
            .with_url(chrome_url())
            .with_background_color(background_for(window.theme() == Theme::Dark))
            .with_bounds(Rect {
                position: RectPosition::new(0.0, 0.0).into(),
                size: RectSize::new(logical_size(&window).0, CHROME_HEIGHT).into(),
            })
            .with_devtools(true)
            .build_as_child(&window)
            .map_err(|error| error.to_string())?;

        let mut app = App {
            window,
            chrome,
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            proxy,
            start_url,
            data,
            cached_settings: settings,
            overlay: false,
            last_maximized: false,
            last_state: String::new(),
            dirty: false,
            deadline: Instant::now() + SYNC_INTERVAL,
            sidebar_collapsed: false,
            sidebar_width: SIDEBAR_WIDTH,
            sidebar_anim: None,
            downloads_panel: false,
        };
        app.last_maximized = app.window.is_maximized();

        let (urls, active) = match session {
            Some(saved) => (
                saved
                    .urls
                    .into_iter()
                    .filter(|url| {
                        url.starts_with("http") && !app.is_internal(url) && url != "about:blank"
                    })
                    .take(32)
                    .collect::<Vec<_>>(),
                saved.active,
            ),
            None => (Vec::new(), 0),
        };

        if urls.is_empty() {
            app.new_tab(app.start_url.clone(), true)?;
        } else {
            for url in urls {
                app.add_tab(url);
            }
            let index = active.min(app.tabs.len().saturating_sub(1));
            app.activate(index);
        }

        // Opened through the Windows "default browser" association (or any
        // command line URL): load it in a fresh tab and focus it.
        if let Some(target) = initial_url {
            let resolved = urlbar::resolve(&target, &app.start_url, &app.engine());
            let _ = app.new_tab(resolved.url, true);
        }

        Ok(app)
    }

    pub fn handle_event(&mut self, event: Event<AppEvent>, control_flow: &mut ControlFlow) {
        match &event {
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                if Instant::now() >= self.deadline {
                    self.sync();
                    self.deadline = Instant::now() + SYNC_INTERVAL;
                }
            }
            Event::MainEventsCleared => self.step_sidebar(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    self.layout();
                    let maximized = self.window.is_maximized();
                    if maximized != self.last_maximized {
                        self.last_maximized = maximized;
                        self.dirty = true;
                        self.push_state();
                    }
                    self.persist();
                }
                WindowEvent::Moved(_) => {
                    self.persist();
                }
                WindowEvent::ThemeChanged(theme) => {
                    self.set_window_background(*theme == Theme::Dark);
                }
                WindowEvent::CloseRequested => {
                    self.persist();
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                _ => {}
            },
            Event::UserEvent(user) => self.on_user_event(user.clone()),
            Event::LoopDestroyed => self.persist(),
            _ => {}
        }

        let mut wake = self.deadline;
        if self.sidebar_anim.is_some() {
            wake = wake.min(Instant::now() + ANIM_FRAME);
        }
        *control_flow = ControlFlow::WaitUntil(wake);
    }

    /* ------------------------------------------------------------------ */
    /* tabs                                                                */
    /* ------------------------------------------------------------------ */

    fn add_tab(&mut self, url: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.tabs.push(Tab {
            id,
            webview: None,
            url,
            title: String::new(),
            icon: None,
            loading: false,
            needs_focus: true,
        });
        self.dirty = true;
        id
    }

    fn new_tab(&mut self, url: String, activate: bool) -> Result<u64, String> {
        let id = self.add_tab(url);
        if activate {
            let index = self.tabs.len().saturating_sub(1);
            self.activate(index);
        }
        Ok(id)
    }

    fn create_webview(&self, id: u64, url: &str) -> Result<WebView, String> {
        let token = ipc::new_token(id);
        let (shortcuts, save_logins, vertical, translate_from, translate_to) = self
            .data
            .lock()
            .map(|state| {
                (
                    state.settings.shortcuts,
                    state.settings.save_logins,
                    state.settings.tab_layout == "vertical",
                    state.settings.translate_from.clone(),
                    state.settings.translate_to.clone(),
                )
            })
            .unwrap_or_else(|_| (true, true, false, "auto".to_string(), "it".to_string()));
        let bounds = self.content_bounds();

        let shortcut_proxy = self.proxy.clone();
        let shortcut_token = token.clone();
        let resize_proxy = self.proxy.clone();
        let window_proxy = self.proxy.clone();
        let page_proxy = self.proxy.clone();
        let title_proxy = self.proxy.clone();

        let webview = WebViewBuilder::new()
            .with_url(url)
            .with_bounds(bounds)
            .with_visible(false)
            .with_background_color(background_for(self.window.theme() == Theme::Dark))
            .with_initialization_script(ipc::page_script(
                &token,
                shortcuts,
                save_logins,
                vertical,
                &translate_from,
                &translate_to,
            ))
            .with_ipc_handler(move |request| {
                let body = request.body();
                if let Some(message) = ipc::parse(body, &shortcut_token) {
                    match message {
                        ipc::Message::Action(action) => {
                            let _ = shortcut_proxy.send_event(AppEvent::Shortcut {
                                tab: id,
                                action: action.to_string(),
                            });
                        }
                        ipc::Message::Resize(direction) => {
                            let _ = resize_proxy.send_event(AppEvent::Resize {
                                tab: id,
                                direction: direction.to_string(),
                            });
                        }
                        ipc::Message::OpenUrl { url, new_tab } => {
                            let _ = shortcut_proxy.send_event(AppEvent::OpenUrl { url, new_tab });
                        }
                        ipc::Message::Search(text) => {
                            let _ = shortcut_proxy.send_event(AppEvent::Search(text));
                        }
                        ipc::Message::Translate(text) => {
                            let _ = shortcut_proxy.send_event(AppEvent::Translate(text));
                        }
                        ipc::Message::PageTranslate {
                            id: request_id,
                            from,
                            to,
                            texts,
                        } => {
                            let _ = shortcut_proxy.send_event(AppEvent::PageTranslate {
                                tab: id,
                                id: request_id,
                                from,
                                to,
                                texts,
                            });
                        }
                        ipc::Message::TranslatePrefs { from, to } => {
                            let _ = shortcut_proxy.send_event(AppEvent::TranslatePrefs { from, to });
                        }
                        ipc::Message::SaveLogin {
                            site,
                            username,
                            password,
                        } => {
                            let _ = shortcut_proxy.send_event(AppEvent::SaveLogin {
                                tab: id,
                                site,
                                username,
                                password,
                            });
                        }
                    }
                }
            })
            .with_new_window_req_handler(move |url, _features| {
                let _ = window_proxy.send_event(AppEvent::NewWindow { url });
                NewWindowResponse::Deny
            })
            .with_on_page_load_handler(move |event, url| {
                let _ = page_proxy.send_event(AppEvent::Page {
                    tab: id,
                    url,
                    finished: matches!(event, PageLoadEvent::Finished),
                });
            })
            .with_document_title_changed_handler(move |title| {
                let _ = title_proxy.send_event(AppEvent::Title { tab: id, title });
            })
            .with_devtools(true)
            .build_as_child(&self.window)
            .map_err(|error| error.to_string())?;

        // Downloads are handled natively through WebView2 so Deeps can replace
        // the built-in save dialog with its own folder chooser.
        #[cfg(windows)]
        attach_downloads(&webview, self.data.clone(), self.proxy.clone());
        // Same idea for `alert` / `confirm` / `prompt`: the page stays blocked
        // while the chrome shows a Deeps-styled card.
        #[cfg(windows)]
        attach_dialogs(&webview, id, self.proxy.clone());
        #[cfg(not(windows))]
        let _ = &webview;

        Ok(webview)
    }

    fn ensure_webview(&mut self, index: usize) -> bool {
        if let Some(Some(_)) = self.tabs.get(index).map(|tab| tab.webview.as_ref()) {
            return true;
        }
        let Some(tab) = self.tabs.get(index) else {
            return false;
        };
        let id = tab.id;
        let url = tab.url.clone();
        match self.create_webview(id, &url) {
            Ok(webview) => {
                if index == self.active && self.content_visible() {
                    let _ = webview.set_bounds(self.content_bounds());
                    let _ = webview.set_visible(true);
                }
                if let Some(tab) = self.tabs.get_mut(index) {
                    tab.webview = Some(webview);
                }
                true
            }
            Err(error) => {
                eprintln!("deeps: cannot create webview: {error}");
                false
            }
        }
    }

    fn activate(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }

        let previous = self.active;
        if previous != index {
            if let Some(view) = self.tabs.get(previous).and_then(|tab| tab.webview.as_ref()) {
                let _ = view.set_visible(false);
            }
        }

        if !self.ensure_webview(index) {
            return;
        }

        let bounds = self.content_bounds();
        self.overlay = false;
        let visible = self.content_visible();
        let title = if let Some(tab) = self.tabs.get(index) {
            if let Some(view) = tab.webview.as_ref() {
                let _ = view.set_bounds(bounds);
                let _ = view.set_visible(visible);
                if visible {
                    let _ = view.focus();
                }
            }
            self.title_of(tab)
        } else {
            "Deeps".to_string()
        };

        self.active = index;
        if !visible {
            let _ = self.chrome.focus();
        }
        let maximized = self.window.is_maximized();
        if maximized != self.last_maximized {
            self.last_maximized = maximized;
        }
        self.layout();
        let _ = self.window.set_title(&format!("{title} — Deeps"));

        self.dirty = true;
        self.push_state();
        self.persist();
    }

    fn close_tab(&mut self, id: u64) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };

        // A page blocked in `confirm()` can still be closed from the chrome:
        // release its dialog first so the JavaScript unblocks and the webview
        // can be destroyed cleanly.
        #[cfg(windows)]
        dismiss_dialogs(id);

        drop(self.tabs.remove(index));

        if self.tabs.is_empty() {
            let _ = self.new_tab(self.start_url.clone(), true);
            return;
        }

        if index < self.active {
            self.active -= 1;
        } else if index >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }

        let next = self.active;
        self.activate(next);
    }

    fn cycle(&mut self, forward: bool) {
        if self.tabs.is_empty() {
            return;
        }
        let len = self.tabs.len();
        let index = if forward {
            (self.active + 1) % len
        } else {
            (self.active + len - 1) % len
        };
        self.activate(index);
    }

    fn move_tab(&mut self, id: u64, before: u64, after: bool) {
        let Some(from) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let Some(target) = self.tabs.iter().position(|tab| tab.id == before) else {
            return;
        };

        let mut to = if after { target + 1 } else { target };
        let active_id = self.active_id();
        let tab = self.tabs.remove(from);
        if from < to {
            to -= 1;
        }
        let to = to.min(self.tabs.len());
        self.tabs.insert(to, tab);
        if let Some(active_id) = active_id {
            self.active = self
                .tabs
                .iter()
                .position(|tab| tab.id == active_id)
                .unwrap_or(0);
        }
        self.dirty = true;
        self.push_state();
        self.persist();
    }

    fn navigate(&mut self, url: String) {
        let index = self.active;
        if !self.ensure_webview(index) {
            return;
        }
        let Some(tab) = self.tabs.get_mut(index) else {
            return;
        };
        if let Some(view) = tab.webview.as_ref() {
            let _ = view.load_url(url.as_str());
            let _ = view.focus();
        }
        tab.url = url;
        tab.loading = true;
        tab.icon = None;
        tab.title.clear();

        self.overlay = false;
        self.layout();
        self.dirty = true;
        self.push_state();
        self.persist();
    }

    fn reload(&mut self) {
        let index = self.active;
        if !self.ensure_webview(index) {
            return;
        }
        if let Some(view) = self.tabs.get(index).and_then(|tab| tab.webview.as_ref()) {
            let _ = view.reload();
        }
        self.overlay = false;
        self.dirty = true;
    }

    fn go_back(&mut self) {
        let index = self.active;
        if !self.ensure_webview(index) {
            return;
        }
        if let Some(view) = self.tabs.get(index).and_then(|tab| tab.webview.as_ref()) {
            let _ = view.go_back();
        }
        self.dirty = true;
    }

    fn go_forward(&mut self) {
        let index = self.active;
        if !self.ensure_webview(index) {
            return;
        }
        if let Some(view) = self.tabs.get(index).and_then(|tab| tab.webview.as_ref()) {
            let _ = view.go_forward();
        }
        self.dirty = true;
    }

    fn open_page(&mut self, page: &str) {
        let page = page.trim().trim_start_matches('/');
        let url = if page.is_empty() {
            self.start_url.clone()
        } else {
            format!("{}{}", self.start_url, page)
        };
        self.overlay = false;
        self.downloads_panel = false;
        self.navigate(url);
        if let Some(view) = self.tabs.get(self.active).and_then(|tab| tab.webview.as_ref()) {
            let _ = view.focus();
        }
    }

    /* ------------------------------------------------------------------ */
    /* events                                                              */
    /* ------------------------------------------------------------------ */

    fn on_user_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Ipc(body) => self.on_ipc(&body),
            AppEvent::Shortcut { tab, action } => {
                if self.active_id() == Some(tab) {
                    let enabled = self
                        .data
                        .lock()
                        .map(|data| data.settings.shortcuts)
                        .unwrap_or(true);
                    if enabled {
                        self.run_shortcut(&action);
                    }
                }
            }
            AppEvent::Resize { tab, direction } => {
                if self.active_id() == Some(tab) {
                    self.resize_window(&direction);
                }
            }
            AppEvent::NewWindow { url } => {
                let _ = self.new_tab(url, true);
            }
            AppEvent::Title { tab, title } => {
                let mut url = String::new();
                if let Some(item) = self.tabs.iter_mut().find(|item| item.id == tab) {
                    item.title = title.trim().chars().take(140).collect();
                    url = item.url.clone();
                }
                if !url.is_empty() {
                    if let Ok(mut data) = self.data.lock() {
                        data.touch_history_title(&url, &title);
                    }
                }
                self.dirty = true;
                self.push_state();
                self.persist();
                if let Some(active) = self.tabs.get(self.active) {
                    let title = self.title_of(active);
                    let _ = self.window.set_title(&format!("{title} — Deeps"));
                }
                self.request_favicon_if_missing(tab);
            }
            AppEvent::Page { tab, url, finished } => {
                let active = self.active_id() == Some(tab);
                let mut focus = false;
                let mut title = String::new();

                if let Some(item) = self.tabs.iter_mut().find(|item| item.id == tab) {
                    item.url = url.clone();
                    item.loading = !finished;
                    if !finished {
                        item.icon = None;
                        item.title.clear();
                    }
                    if finished && active && item.needs_focus {
                        item.needs_focus = false;
                        focus = true;
                    }
                    title = item.title.clone();
                }

                if finished {
                    if let Ok(mut data) = self.data.lock() {
                        if !url.starts_with(&self.start_url) {
                            data.record_history(&url, &title);
                        }
                    }
                    self.request_favicon(tab);
                }

                if focus {
                    if let Some(item) = self.tabs.get(self.active) {
                        if let Some(view) = item.webview.as_ref() {
                            let _ = view.focus();
                        }
                    }
                }

                self.dirty = true;
                self.push_state();
                self.persist();
            }
            AppEvent::Favicon { tab, page, icon } => {
                if icon.is_empty() || icon.len() > 120_000 {
                    return;
                }
                let ok = icon.starts_with("http://")
                    || icon.starts_with("https://")
                    || icon.starts_with("data:image/");
                if !ok {
                    return;
                }
                if let Some(item) = self.tabs.iter_mut().find(|item| item.id == tab) {
                    let same = item.url.trim_end_matches('/') == page.trim_end_matches('/');
                    if same || item.url.is_empty() {
                        item.icon = Some(icon);
                    }
                }
                self.dirty = true;
                self.push_state();
            }
            AppEvent::OpenUrl { url, new_tab } => {
                if new_tab {
                    let _ = self.new_tab(url, true);
                } else {
                    self.navigate(url);
                }
            }
            AppEvent::Search(text) => {
                let url = engines::query(&self.engine(), &text);
                let _ = self.new_tab(url, true);
            }
            AppEvent::Translate(text) => {
                let to = self.current_settings().translate_to;
                let url = format!(
                    "{}translate?text={}&to={}",
                    self.start_url,
                    engines::encode(&text),
                    engines::encode(&to)
                );
                let _ = self.new_tab(url, true);
            }
            AppEvent::PageTranslate {
                tab,
                id,
                from,
                to,
                texts,
            } => {
                let proxy = self.proxy.clone();
                std::thread::spawn(move || {
                    let (translations, error) = match internal::translate_batch(&texts, &from, &to) {
                        Ok(translations) => (translations, None),
                        Err(message) => (Vec::new(), Some(message)),
                    };
                    let _ = proxy.send_event(AppEvent::Translated {
                        tab,
                        id,
                        translations,
                        error,
                    });
                });
            }
            AppEvent::Translated {
                tab,
                id,
                translations,
                error,
            } => {
                let list =
                    serde_json::to_string(&translations).unwrap_or_else(|_| "[]".to_string());
                let failure = match error {
                    Some(message) => serde_json::to_string(&message)
                        .unwrap_or_else(|_| "null".to_string()),
                    None => "null".to_string(),
                };
                let script = format!(
                    "window.__deepsTranslated && window.__deepsTranslated({id}, {list}, {failure})"
                );
                if let Some(view) = self
                    .tabs
                    .iter()
                    .find(|item| item.id == tab)
                    .and_then(|item| item.webview.as_ref())
                {
                    let _ = view.evaluate_script(&script);
                }
            }
            AppEvent::TranslatePrefs { from, to } => {
                let next = {
                    let mut data = match self.data.lock() {
                        Ok(data) => data,
                        Err(_) => return,
                    };
                    match data
                        .settings
                        .patch(&json!({ "translateFrom": from, "translateTo": to }))
                    {
                        Ok(next) => {
                            data.set_settings(next.clone());
                            Some(next)
                        }
                        Err(_) => None,
                    }
                };
                if let Some(next) = next {
                    self.cached_settings = next;
                    self.apply_page_settings();
                    self.dirty = true;
                    self.push_state();
                }
            }
            AppEvent::SaveLogin {
                tab,
                site,
                username,
                password,
            } => {
                let status = {
                    let mut data = match self.data.lock() {
                        Ok(data) => data,
                        Err(_) => return,
                    };
                    if !data.settings.save_logins {
                        "off"
                    } else if !data.vault_exists() {
                        "vault"
                    } else if !data.vault_unlocked() {
                        "locked"
                    } else {
                        match data.upsert_login(&site, &username, &password) {
                            Ok(()) => "ok",
                            Err(_) => "error",
                        }
                    }
                };
                let script = format!(
                    "window.__deepsLoginResult && window.__deepsLoginResult(\"{status}\")"
                );
                if let Some(view) = self
                    .tabs
                    .iter()
                    .find(|item| item.id == tab)
                    .and_then(|item| item.webview.as_ref())
                {
                    let _ = view.evaluate_script(&script);
                }
                self.dirty = true;
                self.push_state();
            }
            AppEvent::DownloadPrompt {
                id,
                url,
                name,
                folder,
            } => {
                self.open_download_prompt(id, url, name, folder);
            }
            AppEvent::ScriptDialog => {
                self.downloads_panel = false;
                self.layout();
                let _ = self.chrome.focus();
                self.dirty = true;
                self.push_state();
            }
            AppEvent::DownloadProgress | AppEvent::DownloadState => {
                self.dirty = true;
                self.push_state();
            }
        }
    }

    fn request_favicon(&self, tab: u64) {
        let Some(view) = self
            .tabs
            .iter()
            .find(|item| item.id == tab)
            .and_then(|item| item.webview.as_ref())
        else {
            return;
        };
        let proxy = self.proxy.clone();
        let _ = view.evaluate_script_with_callback(FAVICON_SCRIPT, move |value| {
            let Ok(parsed) = serde_json::from_str::<Value>(&value) else {
                return;
            };
            let page = parsed
                .get("page")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let icon = parsed
                .get("icon")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let _ = proxy.send_event(AppEvent::Favicon { tab, page, icon });
        });
    }

    fn request_favicon_if_missing(&self, tab: u64) {
        let missing = self
            .tabs
            .iter()
            .find(|item| item.id == tab)
            .map(|item| item.icon.is_none())
            .unwrap_or(false);
        if missing {
            self.request_favicon(tab);
        }
    }

    fn apply_page_settings(&self) {
        let (shortcuts, save_logins, vertical, from, to) = self
            .data
            .lock()
            .map(|data| {
                (
                    data.settings.shortcuts,
                    data.settings.save_logins,
                    data.settings.tab_layout == "vertical",
                    data.settings.translate_from.clone(),
                    data.settings.translate_to.clone(),
                )
            })
            .unwrap_or_else(|_| (true, true, false, "auto".to_string(), "it".to_string()));
        let from = serde_json::to_string(&from).unwrap_or_else(|_| "\"auto\"".to_string());
        let to = serde_json::to_string(&to).unwrap_or_else(|_| "\"it\"".to_string());
        let script = format!(
            "window.__deepsShortcuts = {shortcuts}; window.__deepsSaveLogins = {save_logins}; \
             window.__deepsVertical = {vertical}; window.__deepsTranslateFrom = {from}; \
             window.__deepsTranslateTo = {to}; if (window.__deepsTranslateSync) window.__deepsTranslateSync();"
        );
        for tab in &self.tabs {
            if let Some(view) = tab.webview.as_ref() {
                let _ = view.evaluate_script(&script);
            }
        }
    }

    fn run_shortcut(&mut self, action: &str) {
        match action {
            "new_tab" => {
                let _ = self.new_tab(self.start_url.clone(), true);
            }
            "close_tab" => {
                if let Some(id) = self.active_id() {
                    self.close_tab(id);
                }
            }
            "reload" => self.reload(),
            "back" => self.go_back(),
            "forward" => self.go_forward(),
            "next_tab" => self.cycle(true),
            "prev_tab" => self.cycle(false),
            "menu" => {
                self.set_overlay(!self.overlay);
                if self.overlay {
                    let _ = self.chrome.focus();
                }
            }
            "devtools" => {
                if let Some(view) = self.tabs.get(self.active).and_then(|tab| tab.webview.as_ref()) {
                    let _ = view.open_devtools();
                }
            }
            "focus_address" => {
                let _ = self.chrome.focus();
                let _ = self
                    .chrome
                    .evaluate_script("window.__deeps && window.__deeps.focusAddress()");
            }
            _ => {}
        }
    }

    fn resize_window(&mut self, direction: &str) {
        let direction = match direction {
            "north" => ResizeDirection::North,
            "south" => ResizeDirection::South,
            "east" => ResizeDirection::East,
            "west" => ResizeDirection::West,
            "northeast" => ResizeDirection::NorthEast,
            "northwest" => ResizeDirection::NorthWest,
            "southeast" => ResizeDirection::SouthEast,
            "southwest" => ResizeDirection::SouthWest,
            _ => return,
        };
        let _ = self.window.drag_resize_window(direction);
        self.persist();
    }

    fn set_window_background(&self, dark: bool) {
        let color = if dark {
            (13, 13, 15, 255)
        } else {
            (246, 246, 247, 255)
        };
        self.window.set_background_color(Some(color));
        let background = background_for(dark);
        let _ = self.chrome.set_background_color(background);
        for tab in &self.tabs {
            if let Some(view) = tab.webview.as_ref() {
                let _ = view.set_background_color(background);
            }
        }
    }

    fn on_ipc(&mut self, body: &str) {
        let Ok(request) = serde_json::from_str::<ipc::Request>(body) else {
            return;
        };
        let id = request.id;

        let payload = match self.dispatch(&request) {
            Ok(data) => json!({ "ok": true, "data": data }),
            Err(error) => json!({ "ok": false, "error": error }),
        };

        self.push_state();

        let script = format!(
            "window.__deeps && window.__deeps.resolve({id}, {payload})"
        );
        let _ = self.chrome.evaluate_script(&script);
    }

    fn dispatch(&mut self, request: &ipc::Request) -> Result<Value, String> {
        let args = &request.args;

        match request.cmd.as_str() {
            "ready" => {
                self.last_state.clear();
                self.push_state();
                Ok(Value::Null)
            }
            "new_tab" => {
                let url = args
                    .get("url")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|raw| !raw.is_empty())
                    .map(|raw| urlbar::resolve(raw, &self.start_url, &self.engine()).url)
                    .unwrap_or_else(|| self.start_url.clone());
                self.new_tab(url, true)?;
                Ok(Value::Null)
            }
            "close_tab" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_u64)
                    .or_else(|| self.active_id());
                if let Some(id) = id {
                    self.close_tab(id);
                }
                Ok(Value::Null)
            }
            "select_tab" => {
                if let Some(id) = args.get("id").and_then(Value::as_u64) {
                    if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
                        self.activate(index);
                        if let Some(view) = self
                            .tabs
                            .get(index)
                            .and_then(|tab| tab.webview.as_ref())
                        {
                            let _ = view.focus();
                        }
                    }
                }
                Ok(Value::Null)
            }
            "move_tab" => {
                let id = args.get("id").and_then(Value::as_u64).unwrap_or(0);
                let before = args.get("before").and_then(Value::as_u64).unwrap_or(0);
                let after = args.get("after").and_then(Value::as_bool).unwrap_or(false);
                self.move_tab(id, before, after);
                Ok(Value::Null)
            }
            "navigate" => {
                let raw = args
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing url".to_string())?;
                let resolved = urlbar::resolve(raw, &self.start_url, &self.engine());
                self.navigate(resolved.url);
                Ok(json!({ "notice": resolved.notice }))
            }
            "reload" => {
                self.reload();
                Ok(Value::Null)
            }
            "back" => {
                self.go_back();
                Ok(Value::Null)
            }
            "forward" => {
                self.go_forward();
                Ok(Value::Null)
            }
            "prev_tab" | "next_tab" => {
                self.cycle(request.cmd == "next_tab");
                Ok(Value::Null)
            }
            "devtools" => {
                if let Some(view) = self.tabs.get(self.active).and_then(|tab| tab.webview.as_ref()) {
                    let _ = view.open_devtools();
                }
                Ok(Value::Null)
            }
            "overlay" => {
                let open = args
                    .get("open")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.set_overlay(open);
                Ok(Value::Null)
            }
            "translate_page" => {
                let url = self
                    .tabs
                    .get(self.active)
                    .map(|tab| tab.url.clone())
                    .unwrap_or_default();
                if self.is_internal(&url) {
                    self.open_page("translate");
                } else if let Some(view) = self
                    .tabs
                    .get(self.active)
                    .and_then(|tab| tab.webview.as_ref())
                {
                    let _ = view.evaluate_script(
                        "window.__deepsTranslatePopup && window.__deepsTranslatePopup()",
                    );
                }
                Ok(Value::Null)
            }
            "sidebar" => {
                let collapsed = args
                    .get("collapsed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                if self.current_settings().tab_layout == "vertical"
                    && self.sidebar_collapsed != collapsed
                {
                    self.set_sidebar_collapsed(collapsed);
                    self.dirty = true;
                    self.push_state();
                }
                Ok(Value::Null)
            }
            "import" => {
                let report = match self.data.lock() {
                    Ok(mut data) => importer::import(&mut data),
                    Err(_) => return Err("Stato non disponibile".to_string()),
                };
                self.dirty = true;
                self.push_state();
                Ok(serde_json::to_value(report).unwrap_or(Value::Null))
            }
            "default_browser" => {
                let path = defaults::register_and_open()?;
                Ok(json!({ "registered": true, "path": path }))
            }
            "open" => {
                let page = args.get("page").and_then(Value::as_str).unwrap_or("");
                self.open_page(page);
                Ok(Value::Null)
            }
            "downloads_panel" => {
                let open = args
                    .get("open")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.set_downloads_panel(open);
                Ok(Value::Null)
            }
            "download_accept" => {
                let id = args.get("id").and_then(Value::as_u64).unwrap_or(0);
                let folder = args
                    .get("folder")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let always = args
                    .get("always")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.accept_download(id, &folder, always)?;
                Ok(Value::Null)
            }
            "download_cancel" => {
                let id = args.get("id").and_then(Value::as_u64).unwrap_or(0);
                self.cancel_download(id)?;
                Ok(Value::Null)
            }
            "pick_folder" => {
                let path = crate::downloads::pick_folder()?;
                Ok(json!({ "path": path }))
            }
            "dialog_accept" => {
                let id = args.get("id").and_then(Value::as_u64).unwrap_or(0);
                let text = args
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                self.answer_dialog(id, true, &text)?;
                Ok(Value::Null)
            }
            "dialog_cancel" => {
                let id = args.get("id").and_then(Value::as_u64).unwrap_or(0);
                self.answer_dialog(id, false, "")?;
                Ok(Value::Null)
            }
            "delete_download" => {
                if let Some(id) = args.get("id").and_then(Value::as_u64) {
                    if let Ok(mut data) = self.data.lock() {
                        data.delete_download(id);
                    }
                }
                self.dirty = true;
                Ok(Value::Null)
            }
            "bookmark_toggle" => {
                let url = self
                    .tabs
                    .get(self.active)
                    .map(|tab| tab.url.clone())
                    .unwrap_or_default();
                let title = self
                    .tabs
                    .get(self.active)
                    .map(|tab| tab.title.clone())
                    .unwrap_or_default();
                let mut bookmarked = false;
                if let Ok(mut data) = self.data.lock() {
                    if data.is_bookmarked(&url) {
                        data.remove_bookmark(&url);
                        bookmarked = false;
                    } else {
                        data.add_bookmark(&url, &title)?;
                        bookmarked = true;
                    }
                }
                Ok(json!({ "bookmarked": bookmarked }))
            }
            "set_shortcuts" => {
                let enabled = args
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                if let Ok(mut data) = self.data.lock() {
                    let mut next = data.settings.clone();
                    next.shortcuts = enabled;
                    data.set_settings(next);
                }
                self.apply_page_settings();
                Ok(json!({
                    "notice": if enabled {
                        "Scorciatoie attive"
                    } else {
                        "Scorciatoie disattivate"
                    }
                }))
            }
            "save_settings" => {
                let previous = self.cached_settings.clone();
                let next = {
                    let mut data = self
                        .data
                        .lock()
                        .map_err(|_| "Stato non disponibile".to_string())?;
                    let next = data.settings.patch(args)?;
                    data.set_settings(next.clone());
                    next
                };
                let layout_changed = next.tab_layout != previous.tab_layout
                    || next.onboarded != previous.onboarded
                    || next.wallpaper != previous.wallpaper
                    || next.accent != previous.accent;
                let pages_changed = next.shortcuts != previous.shortcuts
                    || next.save_logins != previous.save_logins
                    || next.tab_layout != previous.tab_layout
                    || next.translate_from != previous.translate_from
                    || next.translate_to != previous.translate_to;
                self.cached_settings = next.clone();
                if next.tab_layout != "vertical" {
                    self.reset_sidebar();
                } else if previous.tab_layout != "vertical" {
                    self.sidebar_width = SIDEBAR_WIDTH;
                    self.sidebar_collapsed = false;
                    self.sidebar_anim = None;
                }
                if layout_changed {
                    self.layout();
                }
                if pages_changed {
                    self.apply_page_settings();
                }
                self.dirty = true;
                Ok(json!({ "notice": "Impostazioni salvate" }))
            }
            "set_adblock" => {
                let enabled = args
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                if let Ok(mut data) = self.data.lock() {
                    let mut next = data.settings.clone();
                    next.adblock = enabled;
                    data.set_settings(next);
                }
                Ok(json!({ "notice": "Riavvia Deeps per applicare Adblock" }))
            }
            "window_min" => {
                self.window.set_minimized(true);
                Ok(Value::Null)
            }
            "window_max" => {
                let maximized = self.window.is_maximized();
                self.window.set_maximized(!maximized);
                self.last_maximized = !maximized;
                self.layout();
                Ok(json!({ "maximized": !maximized }))
            }
            "window_close" => {
                self.persist();
                std::process::exit(0);
            }
            "drag" => {
                let _ = self.window.drag_window();
                Ok(Value::Null)
            }
            "resize" => {
                if let Some(direction) = args.get("dir").and_then(Value::as_str) {
                    self.resize_window(direction);
                }
                Ok(Value::Null)
            }
            "theme" => {
                let dark = args.get("dark").and_then(Value::as_bool).unwrap_or(true);
                self.set_window_background(dark);
                Ok(Value::Null)
            }
            "quit" => {
                self.persist();
                std::process::exit(0);
            }
            other => Err(format!("unknown command: {other}")),
        }
    }

    fn open_download_prompt(&mut self, id: u64, url: String, name: String, folder: String) {
        let _ = (id, url, name, folder);
        self.downloads_panel = false;
        self.layout();
        let _ = self.chrome.focus();
        self.dirty = true;
        self.push_state();
    }

    fn download_prompt_open(&self) -> bool {
        #[cfg(windows)]
        {
            PENDING_DOWNLOADS.with(|queue| !queue.borrow().is_empty())
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    fn download_prompts(&self) -> Vec<Value> {
        #[cfg(windows)]
        {
            PENDING_DOWNLOADS.with(|queue| {
                queue
                    .borrow()
                    .iter()
                    .map(|pending| {
                        json!({
                            "id": pending.id,
                            "url": pending.url,
                            "name": pending.name,
                            "folder": pending.folder,
                        })
                    })
                    .collect()
            })
        }
        #[cfg(not(windows))]
        {
            Vec::new()
        }
    }

    fn set_downloads_panel(&mut self, open: bool) {
        if self.downloads_panel == open {
            return;
        }
        self.downloads_panel = open;
        self.layout();
        self.dirty = true;
        self.push_state();
    }

    fn dialog_open(&self) -> bool {
        #[cfg(windows)]
        {
            PENDING_DIALOGS.with(|queue| !queue.borrow().is_empty())
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    /// The dialogs waiting for an answer, in arrival order.
    fn dialogs(&self) -> Vec<Value> {
        #[cfg(windows)]
        {
            PENDING_DIALOGS.with(|queue| {
                queue
                    .borrow()
                    .iter()
                    .map(|dialog| {
                        json!({
                            "id": dialog.id,
                            "kind": dialog.kind,
                            "message": dialog.message,
                            "defaultText": dialog.default_text,
                            "origin": dialog.origin,
                        })
                    })
                    .collect()
            })
        }
        #[cfg(not(windows))]
        {
            Vec::new()
        }
    }

    /// Accepts or cancels a page dialog. For `prompt`, `text` becomes the
    /// value returned to the page.
    fn answer_dialog(&mut self, id: u64, accept: bool, text: &str) -> Result<(), String> {
        #[cfg(windows)]
        {
            let pending = PENDING_DIALOGS.with(|queue| {
                let mut queue = queue.borrow_mut();
                let index = queue.iter().position(|dialog| dialog.id == id)?;
                queue.remove(index)
            });
            let Some(pending) = pending else {
                return Err("Finestra non trovata".to_string());
            };

            if accept {
                if pending.kind == "prompt" {
                    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                    unsafe {
                        let _ = pending.args.SetResultText(PCWSTR::from_raw(wide.as_ptr()));
                    }
                }
                unsafe {
                    let _ = pending.args.Accept();
                }
            }
            unsafe {
                let _ = pending.deferral.Complete();
            }

            self.dirty = true;
            self.layout();
            self.push_state();
            if let Some(view) = self
                .tabs
                .get(self.active)
                .and_then(|tab| tab.webview.as_ref())
            {
                let _ = view.focus();
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (id, accept, text);
            Err("Finestre non supportate".to_string())
        }
    }

    fn accept_download(&mut self, id: u64, folder: &str, always: bool) -> Result<(), String> {
        #[cfg(windows)]
        {
            let pending = PENDING_DOWNLOADS.with(|queue| {
                let mut queue = queue.borrow_mut();
                let index = queue.iter().position(|pending| pending.id == id)?;
                queue.remove(index)
            });
            let Some(pending) = pending else {
                return Err("Download non trovato".to_string());
            };

            let dir = crate::downloads::destination_dir(folder);
            let destination = crate::downloads::prepare_in(&dir, &pending.name);
            set_result_path(&pending.args, &destination);
            unsafe {
                let _ = pending.args.SetHandled(true);
            }

            let mut download_id = 0;
            if let Ok(mut data) = self.data.lock() {
                download_id = data.start_download(&pending.url, &destination);
            }
            if let Ok(operation) = unsafe { pending.args.DownloadOperation() } {
                attach_operation(&operation, download_id, self.data.clone(), self.proxy.clone());
            }
            unsafe {
                let _ = pending.deferral.Complete();
            }

            if always {
                if let Ok(mut data) = self.data.lock() {
                    let mut next = data.settings.clone();
                    next.download_dir = dir.to_string_lossy().into_owned();
                    next.ask_download_dir = false;
                    data.set_settings(next.clone());
                    self.cached_settings = next;
                }
            }

            self.layout();
            if let Some(view) = self.tabs.get(self.active).and_then(|tab| tab.webview.as_ref()) {
                let _ = view.focus();
            }
            self.dirty = true;
            self.push_state();
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = (id, folder, always);
            Err("Download non supportati".to_string())
        }
    }

    fn cancel_download(&mut self, id: u64) -> Result<(), String> {
        #[cfg(windows)]
        {
            let pending = PENDING_DOWNLOADS.with(|queue| {
                let mut queue = queue.borrow_mut();
                let index = queue.iter().position(|pending| pending.id == id)?;
                queue.remove(index)
            });
            let Some(pending) = pending else {
                return Err("Download non trovato".to_string());
            };
            unsafe {
                let _ = pending.args.SetCancel(true);
                let _ = pending.args.SetHandled(true);
                let _ = pending.deferral.Complete();
            }
            self.layout();
            if let Some(view) = self.tabs.get(self.active).and_then(|tab| tab.webview.as_ref()) {
                let _ = view.focus();
            }
            self.dirty = true;
            self.push_state();
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = id;
            Err("Download non supportati".to_string())
        }
    }

    fn set_overlay(&mut self, open: bool) {
        if self.overlay == open {
            return;
        }
        self.overlay = open;
        if open {
            self.downloads_panel = false;
            if let Some(view) = self.tabs.get(self.active).and_then(|tab| tab.webview.as_ref()) {
                let _ = view.set_visible(false);
            }
            let _ = self.chrome.focus();
        } else if let Some(view) = self.tabs.get(self.active).and_then(|tab| tab.webview.as_ref()) {
            let _ = view.focus();
        }
        self.layout();
        self.dirty = true;
        self.push_state();
    }

    /* ------------------------------------------------------------------ */
    /* layout, state, persistence                                          */
    /* ------------------------------------------------------------------ */

    fn layout(&mut self) {
        let _ = self.chrome.set_bounds(self.chrome_bounds());

        let bounds = self.content_bounds();
        let active = self.active;
        let visible = self.content_visible();
        for (index, tab) in self.tabs.iter().enumerate() {
            if let Some(view) = tab.webview.as_ref() {
                let _ = view.set_bounds(bounds.clone());
                let _ = view.set_visible(index == active && visible);
            }
        }
    }

    /// Moves the native page edge one animation frame towards the target
    /// sidebar width. Called from `Event::MainEventsCleared`.
    fn step_sidebar(&mut self) {
        let Some((from, to, start)) = self
            .sidebar_anim
            .as_ref()
            .map(|anim| (anim.from, anim.to, anim.start))
        else {
            return;
        };
        let progress =
            (start.elapsed().as_secs_f64() / SIDEBAR_ANIM.as_secs_f64()).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        self.sidebar_width = from + (to - from) * eased;
        if progress >= 1.0 {
            self.sidebar_width = to;
            self.sidebar_anim = None;
        }
        self.layout();
    }

    /// Starts an animated collapse/expand. The chrome changes its class
    /// immediately and animates the columns with the same 180ms curve.
    fn set_sidebar_collapsed(&mut self, collapsed: bool) {
        if self.sidebar_collapsed == collapsed && self.sidebar_anim.is_none() {
            return;
        }
        self.sidebar_collapsed = collapsed;
        let to = if collapsed {
            SIDEBAR_COLLAPSED_WIDTH
        } else {
            SIDEBAR_WIDTH
        };
        self.sidebar_anim = Some(SidebarAnim {
            from: self.sidebar_width,
            to,
            start: Instant::now(),
        });
        self.layout();
    }

    fn reset_sidebar(&mut self) {
        self.sidebar_collapsed = false;
        self.sidebar_width = SIDEBAR_WIDTH;
        self.sidebar_anim = None;
    }

    fn current_settings(&self) -> Settings {
        self.data
            .lock()
            .map(|data| data.settings.clone())
            .unwrap_or_else(|_| self.cached_settings.clone())
    }

    fn content_visible(&self) -> bool {
        !self.overlay
            && self.current_settings().onboarded
            && !self.download_prompt_open()
            && !self.dialog_open()
    }

    fn panel_height(&self) -> f64 {
        let settings = self.current_settings();
        if self.downloads_panel
            && !self.overlay
            && settings.onboarded
            && !self.download_prompt_open()
            && !self.dialog_open()
        {
            DOWNLOADS_PANEL_HEIGHT
        } else {
            0.0
        }
    }

    fn chrome_bounds(&self) -> Rect {
        let (width, height) = logical_size(&self.window);
        let settings = self.current_settings();
        let full = self.overlay
            || !settings.onboarded
            || settings.tab_layout == "vertical"
            || self.download_prompt_open()
            || self.dialog_open();
        let chrome_height = if full {
            height
        } else {
            CHROME_HEIGHT + self.panel_height()
        };
        rect(0.0, 0.0, width, chrome_height)
    }

    fn content_bounds(&self) -> Rect {
        let (width, height) = logical_size(&self.window);
        let settings = self.current_settings();
        let top = CHROME_HEIGHT + self.panel_height();
        if settings.tab_layout == "vertical" {
            let sidebar = self.sidebar_width.min(width - 320.0).max(0.0);
            rect(sidebar, top, width - sidebar, height - top)
        } else {
            rect(0.0, top, width, height - top)
        }
    }

    fn active_id(&self) -> Option<u64> {
        self.tabs.get(self.active).map(|tab| tab.id)
    }

    fn is_internal(&self, url: &str) -> bool {
        url.starts_with(&self.start_url)
    }

    fn engine(&self) -> String {
        self.data
            .lock()
            .map(|state| state.settings.engine.clone())
            .unwrap_or_else(|_| "ddg".to_string())
    }

    fn title_of(&self, tab: &Tab) -> String {
        let title = tab.title.trim();
        if !title.is_empty() {
            return title.to_string();
        }
        if self.is_internal(&tab.url) {
            return "Deeps".to_string();
        }
        host_of(&tab.url).unwrap_or_else(|| {
            if self.current_settings().language == "it" {
                "Nuova tab".to_string()
            } else {
                "New tab".to_string()
            }
        })
    }

    fn snapshot(&self) -> Value {
        let (settings, bookmarked, downloads) = match self.data.lock() {
            Ok(data) => (
                data.settings.clone(),
                self.tabs
                    .get(self.active)
                    .map(|tab| data.is_bookmarked(&tab.url))
                    .unwrap_or(false),
                data.downloads(),
            ),
            Err(_) => (Settings::default(), false, Vec::new()),
        };
        let engine_label = engines::by_key(&settings.engine)
            .map(|engine| engine.label)
            .unwrap_or("DuckDuckGo");
        let has_photo = settings.has_photo();
        let photo_url = if settings.photo.is_empty() {
            String::new()
        } else {
            format!(
                "{}wallpaper?t={}",
                self.start_url,
                photo_version(&settings.photo)
            )
        };
        let download_prompts = self.download_prompts();

        let tabs: Vec<Value> = self
            .tabs
            .iter()
            .map(|tab| {
                json!({
                    "id": tab.id,
                    "title": self.title_of(tab),
                    "url": tab.url,
                    "icon": tab.icon,
                    "loading": tab.loading,
                })
            })
            .collect();

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

        json!({
            "tabs": tabs,
            "active": self.active_id().unwrap_or(0),
            "bookmarked": bookmarked,
            "maximized": self.window.is_maximized(),
            "overlay": self.overlay,
            "origin": self.start_url.trim_end_matches('/'),
            "sidebarCollapsed": settings.tab_layout == "vertical" && self.sidebar_collapsed,
            "downloads": downloads,
            "downloadsOpen": self.downloads_panel,
            "downloadPrompt": download_prompts,
            "dialogs": self.dialogs(),
            "photoUrl": photo_url,
            "engines": engines,
            "sites": sites,
            "settings": {
                "engine": settings.engine,
                "engineLabel": engine_label,
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
                "hasPhoto": has_photo,
            },
        })
    }

    fn push_state(&mut self) {
        let state = self.snapshot().to_string();
        if state == self.last_state {
            return;
        }
        self.last_state = state.clone();
        let _ = self
            .chrome
            .evaluate_script(&format!("window.__deeps && window.__deeps.setState({state})"));
    }

    fn sync(&mut self) {
        let current = self
            .tabs
            .get(self.active)
            .and_then(|tab| tab.webview.as_ref())
            .and_then(|view| view.url().ok());

        if let Some(url) = current {
            if !url.is_empty() && url != "about:blank" {
                if let Some(tab) = self.tabs.get_mut(self.active) {
                    if tab.url != url {
                        tab.url = url;
                        self.dirty = true;
                    }
                }
            }
        }

        let settings = self
            .data
            .lock()
            .map(|data| data.settings.clone())
            .unwrap_or_default();
        if settings != self.cached_settings {
            let previous = self.cached_settings.clone();
            let layout_changed = settings.tab_layout != previous.tab_layout
                || settings.onboarded != previous.onboarded
                || settings.wallpaper != previous.wallpaper
                || settings.accent != previous.accent;
            let pages_changed = settings.shortcuts != previous.shortcuts
                || settings.save_logins != previous.save_logins
                || settings.tab_layout != previous.tab_layout
                || settings.translate_from != previous.translate_from
                || settings.translate_to != previous.translate_to;
            self.cached_settings = settings;
            if self.cached_settings.tab_layout != "vertical" {
                self.reset_sidebar();
            } else if previous.tab_layout != "vertical" {
                self.sidebar_width = SIDEBAR_WIDTH;
                self.sidebar_collapsed = false;
                self.sidebar_anim = None;
            }
            if pages_changed {
                self.apply_page_settings();
            }
            if layout_changed {
                self.layout();
            }
            self.dirty = true;
        }

        let maximized = self.window.is_maximized();
        if maximized != self.last_maximized {
            self.last_maximized = maximized;
            self.dirty = true;
        }

        if self.dirty {
            self.dirty = false;
            self.push_state();
            self.persist();
        }
    }

    fn persist(&self) {
        let mut active = 0;
        let mut seen = 0;
        for (index, tab) in self.tabs.iter().enumerate() {
            if self.is_internal(&tab.url) {
                continue;
            }
            if index == self.active {
                active = seen;
                break;
            }
            seen += 1;
        }

        let urls = self
            .tabs
            .iter()
            .map(|tab| tab.url.clone())
            .filter(|url| !self.is_internal(url) && url != "about:blank")
            .collect();

        let (width, height) = logical_size(&self.window);
        let scale = self.window.scale_factor();
        let position = self
            .window
            .outer_position()
            .ok()
            .map(|pos| (pos.x as f64 / scale, pos.y as f64 / scale));

        store::save(&store::Session {
            urls,
            active,
            window: (width, height),
            position,
        });
    }
}

/// Small stable hash used to bust the wallpaper image cache when the photo
/// changes without embedding the data URL in every state snapshot.
fn photo_version(photo: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in photo.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

fn chrome_url() -> String {
    std::env::var("DEEPS_DEV_URL")
        .or_else(|_| std::env::var("DEEPY_DEV_URL"))
        .unwrap_or_else(|_| CHROME_URL.to_string())
}

fn logical_size(window: &Window) -> (f64, f64) {
    let size = window.inner_size();
    let scale = window.scale_factor();
    (
        (size.width as f64 / scale).max(1.0),
        (size.height as f64 / scale).max(1.0),
    )
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        position: RectPosition::new(x, y).into(),
        size: RectSize::new(width.max(1.0), height.max(1.0)).into(),
    }
}

fn background_for(dark: bool) -> (u8, u8, u8, u8) {
    if dark {
        (21, 21, 23, 255)
    } else {
        (255, 255, 255, 255)
    }
}

fn host_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed
        .host_str()
        .map(|host| host.trim_start_matches("www.").to_string())
}
